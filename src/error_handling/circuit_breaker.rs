//! Circuit Breaker Pattern
//!
//! Circuit breaker implementation for fault tolerance

use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};

/// Circuit breaker state
#[derive(Debug, Clone, PartialEq)]
pub enum CircuitBreakerState {
    Closed,
    Open,
    HalfOpen,
}

/// Circuit breaker implementation
pub struct CircuitBreaker {
    pub state: Arc<Mutex<CircuitBreakerState>>,
    failure_count: Arc<Mutex<u32>>,
    failure_threshold: u32,
    timeout: Duration,
    pub last_failure_time: Arc<Mutex<Option<Instant>>>,
}

impl CircuitBreaker {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(CircuitBreakerState::Closed)),
            failure_count: Arc::new(Mutex::new(0)),
            failure_threshold: 5,
            timeout: Duration::from_secs(60),
            last_failure_time: Arc::new(Mutex::new(None)),
        }
    }

    pub fn call<F, R>(&self, operation: F) -> Result<R, String>
    where
        F: FnOnce() -> Result<R, String>,
    {
        let state = self.state.lock().unwrap().clone();

        match state {
            CircuitBreakerState::Open => {
                if self.should_attempt_reset() {
                    *self.state.lock().unwrap() = CircuitBreakerState::HalfOpen;
                } else {
                    return Err("Circuit breaker is open".to_string());
                }
            }
            CircuitBreakerState::HalfOpen => {
                // Allow one request to test
            }
            CircuitBreakerState::Closed => {
                // Normal operation
            }
        }

        match operation() {
            Ok(result) => {
                self.on_success();
                Ok(result)
            }
            Err(error) => {
                self.on_failure();
                Err(error)
            }
        }
    }

    /// Record a successful operation
    pub fn record_success(&self) {
        self.on_success();
    }

    /// Async version of call for compatibility with tests
    pub async fn call_async<F, Fut, R>(&self, operation: F) -> Result<R, String>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<R, String>>,
    {
        let state = self.state.lock().unwrap().clone();

        match state {
            CircuitBreakerState::Open => {
                if self.should_attempt_reset() {
                    *self.state.lock().unwrap() = CircuitBreakerState::HalfOpen;
                } else {
                    return Err("Circuit breaker is open".to_string());
                }
            }
            CircuitBreakerState::HalfOpen => {
                // Allow one request to test
            }
            CircuitBreakerState::Closed => {
                // Normal operation
            }
        }

        match operation().await {
            Ok(result) => {
                self.on_success();
                Ok(result)
            }
            Err(error) => {
                self.on_failure();
                Err(error)
            }
        }
    }

    /// Get current state
    pub fn state(&self) -> CircuitBreakerState {
        self.state.lock().unwrap().clone()
    }

    /// Check if request is allowed
    pub fn allow_request(&self) -> bool {
        let state = self.state.lock().unwrap().clone();
        match state {
            CircuitBreakerState::Closed => true,
            CircuitBreakerState::Open => self.should_attempt_reset(),
            CircuitBreakerState::HalfOpen => true,
        }
    }

    /// Record a failure
    pub fn record_failure(&self) {
        self.on_failure();
    }

    /// Get state as string for compatibility
    pub fn get_state(&self) -> &'static str {
        let state = self.state.lock().unwrap().clone();
        match state {
            CircuitBreakerState::Closed => "closed",
            CircuitBreakerState::Open => "open",
            CircuitBreakerState::HalfOpen => "half-open",
        }
    }

    fn should_attempt_reset(&self) -> bool {
        if let Some(last_failure) = *self.last_failure_time.lock().unwrap() {
            last_failure.elapsed() >= self.timeout
        } else {
            true
        }
    }

    fn on_success(&self) {
        *self.state.lock().unwrap() = CircuitBreakerState::Closed;
        *self.failure_count.lock().unwrap() = 0;
    }

    fn on_failure(&self) {
        let mut failure_count = self.failure_count.lock().unwrap();
        *failure_count += 1;
        *self.last_failure_time.lock().unwrap() = Some(Instant::now());

        if *failure_count >= self.failure_threshold {
            *self.state.lock().unwrap() = CircuitBreakerState::Open;
        }
    }
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self::new()
    }
}
