//! Security Manager
//!
//! Central security management and request validation

use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use crate::error_handling::ThreatLevel;
use crate::security::rate_limiter::RateLimiter;
use crate::security::validator::InputValidator;
use crate::security::authenticator::{Authenticator, ThreatDetector};
use thiserror::Error;

/// Security configuration
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    pub enable_authentication: bool,
    pub enable_rate_limiting: bool,
    pub enable_input_validation: bool,
    pub enable_csrf_protection: bool,
    pub jwt_secret: Option<String>,
    pub rate_limit_requests_per_minute: u32,
    pub rate_limit_burst_capacity: u32,
    pub max_message_size: usize,
    pub allowed_origins: Vec<String>,
    pub require_tls: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enable_authentication: true,
            enable_rate_limiting: true,
            enable_input_validation: true,
            enable_csrf_protection: true,
            jwt_secret: None,
            rate_limit_requests_per_minute: 60,
            rate_limit_burst_capacity: 10,
            max_message_size: 1024 * 1024, // 1MB
            allowed_origins: vec!["*".to_string()], // Should be configured properly
            require_tls: true,
        }
    }
}

/// Security manager handling all security aspects
pub struct SecurityManager {
    config: SecurityConfig,
    rate_limiter: RateLimiter,
    validator: InputValidator,
    authenticator: Authenticator,
    threat_detector: ThreatDetector,
}

impl SecurityManager {
    pub fn new(config: SecurityConfig) -> Self {
        Self {
            rate_limiter: RateLimiter::new(
                config.rate_limit_requests_per_minute,
                config.rate_limit_burst_capacity,
            ),
            validator: InputValidator::new(config.max_message_size),
            authenticator: Authenticator::new(config.jwt_secret.clone()),
            threat_detector: ThreatDetector::new(),
            config,
        }
    }

    /// Validate incoming request for security compliance
    pub fn validate_request(&mut self, request: &SecurityRequest) -> Result<(), SecurityError> {
        // Rate limiting check
        if self.config.enable_rate_limiting {
            self.rate_limiter.check_request(&request.client_id)?;
        }

        // Authentication check
        if self.config.enable_authentication {
            self.authenticator.authenticate(&request.auth_token)?;
        }

        // Input validation
        if self.config.enable_input_validation {
            self.validator.validate_input(&request.payload)?;
        }

        // Origin validation
        self.validate_origin(&request.origin)?;

        // Threat detection
        let threat_level = self.threat_detector.analyze_request(request)?;
        if threat_level >= ThreatLevel::High {
            return Err(SecurityError::ThreatDetected {
                level: threat_level,
                description: "High-risk request detected".to_string(),
            });
        }

        Ok(())
    }

    fn validate_origin(&self, origin: &Option<String>) -> Result<(), SecurityError> {
        if let Some(origin) = origin {
            if self.config.allowed_origins.contains(&"*".to_string()) {
                return Ok(());
            }

            if self.config.allowed_origins.contains(origin) {
                Ok(())
            } else {
                Err(SecurityError::UnauthorizedOrigin {
                    origin: origin.clone(),
                    allowed: self.config.allowed_origins.clone(),
                })
            }
        } else {
            Err(SecurityError::MissingOrigin)
        }
    }

    /// Generate secure session token
    pub fn generate_session_token(&self) -> String {
        // Simple token generation - in production use proper cryptographic library
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        SystemTime::now().hash(&mut hasher);
        format!("session_{:x}", hasher.finish())
    }

    /// Validate session token
    pub fn validate_session_token(&self, token: &str) -> Result<SessionInfo, SecurityError> {
        if token.starts_with("session_") && token.len() == 24 {
            Ok(SessionInfo {
                token: token.to_string(),
                expires_at: SystemTime::now() + Duration::from_secs(3600), // 1 hour
                permissions: vec!["read".to_string(), "write".to_string()],
            })
        } else {
            Err(SecurityError::InvalidSession)
        }
    }
}

/// Request object for security validation
#[derive(Debug, Clone)]
pub struct SecurityRequest {
    pub client_id: String,
    pub auth_token: Option<String>,
    pub payload: Vec<u8>,
    pub origin: Option<String>,
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
    pub timestamp: SystemTime,
}

/// Session information
#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub token: String,
    pub expires_at: SystemTime,
    pub permissions: Vec<String>,
}

/// Security errors
#[derive(Debug, Error)]
pub enum SecurityError {
    #[error("Rate limit exceeded for client: {client_id}")]
    RateLimitExceeded { client_id: String },

    #[error("Authentication failed: {reason}")]
    AuthenticationFailed { reason: String },

    #[error("Invalid input: {reason}")]
    InvalidInput { reason: String },

    #[error("Unauthorized origin: {origin}, allowed: {allowed:?}")]
    UnauthorizedOrigin { origin: String, allowed: Vec<String> },

    #[error("Missing origin header")]
    MissingOrigin,

    #[error("Invalid session")]
    InvalidSession,

    #[error("Threat detected (level: {level:?}): {description}")]
    ThreatDetected { level: ThreatLevel, description: String },

    #[error("CSRF token validation failed")]
    CsrfValidationFailed,

    #[error("TLS required but not present")]
    TlsRequired,
}
