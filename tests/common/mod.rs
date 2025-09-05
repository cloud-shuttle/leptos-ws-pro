//! Common test utilities and helpers for leptos_ws tests

use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Test data structure for testing signals
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TestData {
    pub id: u32,
    pub name: String,
    pub value: i32,
}

impl TestData {
    pub fn new(id: u32, name: &str, value: i32) -> Self {
        Self {
            id,
            name: name.to_string(),
            value,
        }
    }
}

impl Default for TestData {
    fn default() -> Self {
        Self::new(1, "test", 42)
    }
}

/// Test helper for creating mock WebSocket messages
pub mod mock_websocket {
    use super::*;
    use crate::messages::{Messages, ServerSignalMessage, ServerSignalUpdate};
    use json_patch::Patch;
    use serde_json::Value;

    pub fn create_establish_message(name: &str) -> Messages {
        Messages::ServerSignal(ServerSignalMessage::Establish(name.to_string()))
    }

    pub fn create_establish_response(name: &str, value: Value) -> Messages {
        Messages::ServerSignal(ServerSignalMessage::EstablishResponse((
            name.to_string(),
            value,
        )))
    }

    pub fn create_update_message(name: &str, patch: Patch) -> Messages {
        Messages::ServerSignal(ServerSignalMessage::Update(ServerSignalUpdate {
            name: name.into(),
            patch,
        }))
    }
}

/// Test helper for creating test contexts
pub mod test_context {
    use super::*;
    use crate::{ClientSignals, ServerSignals};

    pub fn create_test_client_signals() -> ClientSignals {
        ClientSignals::new()
    }

    pub fn create_test_server_signals() -> ServerSignals {
        ServerSignals::new()
    }
}

/// Test utilities for async operations
pub mod async_utils {
    use tokio::time::{sleep, Duration};

    /// Wait for a condition to be true with timeout
    pub async fn wait_for_condition<F>(mut condition: F, timeout_ms: u64) -> bool
    where
        F: FnMut() -> bool,
    {
        let start = std::time::Instant::now();
        let timeout = Duration::from_millis(timeout_ms);

        while start.elapsed() < timeout {
            if condition() {
                return true;
            }
            sleep(Duration::from_millis(10)).await;
        }
        false
    }

    /// Run a test with a timeout
    pub async fn with_timeout<F, T>(future: F, timeout_ms: u64) -> Result<T, String>
    where
        F: std::future::Future<Output = T>,
    {
        tokio::time::timeout(Duration::from_millis(timeout_ms), future)
            .await
            .map_err(|_| "Test timeout".to_string())
    }
}

/// Test assertions for JSON values
pub mod json_assertions {
    use serde_json::Value;

    pub fn assert_json_eq(actual: &Value, expected: &Value) {
        assert_eq!(actual, expected, "JSON values should be equal");
    }

    pub fn assert_json_contains(actual: &Value, expected_key: &str) {
        match actual {
            Value::Object(map) => {
                assert!(
                    map.contains_key(expected_key),
                    "JSON object should contain key '{}'",
                    expected_key
                );
            }
            _ => panic!("Expected JSON object, got: {:?}", actual),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_creation() {
        let data = TestData::new(1, "test", 42);
        assert_eq!(data.id, 1);
        assert_eq!(data.name, "test");
        assert_eq!(data.value, 42);
    }

    #[test]
    fn test_data_serialization() {
        let data = TestData::default();
        let json = serde_json::to_value(&data).unwrap();
        let deserialized: TestData = serde_json::from_value(json).unwrap();
        assert_eq!(data, deserialized);
    }
}
