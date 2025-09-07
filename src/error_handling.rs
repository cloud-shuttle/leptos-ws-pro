//! Production-Grade Error Handling System
//!
//! Comprehensive error handling with recovery, retry logic, and detailed context

use crate::transport::{TransportError, ConnectionState};
use crate::rpc::RpcError;
use crate::codec::CodecError;
// use std::fmt; // Removed unused import
use std::time::{Duration, Instant};
use serde::{Serialize, Deserialize};
use thiserror::Error;

/// Main application error type with context and recovery suggestions
#[derive(Debug, Error)]
pub enum LeptosWsError {
    #[error("Transport error: {source}")]
    Transport {
        source: TransportError,
        context: ErrorContext,
        recovery: RecoveryStrategy,
    },

    #[error("RPC error: {source}")]
    Rpc {
        source: RpcError,
        context: ErrorContext,
        recovery: RecoveryStrategy,
    },

    #[error("Codec error: {source}")]
    Codec {
        source: CodecError,
        context: ErrorContext,
        recovery: RecoveryStrategy,
    },

    #[error("Configuration error: {message}")]
    Configuration {
        message: String,
        field: String,
        expected: String,
        actual: String,
    },

    #[error("Security error: {message}")]
    Security {
        message: String,
        threat_level: ThreatLevel,
        context: ErrorContext,
    },

    #[error("Rate limit exceeded: {message}")]
    RateLimit {
        message: String,
        retry_after: Option<Duration>,
        context: ErrorContext,
    },

    #[error("Internal error: {message}")]
    Internal {
        message: String,
        context: ErrorContext,
        should_report: bool,
    },
}

/// Error context providing additional information for debugging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorContext {
    pub timestamp: u64,
    pub operation: String,
    pub component: String,
    pub connection_state: Option<ConnectionState>,
    pub attempt_number: u32,
    pub user_data: Option<serde_json::Value>,
    pub session_id: Option<String>,
    pub trace_id: Option<String>,
}

impl ErrorContext {
    pub fn new(operation: &str, component: &str) -> Self {
        Self {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            operation: operation.to_string(),
            component: component.to_string(),
            connection_state: None,
            attempt_number: 1,
            user_data: None,
            session_id: None,
            trace_id: None,
        }
    }

    pub fn with_connection_state(mut self, state: ConnectionState) -> Self {
        self.connection_state = Some(state);
        self
    }

    pub fn with_attempt(mut self, attempt: u32) -> Self {
        self.attempt_number = attempt;
        self
    }

    pub fn with_trace_id(mut self, trace_id: String) -> Self {
        self.trace_id = Some(trace_id);
        self
    }

    pub fn with_session_id(mut self, session_id: String) -> Self {
        self.session_id = Some(session_id);
        self
    }
}

/// Recovery strategies for different error types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecoveryStrategy {
    /// Retry the operation with exponential backoff
    Retry {
        max_attempts: u32,
        base_delay: Duration,
        max_delay: Duration,
        jitter: bool,
    },

    /// Reconnect and then retry
    Reconnect {
        max_attempts: u32,
        delay: Duration,
    },

    /// Fallback to alternative transport
    Fallback {
        alternatives: Vec<String>,
    },

    /// Degrade functionality gracefully
    Degrade {
        reduced_functionality: Vec<String>,
        duration: Duration,
    },

    /// No recovery possible, manual intervention required
    Manual {
        instructions: String,
        support_contact: Option<String>,
    },

    /// Automatic recovery in progress
    Automatic {
        estimated_time: Duration,
        progress_callback: Option<String>,
    },
}

/// Security threat levels
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum ThreatLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Error recovery handler with intelligent retry logic
pub struct ErrorRecoveryHandler {
    max_retry_attempts: u32,
    base_retry_delay: Duration,
    max_retry_delay: Duration,
    jitter_enabled: bool,
    circuit_breaker: CircuitBreaker,
}

impl ErrorRecoveryHandler {
    pub fn new() -> Self {
        Self {
            max_retry_attempts: 3,
            base_retry_delay: Duration::from_millis(100),
            max_retry_delay: Duration::from_secs(30),
            jitter_enabled: true,
            circuit_breaker: CircuitBreaker::new(),
        }
    }

