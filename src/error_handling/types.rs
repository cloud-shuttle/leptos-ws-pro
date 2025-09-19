//! Error Types and Context
//!
//! Core error types, context structures, and error classification

use crate::transport::{TransportError, ConnectionState};
use crate::rpc::RpcError;
use crate::codec::CodecError;
use std::time::Duration;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use thiserror::Error;

/// Main application error type with context and recovery suggestions
#[derive(Debug, Error)]
pub enum LeptosWsError {
    #[error("Transport error: {source}")]
    Transport {
        source: TransportError,
        context: ErrorContext,
        recovery: crate::error_handling::recovery::RecoveryStrategy,
    },

    #[error("RPC error: {source}")]
    Rpc {
        source: RpcError,
        context: ErrorContext,
        recovery: crate::error_handling::recovery::RecoveryStrategy,
    },

    #[error("Codec error: {source}")]
    Codec {
        source: CodecError,
        context: ErrorContext,
        recovery: crate::error_handling::recovery::RecoveryStrategy,
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
    pub error_type: Option<ErrorType>,
    pub message: Option<String>,
    pub service: Option<String>,
    pub correlation_id: Option<String>,
    pub metadata: Option<HashMap<String, String>>,
}

/// Error type classification
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ErrorType {
    Network,
    Timeout,
    Authentication,
    Authorization,
    Validation,
    Serialization,
    Deserialization,
    RateLimit,
    CircuitBreaker,
    ServiceUnavailable,
    Internal,
    Unknown,
    // Additional variants for test compatibility
    Transport,
    Rpc,
    Codec,
}

/// Threat level for security errors
#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum ThreatLevel {
    Low,
    Medium,
    High,
    Critical,
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
            error_type: None,
            message: None,
            service: None,
            correlation_id: None,
            metadata: None,
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

// From implementations for error conversion
impl From<TransportError> for LeptosWsError {
    fn from(error: TransportError) -> Self {
        LeptosWsError::Transport {
            source: error,
            context: ErrorContext::new("transport_operation", "transport"),
            recovery: crate::error_handling::recovery::RecoveryStrategy::RetryWithBackoff {
                max_attempts: 3,
                base_delay: Duration::from_millis(100),
                max_delay: Duration::from_secs(5),
            },
        }
    }
}

impl From<RpcError> for LeptosWsError {
    fn from(error: RpcError) -> Self {
        LeptosWsError::Rpc {
            source: error,
            context: ErrorContext::new("rpc_operation", "rpc"),
            recovery: crate::error_handling::recovery::RecoveryStrategy::RetryWithBackoff {
                max_attempts: 3,
                base_delay: Duration::from_millis(100),
                max_delay: Duration::from_secs(5),
            },
        }
    }
}

impl From<CodecError> for LeptosWsError {
    fn from(error: CodecError) -> Self {
        LeptosWsError::Codec {
            source: error,
            context: ErrorContext::new("codec_operation", "codec"),
            recovery: crate::error_handling::recovery::RecoveryStrategy::FallbackToJson,
        }
    }
}
