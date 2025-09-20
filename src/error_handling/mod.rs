//! Error Handling System
//!
//! Comprehensive error handling, recovery strategies, and circuit breaker patterns

pub mod circuit_breaker;
pub mod recovery;
pub mod types;

// Re-export main types
pub use circuit_breaker::CircuitBreaker;
pub use recovery::{ErrorRecoveryHandler, ErrorReporter, RecoveryStrategy};
pub use types::{ErrorContext, ErrorType, LeptosWsError, ThreatLevel};
