//! Security System
//!
//! Comprehensive security features including rate limiting, input validation, authentication, and threat detection

pub mod manager;
pub mod rate_limiter;
pub mod validator;
pub mod authenticator;
pub mod middleware;

// Re-export main types
pub use manager::{SecurityManager, SecurityConfig, SecurityRequest, SessionInfo, SecurityError};
pub use rate_limiter::{RateLimiter, TokenBucket};
pub use validator::InputValidator;
pub use authenticator::{Authenticator, UserInfo, ThreatDetector, CsrfProtector};
pub use middleware::SecurityMiddleware;
