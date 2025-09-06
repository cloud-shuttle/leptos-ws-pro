//! Middleware system for leptos-ws
//!
//! Provides Tower-compatible middleware for cross-cutting concerns like
//! authentication, rate limiting, compression, and metrics.

use async_trait::async_trait;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tower::{Layer, Service, ServiceExt};

/// Middleware trait for WebSocket services
#[async_trait]
pub trait WebSocketMiddleware: Send + Sync + 'static {
    type Request;
    type Response;
    type Error;

    async fn call(&self, request: Self::Request) -> Result<Self::Response, Self::Error>;
}

/// Authentication middleware
pub struct AuthenticationLayer {
    validator: Box<dyn JwtValidator + Send + Sync>,
}

pub trait JwtValidator: Send + Sync {
    fn validate(&self, token: &str) -> Result<Claims, AuthError>;
}

#[derive(Debug, Clone)]
pub struct Claims {
    pub user_id: String,
    pub permissions: Vec<String>,
    pub expires_at: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Invalid token: {0}")]
    InvalidToken(String),

    #[error("Expired token")]
    ExpiredToken,

    #[error("Missing token")]
    MissingToken,
}

impl AuthenticationLayer {
    pub fn new(validator: Box<dyn JwtValidator + Send + Sync>) -> Self {
        Self { validator }
    }
}

#[async_trait]
impl WebSocketMiddleware for AuthenticationLayer {
    type Request = AuthenticatedRequest;
    type Response = AuthenticatedResponse;
    type Error = AuthError;

    async fn call(&self, request: Self::Request) -> Result<Self::Response, Self::Error> {
        let token = request.token.ok_or(AuthError::MissingToken)?;
        let claims = self.validator.validate(&token)?;

        Ok(AuthenticatedResponse {
            claims,
            original_request: request,
        })
    }
}

#[derive(Debug, Clone)]
pub struct AuthenticatedRequest {
    pub token: Option<String>,
    pub message: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct AuthenticatedResponse {
    pub claims: Claims,
    pub original_request: AuthenticatedRequest,
}

/// Rate limiting middleware
pub struct RateLimitLayer {
    limiter: Box<dyn RateLimiter + Send + Sync>,
}

pub trait RateLimiter: Send + Sync {
    fn check_and_consume(&self, user_id: &str, count: u32) -> Result<(), RateLimitError>;
}

#[derive(Debug, thiserror::Error)]
pub enum RateLimitError {
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
}

impl RateLimitLayer {
    pub fn new(limiter: Box<dyn RateLimiter + Send + Sync>) -> Self {
        Self { limiter }
    }
}

#[async_trait]
impl WebSocketMiddleware for RateLimitLayer {
    type Request = RateLimitedRequest;
    type Response = RateLimitedResponse;
    type Error = RateLimitError;

    async fn call(&self, request: Self::Request) -> Result<Self::Response, Self::Error> {
        self.limiter.check_and_consume(&request.user_id, 1)?;

        Ok(RateLimitedResponse {
            original_request: request,
        })
    }
}

#[derive(Debug, Clone)]
pub struct RateLimitedRequest {
    pub user_id: String,
    pub message: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct RateLimitedResponse {
    pub original_request: RateLimitedRequest,
}

/// Compression middleware
#[cfg(feature = "compression")]
pub struct CompressionLayer {
    threshold: usize,
}

#[cfg(feature = "compression")]
impl CompressionLayer {
    pub fn new(threshold: usize) -> Self {
        Self { threshold }
    }
}

#[cfg(feature = "compression")]
#[async_trait]
impl WebSocketMiddleware for CompressionLayer {
    type Request = CompressedRequest;
    type Response = CompressedResponse;
    type Error = CompressionError;

    async fn call(&self, request: Self::Request) -> Result<Self::Response, Self::Error> {
        let compressed_data = if request.data.len() > self.threshold {
            zstd::encode_all(&request.data, 3)
                .map_err(|e| CompressionError::CompressionFailed(e.to_string()))?
        } else {
            request.data.clone()
        };

        Ok(CompressedResponse {
            data: compressed_data,
            original_size: request.data.len(),
        })
    }
}

#[cfg(feature = "compression")]
#[derive(Debug, Clone)]
pub struct CompressedRequest {
    pub data: Vec<u8>,
}

#[cfg(feature = "compression")]
#[derive(Debug, Clone)]
pub struct CompressedResponse {
    pub data: Vec<u8>,
    pub original_size: usize,
}

#[cfg(feature = "compression")]
#[derive(Debug, thiserror::Error)]
pub enum CompressionError {
    #[error("Compression failed: {0}")]
    CompressionFailed(String),
}

/// Metrics middleware
#[cfg(feature = "metrics")]
pub struct MetricsLayer {
    metrics: Arc<WebSocketMetrics>,
}

#[cfg(feature = "metrics")]
#[derive(Debug, Clone)]
pub struct WebSocketMetrics {
    pub messages_sent: u64,
    pub messages_received: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub active_connections: u64,
    pub reconnection_attempts: u64,
}

#[cfg(feature = "metrics")]
impl MetricsLayer {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(WebSocketMetrics {
                messages_sent: 0,
                messages_received: 0,
                bytes_sent: 0,
                bytes_received: 0,
                active_connections: 0,
                reconnection_attempts: 0,
            }),
        }
    }
}

