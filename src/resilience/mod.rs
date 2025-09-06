//! Connection resilience and recovery for leptos-ws
//!
//! Provides sophisticated connection management with automatic recovery mechanisms,
//! circuit breakers, and health monitoring.

use std::time::{Duration, Instant};
use std::sync::Arc;
use tokio::sync::{RwLock, Mutex};
use futures::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};

/// Reconnection strategy configuration
#[derive(Debug, Clone)]
pub enum ReconnectionStrategy {
    /// Exponential backoff with jitter
    ExponentialBackoff {
        initial: Duration,
        max: Duration,
        jitter: f64,
    },
    /// Adaptive strategy based on success/failure rates
    Adaptive {
        success_threshold: usize,
        failure_threshold: usize,
    },
    /// Fixed interval
    Fixed(Duration),
    /// No reconnection
    None,
}

impl Default for ReconnectionStrategy {
    fn default() -> Self {
        Self::ExponentialBackoff {
            initial: Duration::from_secs(1),
            max: Duration::from_secs(60),
            jitter: 0.1,
        }
    }
}

/// Circuit breaker for connection health
pub struct CircuitBreaker {
    failure_count: Arc<Mutex<usize>>,
    success_count: Arc<Mutex<usize>>,
    failure_threshold: usize,
    success_threshold: usize,
    state: Arc<RwLock<CircuitState>>,
    last_failure: Arc<Mutex<Option<Instant>>>,
    timeout: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: usize, success_threshold: usize, timeout: Duration) -> Self {
        Self {
            failure_count: Arc::new(Mutex::new(0)),
            success_count: Arc::new(Mutex::new(0)),
            failure_threshold,
            success_threshold,
            state: Arc::new(RwLock::new(CircuitState::Closed)),
            last_failure: Arc::new(Mutex::new(None)),
            timeout,
        }
    }

    pub async fn should_trip(&self) -> bool {
        let state = *self.state.read().await;
        match state {
            CircuitState::Closed => {
                let failures = *self.failure_count.lock().await;
                failures >= self.failure_threshold
            }
            CircuitState::Open => {
                let last_failure = *self.last_failure.lock().await;
                if let Some(last) = last_failure {
                    last.elapsed() >= self.timeout
                } else {
                    true
                }
            }
            CircuitState::HalfOpen => false,
        }
    }

    pub async fn record_success(&self) {
        let mut success_count = self.success_count.lock().await;
        *success_count += 1;

        if *success_count >= self.success_threshold {
            let mut state = self.state.write().await;
            *state = CircuitState::Closed;
            *self.failure_count.lock().await = 0;
        }
    }

    pub async fn record_failure(&self) {
        let mut failure_count = self.failure_count.lock().await;
        *failure_count += 1;

        let mut last_failure = self.last_failure.lock().await;
        *last_failure = Some(Instant::now());

        if *failure_count >= self.failure_threshold {
            let mut state = self.state.write().await;
            *state = CircuitState::Open;
        }
    }
}

/// Health monitor for connection status
pub struct HealthMonitor {
    last_heartbeat: Arc<Mutex<Option<Instant>>>,
    heartbeat_interval: Duration,
    timeout: Duration,
}

impl HealthMonitor {
    pub fn new(heartbeat_interval: Duration, timeout: Duration) -> Self {
        Self {
            last_heartbeat: Arc::new(Mutex::new(None)),
            heartbeat_interval,
            timeout,
        }
    }

    pub async fn record_heartbeat(&self) {
        let mut last = self.last_heartbeat.lock().await;
        *last = Some(Instant::now());
    }

    pub async fn is_healthy(&self) -> bool {
        let last = *self.last_heartbeat.lock().await;
        if let Some(last_heartbeat) = last {
            last_heartbeat.elapsed() < self.timeout
        } else {
            false
        }
    }

