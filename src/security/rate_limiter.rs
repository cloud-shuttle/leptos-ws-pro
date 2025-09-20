//! Rate Limiting
//!
//! Token bucket rate limiting implementation

use crate::security::manager::SecurityError;
use std::collections::HashMap;
use std::time::{Duration, Instant};

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
        let bucket = self
            .buckets
            .entry(client_id.to_string())
            .or_insert_with(|| TokenBucket::new(self.requests_per_minute, self.burst_capacity));

        if bucket.consume() {
            Ok(())
        } else {
            Err(SecurityError::RateLimitExceeded {
                client_id: client_id.to_string(),
            })
        }
    }

    /// Check rate limit for a client (alias for check_request)
    pub fn check_rate_limit(&mut self, client_id: String) -> bool {
        self.check_request(&client_id).is_ok()
    }
}

/// Token bucket for rate limiting
pub struct TokenBucket {
    capacity: u32,
    tokens: u32,
    last_refill: Instant,
    refill_rate: u32, // tokens per minute
}

impl TokenBucket {
    pub fn new(refill_rate: u32, capacity: u32) -> Self {
        Self {
            capacity,
            tokens: capacity,
            last_refill: Instant::now(),
            refill_rate,
        }
    }

    pub fn consume(&mut self) -> bool {
        self.refill();

        if self.tokens > 0 {
            self.tokens -= 1;
            true
        } else {
            false
        }
    }

    /// Try to consume a specific number of tokens
    pub fn try_consume(&mut self, tokens: u32) -> bool {
        self.refill();

        if self.tokens >= tokens {
            self.tokens -= tokens;
            true
        } else {
            false
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let time_passed = now.duration_since(self.last_refill);
        let minutes_passed = time_passed.as_secs_f64() / 60.0;

        let tokens_to_add = (minutes_passed * self.refill_rate as f64) as u32;
        self.tokens = (self.tokens + tokens_to_add).min(self.capacity);
        self.last_refill = now;
    }
}