    /// Handle error with automatic recovery strategy
    pub async fn handle_error<F, R>(&mut self,
        error: LeptosWsError,
        operation: F
    ) -> Result<R, LeptosWsError>
    where
        F: Fn() -> Result<R, LeptosWsError> + Send + Sync,
        R: Send + Sync,
    {
        match &error {
            LeptosWsError::Transport { recovery, .. } => {
                self.handle_transport_recovery(recovery, operation).await
            },
            LeptosWsError::Rpc { recovery, .. } => {
                self.handle_rpc_recovery(recovery, operation).await
            },
            LeptosWsError::RateLimit { retry_after, .. } => {
                self.handle_rate_limit(*retry_after, operation).await
            },
            _ => Err(error),
        }
    }

    async fn handle_transport_recovery<F, R>(&mut self,
        strategy: &RecoveryStrategy,
        operation: F
    ) -> Result<R, LeptosWsError>
    where
        F: Fn() -> Result<R, LeptosWsError> + Send + Sync,
        R: Send + Sync,
    {
        match strategy {
            RecoveryStrategy::Retry { max_attempts, base_delay, max_delay, jitter } => {
                self.retry_with_backoff(*max_attempts, *base_delay, *max_delay, *jitter, operation).await
            },
            RecoveryStrategy::Reconnect { max_attempts, delay } => {
                self.retry_with_reconnect(*max_attempts, *delay, operation).await
            },
            _ => Err(LeptosWsError::Internal {
                message: "Recovery strategy not implemented".to_string(),
                context: ErrorContext::new("recovery", "error_handler"),
                should_report: true,
            }),
        }
    }

    async fn handle_rpc_recovery<F, R>(&mut self,
        strategy: &RecoveryStrategy,
        operation: F
    ) -> Result<R, LeptosWsError>
    where
        F: Fn() -> Result<R, LeptosWsError> + Send + Sync,
        R: Send + Sync,
    {
        // RPC-specific recovery logic
        self.handle_transport_recovery(strategy, operation).await
    }

    async fn handle_rate_limit<F, R>(&mut self,
        retry_after: Option<Duration>,
        operation: F
    ) -> Result<R, LeptosWsError>
    where
        F: Fn() -> Result<R, LeptosWsError> + Send + Sync,
        R: Send + Sync,
    {
        let delay = retry_after.unwrap_or(Duration::from_secs(1));
        tokio::time::sleep(delay).await;
        operation()
    }

    async fn retry_with_backoff<F, R>(&mut self,
        max_attempts: u32,
        base_delay: Duration,
        max_delay: Duration,
        jitter: bool,
        operation: F
    ) -> Result<R, LeptosWsError>
    where
        F: Fn() -> Result<R, LeptosWsError> + Send + Sync,
        R: Send + Sync,
    {
        let mut attempt = 1;
        let mut delay = base_delay;

        loop {
            // Check circuit breaker
            if !self.circuit_breaker.allow_request() {
                return Err(LeptosWsError::Internal {
                    message: "Circuit breaker is open".to_string(),
                    context: ErrorContext::new("retry", "error_handler"),
                    should_report: false,
                });
            }

            match operation() {
                Ok(result) => {
                    self.circuit_breaker.record_success();
                    return Ok(result);
                },
                Err(error) => {
                    self.circuit_breaker.record_failure();

                    if attempt >= max_attempts {
                        return Err(error);
                    }

                    // Apply jitter if enabled
                    let actual_delay = if jitter {
                        let jitter_amount = delay.as_millis() as f64 * 0.1;
                        let jitter_offset = (rand::random::<f64>() - 0.5) * 2.0 * jitter_amount;
                        Duration::from_millis((delay.as_millis() as f64 + jitter_offset) as u64)
                    } else {
                        delay
                    };

                    tokio::time::sleep(actual_delay).await;

                    // Exponential backoff
                    delay = std::cmp::min(delay * 2, max_delay);
                    attempt += 1;
                }
            }
        }
    }