    pub async fn check(&self) -> HealthStatus {
        if self.is_healthy().await {
            HealthStatus::Healthy
        } else {
            HealthStatus::Unhealthy
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Unhealthy,
    Unknown,
}

/// Message buffer for offline scenarios
pub struct MessageBuffer<T> {
    buffer: Arc<Mutex<Vec<T>>>,
    max_size: usize,
}

impl<T> MessageBuffer<T> {
    pub fn new(max_size: usize) -> Self {
        Self {
            buffer: Arc::new(Mutex::new(Vec::new())),
            max_size,
        }
    }

    pub async fn push(&self, message: T) -> Result<(), BufferError> {
        let mut buffer = self.buffer.lock().await;
        if buffer.len() >= self.max_size {
            return Err(BufferError::BufferFull);
        }
        buffer.push(message);
        Ok(())
    }

    pub async fn pop(&self) -> Option<T> {
        let mut buffer = self.buffer.lock().await;
        buffer.pop()
    }

    pub async fn len(&self) -> usize {
        let buffer = self.buffer.lock().await;
        buffer.len()
    }

    pub async fn is_empty(&self) -> bool {
        self.len().await == 0
    }

    pub async fn clear(&self) {
        let mut buffer = self.buffer.lock().await;
        buffer.clear();
    }
}

#[derive(Debug, thiserror::Error)]
pub enum BufferError {
    #[error("Buffer is full")]
    BufferFull,
}

/// Resilient connection manager
pub struct ResilientConnection<T> {
    strategy: ReconnectionStrategy,
    circuit_breaker: CircuitBreaker,
    message_buffer: MessageBuffer<T>,
    health_monitor: HealthMonitor,
    connection_state: Arc<RwLock<ConnectionState>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Failed,
}

impl<T> ResilientConnection<T> {
    pub fn new(
        strategy: ReconnectionStrategy,
        circuit_breaker: CircuitBreaker,
        message_buffer: MessageBuffer<T>,
        health_monitor: HealthMonitor,
    ) -> Self {
        Self {
            strategy,
            circuit_breaker,
            message_buffer,
            health_monitor,
            connection_state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
        }
    }

    pub async fn maintain_connection(&mut self) {
        loop {
            tokio::select! {
                _ = self.health_monitor.check() => {
                    if self.circuit_breaker.should_trip().await {
                        self.initiate_reconnection().await;
                    }
                }
                _ = self.flush_buffer() => {
                    // Buffer flushed
                }
            }
        }
    }

    async fn initiate_reconnection(&mut self) {
        let mut state = self.connection_state.write().await;
        *state = ConnectionState::Reconnecting;

        // Implement reconnection logic based on strategy
        match &self.strategy {
            ReconnectionStrategy::ExponentialBackoff { initial, max, jitter } => {
                // Implement exponential backoff
                let delay = std::cmp::min(*initial * 2, *max);
                tokio::time::sleep(delay).await;
            }
            ReconnectionStrategy::Adaptive { .. } => {
                // Implement adaptive strategy
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
            ReconnectionStrategy::Fixed(duration) => {
                tokio::time::sleep(*duration).await;
            }
            ReconnectionStrategy::None => {
                // No reconnection
                return;
            }
        }

        // Attempt reconnection
        if let Err(_) = self.connect().await {
            self.circuit_breaker.record_failure().await;
        } else {
            self.circuit_breaker.record_success().await;
            *state = ConnectionState::Connected;
        }
    }

    async fn connect(&self) -> Result<(), ConnectionError> {
        // Placeholder for actual connection logic
        Ok(())
    }

    async fn flush_buffer(&self) {
        while !self.message_buffer.is_empty().await {
            if let Some(message) = self.message_buffer.pop().await {
                // Send message
                // In real implementation, this would send the message
                drop(message);
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConnectionError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Circuit breaker open")]
    CircuitBreakerOpen,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_circuit_breaker() {
        let breaker = CircuitBreaker::new(3, 2, Duration::from_secs(1));

        // Should not trip initially
        assert!(!breaker.should_trip().await);

        // Record failures
        breaker.record_failure().await;
        breaker.record_failure().await;
        breaker.record_failure().await;

        // Should trip after threshold
        assert!(breaker.should_trip().await);
    }

    #[tokio::test]
    async fn test_health_monitor() {
        let monitor = HealthMonitor::new(
            Duration::from_secs(1),
            Duration::from_secs(5),
        );

        // Initially unhealthy
        assert_eq!(monitor.check().await, HealthStatus::Unhealthy);

        // Record heartbeat
        monitor.record_heartbeat().await;
        assert_eq!(monitor.check().await, HealthStatus::Healthy);
    }

    #[tokio::test]
    async fn test_message_buffer() {
        let buffer = MessageBuffer::new(10);

        assert!(buffer.is_empty().await);

        // Push messages
        for i in 0..5 {
            buffer.push(i).await.unwrap();
        }

        assert_eq!(buffer.len().await, 5);

        // Pop messages
        for i in (0..5).rev() {
            assert_eq!(buffer.pop().await, Some(i));
        }

        assert!(buffer.is_empty().await);
    }

    #[test]
    fn test_reconnection_strategy_default() {
        let strategy = ReconnectionStrategy::default();
        match strategy {
            ReconnectionStrategy::ExponentialBackoff { initial, max, jitter } => {
                assert_eq!(initial, Duration::from_secs(1));
                assert_eq!(max, Duration::from_secs(60));
                assert_eq!(jitter, 0.1);
            }
            _ => panic!("Expected ExponentialBackoff strategy"),
        }
    }
}
