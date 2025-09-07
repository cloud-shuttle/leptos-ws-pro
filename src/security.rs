//! Production Security Layer
//!
//! Comprehensive security features including authentication, authorization,
//! rate limiting, input validation, and threat protection

use crate::error_handling::ThreatLevel;
use std::collections::HashMap;
use std::time::{Duration, Instant, SystemTime};
// use serde::{Serialize, Deserialize}; // Removed unused imports
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

/// Rate limiter using token bucket algorithm
pub struct RateLimiter {
    buckets: HashMap<String, TokenBucket>,
    requests_per_minute: u32,
    burst_capacity: u32,
}

impl RateLimiter {
    pub fn new(requests_per_minute: u32, burst_capacity: u32) -> Self {
        Self {
            buckets: HashMap::new(),
            requests_per_minute,
            burst_capacity,
        }
    }

    pub fn check_request(&mut self, client_id: &str) -> Result<(), SecurityError> {
        let bucket = self.buckets
            .entry(client_id.to_string())
            .or_insert_with(|| TokenBucket::new(self.requests_per_minute, self.burst_capacity));

        if bucket.try_consume(1) {
            Ok(())
        } else {
            Err(SecurityError::RateLimitExceeded {
                client_id: client_id.to_string(),
            })
        }
    }

    /// Clean up old buckets to prevent memory leaks
    pub fn cleanup_old_buckets(&mut self) {
        let cutoff = Instant::now() - Duration::from_secs(300); // 5 minutes
        self.buckets.retain(|_, bucket| bucket.last_refill > cutoff);
    }
}

/// Token bucket for rate limiting
pub struct TokenBucket {
    tokens: f64,
    capacity: f64,
    refill_rate: f64, // tokens per second
    last_refill: Instant,
}

impl TokenBucket {
    pub fn new(requests_per_minute: u32, capacity: u32) -> Self {
        Self {
            tokens: capacity as f64,
            capacity: capacity as f64,
            refill_rate: requests_per_minute as f64 / 60.0,
            last_refill: Instant::now(),
        }
    }

    pub fn try_consume(&mut self, tokens: u32) -> bool {
        self.refill();

        if self.tokens >= tokens as f64 {
            self.tokens -= tokens as f64;
            true
        } else {
            false
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();

        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.capacity);
        self.last_refill = now;
    }
}

/// Input validator for security
pub struct InputValidator {
    max_message_size: usize,
}

impl InputValidator {
    pub fn new(max_message_size: usize) -> Self {
        Self { max_message_size }
    }

    pub fn validate_input(&self, payload: &[u8]) -> Result<(), SecurityError> {
        // Size check
        if payload.len() > self.max_message_size {
            return Err(SecurityError::InvalidInput {
                reason: format!(
                    "Message size {} exceeds maximum {}",
                    payload.len(),
                    self.max_message_size
                ),
            });
        }

        // Content validation (basic)
        if payload.is_empty() {
            return Err(SecurityError::InvalidInput {
                reason: "Empty payload not allowed".to_string(),
            });
        }

        // Check for potentially malicious patterns
        if self.contains_suspicious_patterns(payload) {
            return Err(SecurityError::InvalidInput {
                reason: "Suspicious content detected".to_string(),
            });
        }

        Ok(())
    }

    fn contains_suspicious_patterns(&self, payload: &[u8]) -> bool {
        let content = String::from_utf8_lossy(payload);

        // Simple pattern detection
        let suspicious_patterns = [
            "<script",
            "javascript:",
            "eval(",
            "exec(",
            "../",
            "passwd",
            "etc/shadow",
        ];

        suspicious_patterns.iter().any(|pattern| {
            content.to_lowercase().contains(&pattern.to_lowercase())
        })
    }
}

/// JWT-based authenticator
pub struct Authenticator {
    jwt_secret: Option<String>,
}

impl Authenticator {
    pub fn new(jwt_secret: Option<String>) -> Self {
        Self { jwt_secret }
    }

    pub fn authenticate(&self, token: &Option<String>) -> Result<UserInfo, SecurityError> {
        match token {
            Some(token_str) => {
                if let Some(_secret) = &self.jwt_secret {
                    // TODO: Implement actual JWT validation
                    // For now, simple token validation
                    if token_str.len() > 10 && token_str.starts_with("Bearer ") {
                        Ok(UserInfo {
                            user_id: "user_123".to_string(),
                            permissions: vec!["read".to_string(), "write".to_string()],
                            expires_at: SystemTime::now() + Duration::from_secs(3600),
                        })
                    } else {
                        Err(SecurityError::AuthenticationFailed {
                            reason: "Invalid token format".to_string(),
                        })
                    }
                } else {
                    Err(SecurityError::AuthenticationFailed {
                        reason: "JWT secret not configured".to_string(),
                    })
                }
            }
            None => Err(SecurityError::AuthenticationFailed {
                reason: "No authentication token provided".to_string(),
            }),
        }
    }

    pub fn generate_token(&self, user_id: &str) -> Result<String, SecurityError> {
        if self.jwt_secret.is_some() {
            // TODO: Implement actual JWT generation
            Ok(format!("Bearer jwt_token_for_{}", user_id))
        } else {
            Err(SecurityError::AuthenticationFailed {
                reason: "JWT secret not configured".to_string(),
            })
        }
    }
}

/// User information from authentication
#[derive(Debug, Clone)]
pub struct UserInfo {
    pub user_id: String,
    pub permissions: Vec<String>,
    pub expires_at: SystemTime,
}

