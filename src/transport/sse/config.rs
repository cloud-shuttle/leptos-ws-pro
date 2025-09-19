//! SSE Configuration
//!
//! Configuration types for Server-Sent Events

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Reconnection strategy for SSE
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReconnectionStrategy {
    None,
    Immediate,
    ExponentialBackoff { base_delay: Duration, max_delay: Duration, max_attempts: u32 },
    LinearBackoff { delay: Duration, max_attempts: u32 },
}

/// Heartbeat configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatConfig {
    pub enabled: bool,
    pub interval: Duration,
    pub timeout: Duration,
    pub event_type: String,
}

impl Default for HeartbeatConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            interval: Duration::from_secs(30),
            timeout: Duration::from_secs(60),
            event_type: "heartbeat".to_string(),
        }
    }
}
