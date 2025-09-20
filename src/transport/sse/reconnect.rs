//! SSE Reconnection Logic
//!
//! Handles reconnection strategies and backoff algorithms for SSE connections

use crate::transport::{ConnectionState, TransportError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::time::sleep;

use super::config::ReconnectionStrategy;

/// Reconnection manager for SSE connections
pub struct ReconnectionManager {
    strategy: Arc<Mutex<ReconnectionStrategy>>,
    attempt_count: Arc<Mutex<u32>>,
    last_attempt: Arc<Mutex<Option<Instant>>>,
    max_attempts: u32,
    is_reconnecting: Arc<Mutex<bool>>,
}

impl ReconnectionManager {
    /// Create a new reconnection manager
    pub fn new(strategy: ReconnectionStrategy, max_attempts: u32) -> Self {
        Self {
            strategy: Arc::new(Mutex::new(strategy)),
            attempt_count: Arc::new(Mutex::new(0)),
            last_attempt: Arc::new(Mutex::new(None)),
            max_attempts,
            is_reconnecting: Arc::new(Mutex::new(false)),
        }
    }

    /// Update the reconnection strategy
    pub fn set_strategy(&self, strategy: ReconnectionStrategy) {
        *self.strategy.lock().unwrap() = strategy;
    }

    /// Get the current reconnection strategy
    pub fn get_strategy(&self) -> ReconnectionStrategy {
        self.strategy.lock().unwrap().clone()
    }

    /// Check if reconnection should be attempted
    pub fn should_reconnect(&self) -> bool {
        let attempt_count = *self.attempt_count.lock().unwrap();
        let strategy = self.strategy.lock().unwrap();

        match *strategy {
            ReconnectionStrategy::None => false,
            _ => attempt_count < self.max_attempts,
        }
    }

    /// Get the delay for the next reconnection attempt
    pub fn get_next_delay(&self) -> Duration {
        let attempt_count = *self.attempt_count.lock().unwrap();
        let strategy = self.strategy.lock().unwrap();

        match *strategy {
            ReconnectionStrategy::None => Duration::from_secs(0),
            ReconnectionStrategy::Immediate => Duration::from_millis(100),
            ReconnectionStrategy::ExponentialBackoff {
                base_delay,
                max_delay,
                ..
            } => {
                let delay = base_delay * 2_u32.pow(attempt_count.min(10));
                delay.min(max_delay)
            }
            ReconnectionStrategy::LinearBackoff { delay, .. } => delay,
        }
    }

    /// Record a reconnection attempt
    pub fn record_attempt(&self) {
        *self.attempt_count.lock().unwrap() += 1;
        *self.last_attempt.lock().unwrap() = Some(Instant::now());
    }

    /// Reset the attempt counter (called on successful connection)
    pub fn reset_attempts(&self) {
        *self.attempt_count.lock().unwrap() = 0;
        *self.last_attempt.lock().unwrap() = None;
        *self.is_reconnecting.lock().unwrap() = false;
    }

    /// Get the current attempt count
    pub fn get_attempt_count(&self) -> u32 {
        *self.attempt_count.lock().unwrap()
    }

    /// Get the time since the last attempt
    pub fn get_time_since_last_attempt(&self) -> Option<Duration> {
        self.last_attempt
            .lock()
            .unwrap()
            .map(|last| last.elapsed())
    }

    /// Check if currently reconnecting
    pub fn is_reconnecting(&self) -> bool {
        *self.is_reconnecting.lock().unwrap()
    }

    /// Set reconnecting state
    pub fn set_reconnecting(&self, reconnecting: bool) {
        *self.is_reconnecting.lock().unwrap() = reconnecting;
    }

    /// Wait for the next reconnection attempt
    pub async fn wait_for_next_attempt(&self) -> Result<(), TransportError> {
        if !self.should_reconnect() {
            return Err(TransportError::ConnectionFailed(
                "Maximum reconnection attempts reached".to_string(),
            ));
        }

        let delay = self.get_next_delay();
        self.set_reconnecting(true);

        sleep(delay).await;
        self.record_attempt();

        Ok(())
    }

    /// Get connection state based on reconnection status
    pub fn get_connection_state(&self) -> ConnectionState {
        if self.is_reconnecting() {
            ConnectionState::Reconnecting
        } else if self.get_attempt_count() >= self.max_attempts {
            ConnectionState::Failed
        } else {
            ConnectionState::Disconnected
        }
    }
}

/// Exponential backoff calculator
pub struct ExponentialBackoff {
    base_delay: Duration,
    max_delay: Duration,
    multiplier: f64,
    jitter: bool,
}

impl ExponentialBackoff {
    /// Create a new exponential backoff calculator
    pub fn new(base_delay: Duration, max_delay: Duration) -> Self {
        Self {
            base_delay,
            max_delay,
            multiplier: 2.0,
            jitter: true,
        }
    }

    /// Create with custom multiplier
    pub fn with_multiplier(mut self, multiplier: f64) -> Self {
        self.multiplier = multiplier;
        self
    }

    /// Enable or disable jitter
    pub fn with_jitter(mut self, jitter: bool) -> Self {
        self.jitter = jitter;
        self
    }

    /// Calculate delay for the given attempt number
    pub fn calculate_delay(&self, attempt: u32) -> Duration {
        let delay_ms = self.base_delay.as_millis() as f64;
        let exponential_delay = delay_ms * self.multiplier.powi(attempt as i32);
        let capped_delay = exponential_delay.min(self.max_delay.as_millis() as f64);

        let mut final_delay = capped_delay as u64;

        // Add jitter to prevent thundering herd
        if self.jitter {
            let jitter_amount = final_delay / 4; // 25% jitter
            let jitter = (rand::random::<u64>() % jitter_amount) as u64;
            final_delay += jitter;
        }

        Duration::from_millis(final_delay)
    }
}

/// Linear backoff calculator
pub struct LinearBackoff {
    base_delay: Duration,
    increment: Duration,
    max_delay: Duration,
}

impl LinearBackoff {
    /// Create a new linear backoff calculator
    pub fn new(base_delay: Duration, increment: Duration, max_delay: Duration) -> Self {
        Self {
            base_delay,
            increment,
            max_delay,
        }
    }

    /// Calculate delay for the given attempt number
    pub fn calculate_delay(&self, attempt: u32) -> Duration {
        let delay = self.base_delay + (self.increment * attempt);
        delay.min(self.max_delay)
    }
}

/// Connection health monitor
pub struct ConnectionHealthMonitor {
    last_heartbeat: Arc<Mutex<Option<Instant>>>,
    heartbeat_timeout: Duration,
    max_missed_heartbeats: u32,
    missed_heartbeats: Arc<Mutex<u32>>,
}

impl ConnectionHealthMonitor {
    /// Create a new connection health monitor
    pub fn new(heartbeat_timeout: Duration, max_missed_heartbeats: u32) -> Self {
        Self {
            last_heartbeat: Arc::new(Mutex::new(None)),
            heartbeat_timeout,
            max_missed_heartbeats,
            missed_heartbeats: Arc::new(Mutex::new(0)),
        }
    }

    /// Record a heartbeat
    pub fn record_heartbeat(&self) {
        *self.last_heartbeat.lock().unwrap() = Some(Instant::now());
        *self.missed_heartbeats.lock().unwrap() = 0;
    }

    /// Check if the connection is healthy
    pub fn is_healthy(&self) -> bool {
        let last_heartbeat = self.last_heartbeat.lock().unwrap();
        let missed_heartbeats = *self.missed_heartbeats.lock().unwrap();

        if missed_heartbeats >= self.max_missed_heartbeats {
            return false;
        }

        if let Some(last) = *last_heartbeat {
            last.elapsed() <= self.heartbeat_timeout
        } else {
            true // No heartbeat yet, assume healthy
        }
    }

    /// Get the time since the last heartbeat
    pub fn get_time_since_heartbeat(&self) -> Option<Duration> {
        self.last_heartbeat
            .lock()
            .unwrap()
            .map(|last| last.elapsed())
    }

    /// Increment missed heartbeat count
    pub fn increment_missed_heartbeats(&self) {
        *self.missed_heartbeats.lock().unwrap() += 1;
    }

    /// Reset missed heartbeat count
    pub fn reset_missed_heartbeats(&self) {
        *self.missed_heartbeats.lock().unwrap() = 0;
    }

    /// Get the number of missed heartbeats
    pub fn get_missed_heartbeats(&self) -> u32 {
        *self.missed_heartbeats.lock().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reconnection_manager_creation() {
        let manager = ReconnectionManager::new(
            ReconnectionStrategy::ExponentialBackoff {
                base_delay: Duration::from_secs(1),
                max_delay: Duration::from_secs(30),
                max_attempts: 5,
            },
            5,
        );

        assert_eq!(manager.get_attempt_count(), 0);
        assert!(manager.should_reconnect());
    }

    #[test]
    fn test_reconnection_manager_strategy_update() {
        let manager = ReconnectionManager::new(ReconnectionStrategy::None, 5);
        assert!(!manager.should_reconnect());

        manager.set_strategy(ReconnectionStrategy::Immediate);
        assert!(manager.should_reconnect());
    }

    #[test]
    fn test_reconnection_manager_attempts() {
        let manager = ReconnectionManager::new(ReconnectionStrategy::Immediate, 3);

        assert_eq!(manager.get_attempt_count(), 0);
        assert!(manager.should_reconnect());

        manager.record_attempt();
        assert_eq!(manager.get_attempt_count(), 1);
        assert!(manager.should_reconnect());

        manager.record_attempt();
        manager.record_attempt();
        assert_eq!(manager.get_attempt_count(), 3);
        assert!(!manager.should_reconnect());

        manager.reset_attempts();
        assert_eq!(manager.get_attempt_count(), 0);
        assert!(manager.should_reconnect());
    }

    #[test]
    fn test_exponential_backoff() {
        let backoff = ExponentialBackoff::new(
            Duration::from_secs(1),
            Duration::from_secs(30),
        );

        let delay1 = backoff.calculate_delay(0);
        let delay2 = backoff.calculate_delay(1);
        let delay3 = backoff.calculate_delay(2);

        assert!(delay2 > delay1);
        assert!(delay3 > delay2);
        assert!(delay3 <= Duration::from_secs(30));
    }

    #[test]
    fn test_linear_backoff() {
        let backoff = LinearBackoff::new(
            Duration::from_secs(1),
            Duration::from_secs(2),
            Duration::from_secs(10),
        );

        let delay1 = backoff.calculate_delay(0);
        let delay2 = backoff.calculate_delay(1);
        let delay3 = backoff.calculate_delay(2);

        assert_eq!(delay1, Duration::from_secs(1));
        assert_eq!(delay2, Duration::from_secs(3));
        assert_eq!(delay3, Duration::from_secs(5));
    }

    #[test]
    fn test_connection_health_monitor() {
        let monitor = ConnectionHealthMonitor::new(
            Duration::from_secs(30),
            3,
        );

        assert!(monitor.is_healthy());
        assert_eq!(monitor.get_missed_heartbeats(), 0);

        monitor.record_heartbeat();
        assert!(monitor.is_healthy());

        monitor.increment_missed_heartbeats();
        monitor.increment_missed_heartbeats();
        monitor.increment_missed_heartbeats();
        assert!(!monitor.is_healthy());

        monitor.reset_missed_heartbeats();
        assert!(monitor.is_healthy());
    }
}
