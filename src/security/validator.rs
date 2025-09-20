//! Input Validation
//!
//! Input validation and sanitization

use crate::security::manager::SecurityError;

/// Input validator for security
pub struct InputValidator {
    max_size: usize,
}

impl InputValidator {
    pub fn new(max_size: usize) -> Self {
        Self { max_size }
    }

    pub fn validate_input(&self, payload: &[u8]) -> Result<(), SecurityError> {
        // Size check
        if payload.len() > self.max_size {
            return Err(SecurityError::InvalidInput {
                reason: format!(
                    "Payload too large: {} bytes (max: {})",
                    payload.len(),
                    self.max_size
                ),
            });
        }

        // Basic content validation
        if payload.is_empty() {
            return Err(SecurityError::InvalidInput {
                reason: "Empty payload not allowed".to_string(),
            });
        }

        // Check for suspicious patterns
        if self.contains_suspicious_content(payload) {
            return Err(SecurityError::InvalidInput {
                reason: "Suspicious content detected".to_string(),
            });
        }

        Ok(())
    }

    /// Validate string input (convenience method)
    pub fn validate_string(&self, input: String) -> Result<(), SecurityError> {
        self.validate_input(input.as_bytes())
    }

    fn contains_suspicious_content(&self, payload: &[u8]) -> bool {
        // Simple pattern detection - in production use more sophisticated methods
        let suspicious_patterns: &[&[u8]] =
            &[b"<script", b"javascript:", b"eval(", b"exec(", b"system("];

        for pattern in suspicious_patterns {
            if payload
                .windows(pattern.len())
                .any(|window| window == *pattern)
            {
                return true;
            }
        }

        false
    }
}
