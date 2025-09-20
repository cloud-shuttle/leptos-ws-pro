//! SSE Events
//!
//! Event types and structures for Server-Sent Events

use crate::transport::TransportError;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// SSE Event structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SseEvent {
    pub event_type: String,
    pub data: String,
    pub id: Option<String>,
    pub retry: Option<u64>,
}

impl SseEvent {
    /// Create a new SSE event
    pub fn new(event_type: String, data: String) -> Self {
        Self {
            event_type,
            data,
            id: None,
            retry: None,
        }
    }

    /// Create a new SSE event with ID
    pub fn with_id(mut self, id: String) -> Self {
        self.id = Some(id);
        self
    }

    /// Create a new SSE event with retry interval
    pub fn with_retry(mut self, retry: u64) -> Self {
        self.retry = Some(retry);
        self
    }

    /// Parse SSE event from raw data
    pub fn parse(data: &str) -> Result<Self, TransportError> {
        let lines: Vec<&str> = data.lines().collect();
        let mut event_type = "message".to_string();
        let mut event_data = String::new();
        let mut event_id = None;
        let mut retry = None;

        for line in lines {
            if line.starts_with("event: ") {
                event_type = line[7..].to_string();
            } else if line.starts_with("data: ") {
                if !event_data.is_empty() {
                    event_data.push('\n');
                }
                event_data.push_str(&line[6..]);
            } else if line.starts_with("id: ") {
                event_id = Some(line[4..].to_string());
            } else if line.starts_with("retry: ") {
                retry = line[7..].parse().ok();
            }
        }

        Ok(Self {
            event_type,
            data: event_data,
            id: event_id,
            retry,
        })
    }

    /// Convert SSE event to raw format
    pub fn to_raw(&self) -> String {
        let mut raw = String::new();

        if !self.event_type.is_empty() && self.event_type != "message" {
            raw.push_str(&format!("event: {}\n", self.event_type));
        }

        if let Some(id) = &self.id {
            raw.push_str(&format!("id: {}\n", id));
        }

        if let Some(retry) = self.retry {
            raw.push_str(&format!("retry: {}\n", retry));
        }

        // Handle multi-line data
        for line in self.data.lines() {
            raw.push_str(&format!("data: {}\n", line));
        }

        raw.push('\n');
        raw
    }

    /// Check if this is a heartbeat event
    pub fn is_heartbeat(&self) -> bool {
        self.event_type == "heartbeat" || self.event_type == "ping"
    }

    /// Check if this is a connection close event
    pub fn is_connection_close(&self) -> bool {
        self.event_type == "close" || self.event_type == "disconnect"
    }
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
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            sequence,
        }
    }

    /// Convert to SSE event
    pub fn to_sse_event(&self) -> SseEvent {
        SseEvent::new("heartbeat".to_string(), serde_json::to_string(self).unwrap_or_default())
            .with_id(format!("heartbeat_{}", self.sequence))
    }
}

/// Connection status event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionStatusEvent {
    pub status: String,
    pub timestamp: u64,
    pub client_id: Option<String>,
}

impl ConnectionStatusEvent {
    pub fn new(status: String) -> Self {
        Self {
            status,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            client_id: None,
        }
    }

    pub fn with_client_id(mut self, client_id: String) -> Self {
        self.client_id = Some(client_id);
        self
    }

    /// Convert to SSE event
    pub fn to_sse_event(&self) -> SseEvent {
        SseEvent::new("connection_status".to_string(), serde_json::to_string(self).unwrap_or_default())
    }
}

/// Error event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorEvent {
    pub error_type: String,
    pub message: String,
    pub timestamp: u64,
    pub recoverable: bool,
}

impl ErrorEvent {
    pub fn new(error_type: String, message: String, recoverable: bool) -> Self {
        Self {
            error_type,
            message,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            recoverable,
        }
    }

    /// Convert to SSE event
    pub fn to_sse_event(&self) -> SseEvent {
        SseEvent::new("error".to_string(), serde_json::to_string(self).unwrap_or_default())
    }
}

