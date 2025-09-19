//! Security Middleware
//!
//! Middleware for integrating security features with transport layer

use crate::security::manager::{SecurityManager, SecurityRequest, SecurityError};
use crate::transport::{Message, TransportError};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Security middleware for transport layer
pub struct SecurityMiddleware {
    security_manager: Arc<Mutex<SecurityManager>>,
}

impl SecurityMiddleware {
    pub fn new(security_manager: SecurityManager) -> Self {
        Self {
            security_manager: Arc::new(Mutex::new(security_manager)),
        }
    }

    /// Validate incoming message for security compliance
    pub async fn validate_incoming_message(
        &self,
        message: &Message,
        client_id: &str,
        origin: Option<&str>,
    ) -> Result<(), TransportError> {
        let security_request = SecurityRequest {
            client_id: client_id.to_string(),
            auth_token: None, // Will be extracted from message headers in real implementation
            payload: message.data.clone(),
            origin: origin.map(|s| s.to_string()),
            user_agent: None, // Will be extracted from connection headers
            ip_address: None, // Will be extracted from connection
            timestamp: std::time::SystemTime::now(),
        };

        let mut manager = self.security_manager.lock().await;
        manager.validate_request(&security_request)
            .map_err(|e| TransportError::ProtocolError(format!("Security validation failed: {}", e)))
    }

    /// Validate outgoing message for security compliance
    pub async fn validate_outgoing_message(
        &self,
        message: &Message,
        client_id: &str,
    ) -> Result<(), TransportError> {
        // For outgoing messages, we mainly check size and content
        let security_request = SecurityRequest {
            client_id: client_id.to_string(),
            auth_token: None,
            payload: message.data.clone(),
            origin: None,
            user_agent: None,
            ip_address: None,
            timestamp: std::time::SystemTime::now(),
        };

        let mut manager = self.security_manager.lock().await;
        manager.validate_request(&security_request)
            .map_err(|e| TransportError::ProtocolError(format!("Security validation failed: {}", e)))
    }

    /// Check if client is rate limited
    pub async fn check_rate_limit(&self, client_id: &str) -> Result<(), TransportError> {
        let security_request = SecurityRequest {
            client_id: client_id.to_string(),
            auth_token: None,
            payload: vec![],
            origin: None,
            user_agent: None,
            ip_address: None,
            timestamp: std::time::SystemTime::now(),
        };

        let mut manager = self.security_manager.lock().await;
        manager.validate_request(&security_request)
            .map_err(|e| match e {
                SecurityError::RateLimitExceeded { .. } => TransportError::RateLimited,
                _ => TransportError::ProtocolError(format!("Security check failed: {}", e)),
            })
    }

    /// Generate session token for client
    pub async fn generate_session_token(&self) -> String {
        let manager = self.security_manager.lock().await;
        manager.generate_session_token()
    }

    /// Validate session token
    pub async fn validate_session_token(&self, token: &str) -> Result<(), TransportError> {
        let manager = self.security_manager.lock().await;
        manager.validate_session_token(token)
            .map_err(|_| TransportError::AuthFailed("Invalid session token".to_string()))
            .map(|_| ())
    }
}
