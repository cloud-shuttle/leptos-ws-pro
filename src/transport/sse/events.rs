//! SSE Events
//!
//! Event types and structures for Server-Sent Events

use serde::{Deserialize, Serialize};

/// SSE Event structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SseEvent {
    pub event_type: String,
    pub data: String,
    pub id: Option<String>,
    pub retry: Option<u64>,
}

/// Heartbeat event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatEvent {
    pub timestamp: u64,
    pub sequence: u64,
}

impl HeartbeatEvent {
    pub fn new(sequence: u64) -> Self {
        Self {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            sequence,
        }
    }
}