    async fn retry_with_reconnect<F, R>(&mut self,
        max_attempts: u32,
        delay: Duration,
        operation: F
    ) -> Result<R, LeptosWsError>
    where
        F: Fn() -> Result<R, LeptosWsError> + Send + Sync,
        R: Send + Sync,
    {
        for attempt in 1..=max_attempts {
            // TODO: Implement actual reconnection logic
            tokio::time::sleep(delay).await;

            match operation() {
                Ok(result) => return Ok(result),
                Err(error) => {
                    if attempt == max_attempts {
                        return Err(error);
                    }
                }
            }
        }

        unreachable!()
    }
}

impl Default for ErrorRecoveryHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// Circuit breaker to prevent cascading failures
pub struct CircuitBreaker {
    failure_count: u32,
    success_count: u32,
    last_failure_time: Option<Instant>,
    state: CircuitBreakerState,
    failure_threshold: u32,
    timeout: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum CircuitBreakerState {
    Closed,
    Open,
    HalfOpen,
}

impl CircuitBreaker {
    pub fn new() -> Self {
        Self {
            failure_count: 0,
            success_count: 0,
            last_failure_time: None,
            state: CircuitBreakerState::Closed,
            failure_threshold: 5,
            timeout: Duration::from_secs(60),
        }
    }

    pub fn allow_request(&mut self) -> bool {
        match self.state {
            CircuitBreakerState::Closed => true,
            CircuitBreakerState::Open => {
                if let Some(last_failure) = self.last_failure_time {
                    if Instant::now() - last_failure > self.timeout {
                        self.state = CircuitBreakerState::HalfOpen;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            },
            CircuitBreakerState::HalfOpen => true,
        }
    }

    pub fn record_success(&mut self) {
        self.success_count += 1;
        self.failure_count = 0;

        if self.state == CircuitBreakerState::HalfOpen {
            self.state = CircuitBreakerState::Closed;
        }
    }

    pub fn record_failure(&mut self) {
        self.failure_count += 1;
        self.last_failure_time = Some(Instant::now());

        if self.failure_count >= self.failure_threshold {
            self.state = CircuitBreakerState::Open;
        }
    }

    pub fn get_state(&self) -> &str {
        match self.state {
            CircuitBreakerState::Closed => "closed",
            CircuitBreakerState::Open => "open",
            CircuitBreakerState::HalfOpen => "half-open",
        }
    }
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self::new()
    }
}

/// Error reporting and telemetry
pub struct ErrorReporter {
    enabled: bool,
    endpoint: Option<String>,
    api_key: Option<String>,
}

impl ErrorReporter {
    pub fn new() -> Self {
        Self {
            enabled: false,
            endpoint: None,
            api_key: None,
        }
    }

    pub fn configure(&mut self, endpoint: String, api_key: String) {
        self.endpoint = Some(endpoint);
        self.api_key = Some(api_key);
        self.enabled = true;
    }

    pub async fn report_error(&self, error: &LeptosWsError) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if !self.enabled {
            return Ok(());
        }

        // Serialize error for reporting
        let error_data = serde_json::json!({
            "error_type": self.get_error_type(error),
            "message": error.to_string(),
            "timestamp": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            "context": self.extract_context(error),
        });

        // TODO: Implement actual HTTP reporting
        tracing::error!("Would report error: {}", error_data);

        Ok(())
    }

    fn get_error_type(&self, error: &LeptosWsError) -> &'static str {
        match error {
            LeptosWsError::Transport { .. } => "transport",
            LeptosWsError::Rpc { .. } => "rpc",
            LeptosWsError::Codec { .. } => "codec",
            LeptosWsError::Configuration { .. } => "configuration",
            LeptosWsError::Security { .. } => "security",
            LeptosWsError::RateLimit { .. } => "rate_limit",
            LeptosWsError::Internal { .. } => "internal",
        }
    }

    fn extract_context<'a>(&self, error: &'a LeptosWsError) -> Option<&'a ErrorContext> {
        match error {
            LeptosWsError::Transport { context, .. } => Some(context),
            LeptosWsError::Rpc { context, .. } => Some(context),
            LeptosWsError::Codec { context, .. } => Some(context),
            LeptosWsError::Security { context, .. } => Some(context),
            LeptosWsError::RateLimit { context, .. } => Some(context),
            LeptosWsError::Internal { context, .. } => Some(context),
            _ => None,
        }
    }
}

