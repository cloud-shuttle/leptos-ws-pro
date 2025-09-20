//! Security System
//!
//! Comprehensive security features including rate limiting, input validation, authentication, and threat detection

pub mod authenticator;
pub mod manager;
pub mod middleware;
pub mod rate_limiter;
pub mod validator;

// Re-export main types
pub use authenticator::{Authenticator, CsrfProtector, ThreatDetector, UserInfo};
pub use manager::{SecurityConfig, SecurityError, SecurityManager, SecurityRequest, SessionInfo};
pub use middleware::SecurityMiddleware;
pub use rate_limiter::{RateLimiter, TokenBucket};
pub use validator::InputValidator;
