//! Error Handling System
//!
//! Comprehensive error handling, recovery strategies, and circuit breaker patterns

pub mod types;
pub mod recovery;
pub mod circuit_breaker;

// Re-export main types
pub use types::{LeptosWsError, ErrorContext, ErrorType, ThreatLevel};
pub use recovery::{ErrorRecoveryHandler, RecoveryStrategy, ErrorReporter};
pub use circuit_breaker::CircuitBreaker;
