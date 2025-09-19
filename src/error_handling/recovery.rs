//! Error Recovery Strategies
//!
//! Recovery strategies and error handling mechanisms

use std::time::Duration;
use std::collections::HashMap;

/// Recovery strategies for different error types
#[derive(Debug, Clone, PartialEq)]
pub enum RecoveryStrategy {
    Retry,
    RetryWithBackoff {
        max_attempts: u32,
        base_delay: Duration,
        max_delay: Duration,
    },
    FallbackToJson,
    CircuitBreaker,
    GracefulDegradation,
    FailFast,
}

/// Error recovery handler
pub struct ErrorRecoveryHandler {
    max_retry_attempts: u32,
    base_retry_delay: Duration,
    max_retry_delay: Duration,
    jitter_enabled: bool,
}

impl ErrorRecoveryHandler {
    pub fn new() -> Self {
        Self {
            max_retry_attempts: 3,
            base_retry_delay: Duration::from_millis(100),
            max_retry_delay: Duration::from_secs(5),
            jitter_enabled: true,
        }
    }
}

impl Default for ErrorRecoveryHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// Error reporter for logging and monitoring
pub struct ErrorReporter {
    error_counts: HashMap<String, u64>,
}

impl ErrorReporter {
    pub fn new() -> Self {
        Self {
            error_counts: HashMap::new(),
        }
    }
}

impl Default for ErrorReporter {
    fn default() -> Self {
        Self::new()
    }
}