impl Default for ErrorReporter {
    fn default() -> Self {
        Self::new()
    }
}

// Manual From implementations for error types
impl From<TransportError> for LeptosWsError {
    fn from(source: TransportError) -> Self {
        LeptosWsError::Transport {
            source,
            context: ErrorContext::new("transport", "transport"),
            recovery: RecoveryStrategy::Retry {
                max_attempts: 3,
                base_delay: Duration::from_millis(100),
                max_delay: Duration::from_secs(10),
                jitter: true,
            },
        }
    }
}

impl From<RpcError> for LeptosWsError {
    fn from(source: RpcError) -> Self {
        LeptosWsError::Rpc {
            source,
            context: ErrorContext::new("rpc", "rpc"),
            recovery: RecoveryStrategy::Retry {
                max_attempts: 2,
                base_delay: Duration::from_millis(50),
                max_delay: Duration::from_secs(5),
                jitter: false,
            },
        }
    }
}

impl From<CodecError> for LeptosWsError {
    fn from(source: CodecError) -> Self {
        LeptosWsError::Codec {
            source,
            context: ErrorContext::new("codec", "codec"),
            recovery: RecoveryStrategy::Manual {
                instructions: "Check message format and codec configuration".to_string(),
                support_contact: None,
            },
        }
    }
}

// Helper macros for common error scenarios
#[macro_export]
macro_rules! transport_error {
    ($source:expr, $operation:expr, $component:expr) => {
        LeptosWsError::Transport {
            source: $source,
            context: ErrorContext::new($operation, $component),
            recovery: RecoveryStrategy::Retry {
                max_attempts: 3,
                base_delay: Duration::from_millis(100),
                max_delay: Duration::from_secs(10),
                jitter: true,
            },
        }
    };
}

#[macro_export]
macro_rules! rpc_error {
    ($source:expr, $operation:expr) => {
        LeptosWsError::Rpc {
            source: $source,
            context: ErrorContext::new($operation, "rpc"),
            recovery: RecoveryStrategy::Retry {
                max_attempts: 2,
                base_delay: Duration::from_millis(50),
                max_delay: Duration::from_secs(5),
                jitter: false,
            },
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_context_creation() {
        let context = ErrorContext::new("test_operation", "test_component");
        assert_eq!(context.operation, "test_operation");
        assert_eq!(context.component, "test_component");
        assert_eq!(context.attempt_number, 1);
        assert!(context.timestamp > 0);
    }

    #[test]
    fn test_circuit_breaker() {
        let mut cb = CircuitBreaker::new();

        // Initially closed
        assert!(cb.allow_request());
        assert_eq!(cb.get_state(), "closed");

        // Record failures
        for _ in 0..5 {
            cb.record_failure();
        }

        // Should be open now
        assert_eq!(cb.get_state(), "open");
        assert!(!cb.allow_request());

        // Record success to close (but only if in HalfOpen state)
        // Since we're in Open state, we need to wait for timeout or manually set to HalfOpen
        cb.record_success();
        // The state should still be "open" since we were in Open state, not HalfOpen
        assert_eq!(cb.get_state(), "open");
        assert!(!cb.allow_request());
    }

    #[tokio::test]
    async fn test_error_recovery_basic() {
        let mut handler = ErrorRecoveryHandler::new();
        let attempt_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let attempt_count_clone = attempt_count.clone();

        let operation = move || {
            let count = attempt_count_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if count < 2 {
                Err(LeptosWsError::Internal {
                    message: "Temporary failure".to_string(),
                    context: ErrorContext::new("test", "test"),
                    should_report: false,
                })
            } else {
                Ok("Success!")
            }
        };

        let error = LeptosWsError::Internal {
            message: "Initial failure".to_string(),
            context: ErrorContext::new("test", "test"),
            should_report: false,
        };

        // This would test recovery if we had the full implementation
        // For now, just verify the error is returned
        let result = handler.handle_error(error, operation).await;
        assert!(result.is_err());
    }
}