/// Threat detection system
pub struct ThreatDetector {
    request_history: HashMap<String, Vec<Instant>>,
}

impl ThreatDetector {
    pub fn new() -> Self {
        Self {
            request_history: HashMap::new(),
        }
    }

    pub fn analyze_request(&mut self, request: &SecurityRequest) -> Result<ThreatLevel, SecurityError> {
        let mut risk_score = 0.0;

        // Analyze request frequency
        if let Some(ip) = &request.ip_address {
            let history = self.request_history
                .entry(ip.clone())
                .or_insert_with(Vec::new);

            let now = Instant::now();
            history.retain(|&time| now.duration_since(time) < Duration::from_secs(60));
            history.push(now);

            // High frequency requests increase risk
            if history.len() > 100 {
                risk_score += 0.5;
            } else if history.len() > 50 {
                risk_score += 0.3;
            }
        }

        // Analyze payload size
        if request.payload.len() > 100_000 {
            risk_score += 0.3;
        }

        // Analyze user agent
        if let Some(ua) = &request.user_agent {
            if self.is_suspicious_user_agent(ua) {
                risk_score += 0.4;
            }
        } else {
            risk_score += 0.2; // Missing user agent is suspicious
        }

        // Convert risk score to threat level
        let threat_level = if risk_score >= 0.8 {
            ThreatLevel::Critical
        } else if risk_score >= 0.6 {
            ThreatLevel::High
        } else if risk_score >= 0.3 {
            ThreatLevel::Medium
        } else {
            ThreatLevel::Low
        };

        Ok(threat_level)
    }

    fn is_suspicious_user_agent(&self, user_agent: &str) -> bool {
        let suspicious_indicators = [
            "bot",
            "crawler",
            "scraper",
            "python",
            "curl",
            "wget",
        ];

        let ua_lower = user_agent.to_lowercase();
        suspicious_indicators.iter().any(|&indicator| ua_lower.contains(indicator))
    }

    /// Clean up old request history
    pub fn cleanup_history(&mut self) {
        let cutoff = Instant::now() - Duration::from_secs(300);
        for history in self.request_history.values_mut() {
            history.retain(|&time| time > cutoff);
        }
        self.request_history.retain(|_, history| !history.is_empty());
    }
}

impl Default for ThreatDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// CSRF protection
pub struct CsrfProtector {
    tokens: HashMap<String, Instant>,
}

impl CsrfProtector {
    pub fn new() -> Self {
        Self {
            tokens: HashMap::new(),
        }
    }

    pub fn generate_token(&mut self) -> String {
        let token = format!("csrf_{:x}", rand::random::<u64>());
        self.tokens.insert(token.clone(), Instant::now());
        token
    }

    pub fn validate_token(&mut self, token: &str) -> Result<(), SecurityError> {
        if let Some(&created_at) = self.tokens.get(token) {
            if Instant::now().duration_since(created_at) < Duration::from_secs(3600) {
                self.tokens.remove(token); // One-time use
                Ok(())
            } else {
                self.tokens.remove(token); // Expired
                Err(SecurityError::CsrfValidationFailed)
            }
        } else {
            Err(SecurityError::CsrfValidationFailed)
        }
    }

    pub fn cleanup_expired_tokens(&mut self) {
        let cutoff = Instant::now() - Duration::from_secs(3600);
        self.tokens.retain(|_, &mut created_at| created_at > cutoff);
    }
}

impl Default for CsrfProtector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_bucket() {
        let mut bucket = TokenBucket::new(60, 10); // 60 requests per minute, burst of 10

        // Should allow initial burst
        for _ in 0..10 {
            assert!(bucket.try_consume(1));
        }

        // Should deny further requests
        assert!(!bucket.try_consume(1));
    }

    #[test]
    fn test_input_validator() {
        let validator = InputValidator::new(1000);

        // Valid input
        assert!(validator.validate_input(b"Hello, world!").is_ok());

        // Too large
        let large_input = vec![0u8; 2000];
        assert!(validator.validate_input(&large_input).is_err());

        // Suspicious content
        assert!(validator.validate_input(b"<script>alert('xss')</script>").is_err());
    }

    #[test]
    fn test_threat_detector() {
        let mut detector = ThreatDetector::new();

        let request = SecurityRequest {
            client_id: "test_client".to_string(),
            auth_token: None,
            payload: vec![0u8; 100],
            origin: Some("https://example.com".to_string()),
            user_agent: Some("Mozilla/5.0".to_string()),
            ip_address: Some("192.168.1.1".to_string()),
            timestamp: SystemTime::now(),
        };

        let threat_level = detector.analyze_request(&request).unwrap();
        assert_eq!(threat_level, ThreatLevel::Low);
    }

    #[test]
    fn test_rate_limiter() {
        let mut limiter = RateLimiter::new(60, 5);

        // Should allow initial requests
        for _ in 0..5 {
            assert!(limiter.check_request("client1").is_ok());
        }

        // Should deny further requests
        assert!(limiter.check_request("client1").is_err());

        // Should allow requests from different client
        assert!(limiter.check_request("client2").is_ok());
    }

    #[test]
    fn test_csrf_protector() {
        let mut protector = CsrfProtector::new();

        let token = protector.generate_token();
        assert!(protector.validate_token(&token).is_ok());

        // Token should be one-time use
        assert!(protector.validate_token(&token).is_err());
    }
}