#[cfg(feature = "metrics")]
#[async_trait]
impl WebSocketMiddleware for MetricsLayer {
    type Request = MetricsRequest;
    type Response = MetricsResponse;
    type Error = MetricsError;

    async fn call(&self, request: Self::Request) -> Result<Self::Response, Self::Error> {
        // Update metrics based on request type
        match request.request_type {
            MetricsRequestType::MessageSent => {
                self.metrics.messages_sent += 1;
                self.metrics.bytes_sent += request.data.len() as u64;
            }
            MetricsRequestType::MessageReceived => {
                self.metrics.messages_received += 1;
                self.metrics.bytes_received += request.data.len() as u64;
            }
            MetricsRequestType::ConnectionEstablished => {
                self.metrics.active_connections += 1;
            }
            MetricsRequestType::ConnectionClosed => {
                if self.metrics.active_connections > 0 {
                    self.metrics.active_connections -= 1;
                }
            }
            MetricsRequestType::ReconnectionAttempt => {
                self.metrics.reconnection_attempts += 1;
            }
        }

        Ok(MetricsResponse {
            metrics: self.metrics.clone(),
            original_request: request,
        })
    }
}

#[cfg(feature = "metrics")]
#[derive(Debug, Clone)]
pub struct MetricsRequest {
    pub request_type: MetricsRequestType,
    pub data: Vec<u8>,
}

#[cfg(feature = "metrics")]
#[derive(Debug, Clone)]
pub enum MetricsRequestType {
    MessageSent,
    MessageReceived,
    ConnectionEstablished,
    ConnectionClosed,
    ReconnectionAttempt,
}

#[cfg(feature = "metrics")]
#[derive(Debug, Clone)]
pub struct MetricsResponse {
    pub metrics: WebSocketMetrics,
    pub original_request: MetricsRequest,
}

#[cfg(feature = "metrics")]
#[derive(Debug, thiserror::Error)]
pub enum MetricsError {
    #[error("Metrics collection failed: {0}")]
    CollectionFailed(String),
}

/// Middleware stack builder
pub struct MiddlewareStackBuilder<T> {
    middlewares: Vec<Box<dyn WebSocketMiddleware<Request = T, Response = T, Error = MiddlewareError> + Send + Sync>>,
}

impl<T> MiddlewareStackBuilder<T> {
    pub fn new() -> Self {
        Self {
            middlewares: Vec::new(),
        }
    }

    pub fn add<M>(mut self, middleware: M) -> Self
    where
        M: WebSocketMiddleware<Request = T, Response = T, Error = MiddlewareError> + Send + Sync + 'static,
    {
        self.middlewares.push(Box::new(middleware));
        self
    }

    pub fn build(self) -> MiddlewareStack<T> {
        MiddlewareStack {
            middlewares: self.middlewares,
        }
    }
}

/// Middleware stack
pub struct MiddlewareStack<T> {
    middlewares: Vec<Box<dyn WebSocketMiddleware<Request = T, Response = T, Error = MiddlewareError> + Send + Sync>>,
}

impl<T> MiddlewareStack<T> {
    pub async fn process(&self, mut request: T) -> Result<T, MiddlewareError> {
        for middleware in &self.middlewares {
            request = middleware.call(request).await?;
        }
        Ok(request)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MiddlewareError {
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(#[from] AuthError),

    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(#[from] RateLimitError),

    #[cfg(feature = "compression")]
    #[error("Compression failed: {0}")]
    CompressionFailed(#[from] CompressionError),

    #[cfg(feature = "metrics")]
    #[error("Metrics error: {0}")]
    MetricsError(#[from] MetricsError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_authentication_layer() {
        struct MockValidator;

        impl JwtValidator for MockValidator {
            fn validate(&self, token: &str) -> Result<Claims, AuthError> {
                if token == "valid_token" {
                    Ok(Claims {
                        user_id: "user1".to_string(),
                        permissions: vec!["read".to_string(), "write".to_string()],
                        expires_at: 1234567890,
                    })
                } else {
                    Err(AuthError::InvalidToken("Invalid token".to_string()))
                }
            }
        }

        let layer = AuthenticationLayer::new(Box::new(MockValidator));

        let request = AuthenticatedRequest {
            token: Some("valid_token".to_string()),
            message: b"test message".to_vec(),
        };

        let response = layer.call(request).await;
        assert!(response.is_ok());

        let claims = response.unwrap().claims;
        assert_eq!(claims.user_id, "user1");
    }

    #[tokio::test]
    async fn test_rate_limit_layer() {
        struct MockLimiter;

        impl RateLimiter for MockLimiter {
            fn check_and_consume(&self, _user_id: &str, _count: u32) -> Result<(), RateLimitError> {
                Ok(())
            }
        }

        let layer = RateLimitLayer::new(Box::new(MockLimiter));

        let request = RateLimitedRequest {
            user_id: "user1".to_string(),
            message: b"test message".to_vec(),
        };

        let response = layer.call(request).await;
        assert!(response.is_ok());
    }

    #[test]
    fn test_middleware_stack_builder() {
        let stack = MiddlewareStackBuilder::<()>::new()
            .build();

        assert_eq!(stack.middlewares.len(), 0);
    }
}