/// Event filter for filtering SSE events
#[derive(Debug, Clone)]
pub struct EventFilter {
    pub event_types: Vec<String>,
    pub exclude_types: Vec<String>,
    pub data_pattern: Option<String>,
}

impl EventFilter {
    pub fn new() -> Self {
        Self {
            event_types: Vec::new(),
            exclude_types: Vec::new(),
            data_pattern: None,
        }
    }

    pub fn with_event_types(mut self, types: Vec<String>) -> Self {
        self.event_types = types;
        self
    }

    pub fn with_exclude_types(mut self, types: Vec<String>) -> Self {
        self.exclude_types = types;
        self
    }

    pub fn with_data_pattern(mut self, pattern: String) -> Self {
        self.data_pattern = Some(pattern);
        self
    }

    /// Check if an event matches this filter
    pub fn matches(&self, event: &SseEvent) -> bool {
        // Check excluded types first
        if self.exclude_types.contains(&event.event_type) {
            return false;
        }

        // Check included types (if any specified)
        if !self.event_types.is_empty() && !self.event_types.contains(&event.event_type) {
            return false;
        }

        // Check data pattern if specified
        if let Some(pattern) = &self.data_pattern {
            if !event.data.contains(pattern) {
                return false;
            }
        }

        true
    }
}

impl Default for EventFilter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sse_event_creation() {
        let event = SseEvent::new("test".to_string(), "Hello World".to_string());
        assert_eq!(event.event_type, "test");
        assert_eq!(event.data, "Hello World");
        assert!(event.id.is_none());
        assert!(event.retry.is_none());
    }

    #[test]
    fn test_sse_event_with_id() {
        let event = SseEvent::new("test".to_string(), "Hello World".to_string())
            .with_id("123".to_string());
        assert_eq!(event.id, Some("123".to_string()));
    }

    #[test]
    fn test_sse_event_parsing() {
        let data = "event: message\ndata: Hello World\nid: 123\n\n";
        let event = SseEvent::parse(data).unwrap();

        assert_eq!(event.event_type, "message");
        assert_eq!(event.data, "Hello World");
        assert_eq!(event.id, Some("123".to_string()));
    }

    #[test]
    fn test_sse_event_parsing_multiline() {
        let data = "event: message\ndata: Line 1\ndata: Line 2\nid: 456\n\n";
        let event = SseEvent::parse(data).unwrap();

        assert_eq!(event.event_type, "message");
        assert_eq!(event.data, "Line 1\nLine 2");
        assert_eq!(event.id, Some("456".to_string()));
    }

    #[test]
    fn test_sse_event_to_raw() {
        let event = SseEvent::new("test".to_string(), "Hello World".to_string())
            .with_id("123".to_string())
            .with_retry(5000);

        let raw = event.to_raw();
        assert!(raw.contains("event: test"));
        assert!(raw.contains("data: Hello World"));
        assert!(raw.contains("id: 123"));
        assert!(raw.contains("retry: 5000"));
    }

    #[test]
    fn test_heartbeat_event() {
        let heartbeat = HeartbeatEvent::new(1);
        assert!(heartbeat.sequence == 1);
        assert!(heartbeat.timestamp > 0);

        let sse_event = heartbeat.to_sse_event();
        assert_eq!(sse_event.event_type, "heartbeat");
        assert!(sse_event.id.unwrap().contains("heartbeat_1"));
    }

    #[test]
    fn test_event_filter() {
        let filter = EventFilter::new()
            .with_event_types(vec!["message".to_string(), "notification".to_string()])
            .with_exclude_types(vec!["heartbeat".to_string()]);

        let message_event = SseEvent::new("message".to_string(), "test".to_string());
        let heartbeat_event = SseEvent::new("heartbeat".to_string(), "ping".to_string());
        let other_event = SseEvent::new("other".to_string(), "test".to_string());

        assert!(filter.matches(&message_event));
        assert!(!filter.matches(&heartbeat_event));
        assert!(!filter.matches(&other_event));
    }
}
