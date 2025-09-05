//! Example tests demonstrating TDD (Test-Driven Development) patterns for leptos_ws

use leptos_ws::*;
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Example: TDD approach for a new feature - Signal Validation
/// 
/// This demonstrates the TDD cycle:
/// 1. Write a failing test (Red)
/// 2. Write minimal code to make it pass (Green)
/// 3. Refactor while keeping tests green (Refactor)

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct UserProfile {
    id: u32,
    username: String,
    email: String,
    is_active: bool,
}

impl UserProfile {
    fn new(id: u32, username: &str, email: &str) -> Self {
        Self {
            id,
            username: username.to_string(),
            email: email.to_string(),
            is_active: true,
        }
    }
}

// TDD Example 1: Signal Name Validation
// Step 1: Write failing test first (Red phase)

#[test]
fn test_signal_name_validation_invalid_characters() {
    // Arrange - Test invalid signal names
    let invalid_names = vec![
        "",           // Empty name
        " ",          // Whitespace only
        "signal with spaces",  // Contains spaces
        "signal@invalid",      // Contains special characters
        "signal\nwith\nnewlines", // Contains newlines
    ];

    for name in invalid_names {
        // Act & Assert - This test will fail initially because validation doesn't exist yet
        // In TDD, we write this test first, see it fail, then implement the feature
        assert!(
            !is_valid_signal_name(name),
            "Signal name '{}' should be invalid",
            name
        );
    }
}

#[test]
fn test_signal_name_validation_valid_names() {
    // Arrange - Test valid signal names
    let valid_names = vec![
        "user_profile",
        "counter",
        "settings",
        "signal_123",
        "mySignal",
        "SIGNAL_NAME",
    ];

    for name in valid_names {
        // Act & Assert
        assert!(
            is_valid_signal_name(name),
            "Signal name '{}' should be valid",
            name
        );
    }
}

// Step 2: Implement minimal code to make tests pass (Green phase)
fn is_valid_signal_name(name: &str) -> bool {
    if name.is_empty() || name.trim().is_empty() {
        return false;
    }
    
    name.chars().all(|c| c.is_alphanumeric() || c == '_')
}

// TDD Example 2: Signal Update Batching
// This demonstrates testing complex behavior with multiple scenarios

#[test]
fn test_signal_update_batching_empty_batch() {
    // Arrange
    let mut batch = SignalUpdateBatch::new();
    
    // Act
    let updates = batch.flush();
    
    // Assert
    assert!(updates.is_empty());
}

#[test]
fn test_signal_update_batching_single_update() {
    // Arrange
    let mut batch = SignalUpdateBatch::new();
    let old_profile = UserProfile::new(1, "user1", "user1@example.com");
    let new_profile = UserProfile::new(1, "user1_updated", "user1@example.com");
    
    // Act
    batch.add_update("user_profile", &old_profile, &new_profile).unwrap();
    let updates = batch.flush();
    
    // Assert
    assert_eq!(updates.len(), 1);
    assert_eq!(updates[0].name, "user_profile");
}

#[test]
fn test_signal_update_batching_multiple_updates() {
    // Arrange
    let mut batch = SignalUpdateBatch::new();
    let old_profile = UserProfile::new(1, "user1", "user1@example.com");
    let new_profile = UserProfile::new(1, "user1_updated", "user1@example.com");
    
    // Act
    batch.add_update("user_profile", &old_profile, &new_profile).unwrap();
    batch.add_update("counter", &json!(10), &json!(20)).unwrap();
    let updates = batch.flush();
    
    // Assert
    assert_eq!(updates.len(), 2);
    
    // Check that both updates are present (order may vary due to HashMap)
    let names: std::collections::HashSet<String> = updates.iter().map(|u| u.name.to_string()).collect();
    assert!(names.contains("user_profile"));
    assert!(names.contains("counter"));
}

#[test]
fn test_signal_update_batching_duplicate_signals() {
    // Arrange
    let mut batch = SignalUpdateBatch::new();
    let old_profile = UserProfile::new(1, "user1", "user1@example.com");
    let new_profile1 = UserProfile::new(1, "user1_v1", "user1@example.com");
    let new_profile2 = UserProfile::new(1, "user1_v2", "user1@example.com");
    
    // Act
    batch.add_update("user_profile", &old_profile, &new_profile1).unwrap();
    batch.add_update("user_profile", &new_profile1, &new_profile2).unwrap();
    let updates = batch.flush();
    
    // Assert - Should only have one update for the final state
    assert_eq!(updates.len(), 1);
    assert_eq!(updates[0].name, "user_profile");
}

// Step 2: Implement the SignalUpdateBatch (Green phase)
struct SignalUpdateBatch {
    updates: std::collections::HashMap<String, messages::ServerSignalUpdate>,
}

impl SignalUpdateBatch {
    fn new() -> Self {
        Self {
            updates: std::collections::HashMap::new(),
        }
    }
    
    fn add_update<T>(&mut self, name: &str, old: &T, new: &T) -> Result<(), error::Error>
    where
        T: Serialize,
    {
        let update = messages::ServerSignalUpdate::new(name.to_string(), old, new)?;
        self.updates.insert(name.to_string(), update);
        Ok(())
    }
    
    fn flush(&mut self) -> Vec<messages::ServerSignalUpdate> {
        self.updates.drain().map(|(_, update)| update).collect()
    }
}

// TDD Example 3: Error Recovery and Resilience
// This demonstrates testing error conditions and recovery scenarios

#[test]
fn test_signal_recovery_after_serialization_error() {
    // Arrange - Create a type that will fail serialization
    let invalid_data = InvalidSerializableData {
        data: std::sync::Arc::new(std::sync::Mutex::new(vec![1, 2, 3])),
    };
    
    // Act & Assert - Should handle serialization errors gracefully
    let result = messages::ServerSignalUpdate::new("test_signal", &invalid_data, &invalid_data);
    assert!(result.is_err());
    
    // Verify the error is the expected type
    match result.unwrap_err() {
        serde_json::Error { .. } => {
            // Expected error type
        }
    }
}

#[derive(Clone, Debug)]
struct InvalidSerializableData {
    data: std::sync::Arc<std::sync::Mutex<Vec<i32>>>,
}

impl Serialize for InvalidSerializableData {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // This will always fail to demonstrate error handling
        Err(serde::ser::Error::custom("Cannot serialize Mutex"))
    }
}

// TDD Example 4: Performance and Edge Cases
// This demonstrates testing performance characteristics and edge cases

#[test]
fn test_large_signal_update_performance() {
    // Arrange - Create a large data structure
    let large_data = create_large_test_data(1000);
    let mut modified_data = large_data.clone();
    modified_data.data[500] = "modified".to_string();
    
    // Act - Measure the time to create an update
    let start = std::time::Instant::now();
    let update = messages::ServerSignalUpdate::new("large_signal", &large_data, &modified_data).unwrap();
    let duration = start.elapsed();
    
    // Assert - Should complete within reasonable time (less than 100ms)
    assert!(duration.as_millis() < 100, "Update creation took too long: {:?}", duration);
    assert_eq!(update.name, "large_signal");
    assert!(!update.patch.0.is_empty());
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct LargeTestData {
    data: Vec<String>,
    metadata: std::collections::HashMap<String, String>,
}

impl LargeTestData {
    fn new(size: usize) -> Self {
        let mut data = Vec::with_capacity(size);
        let mut metadata = std::collections::HashMap::new();
        
        for i in 0..size {
            data.push(format!("item_{}", i));
            metadata.insert(format!("key_{}", i), format!("value_{}", i));
        }
        
        Self { data, metadata }
    }
}

fn create_large_test_data(size: usize) -> LargeTestData {
    LargeTestData::new(size)
}

// TDD Example 5: Integration with Real Leptos Patterns
// This demonstrates testing patterns that would be used in real applications

#[test]
fn test_signal_lifecycle_management() {
    // Arrange
    let mut lifecycle = SignalLifecycle::new();
    
    // Act & Assert - Test the complete lifecycle
    let signal_id = lifecycle.create_signal("test_signal", &json!({"value": 0})).unwrap();
    assert_eq!(signal_id, "test_signal");
    
    let update = lifecycle.update_signal("test_signal", &json!({"value": 42})).unwrap();
    assert_eq!(update.name, "test_signal");
    
    lifecycle.destroy_signal("test_signal");
    assert!(lifecycle.get_signal("test_signal").is_none());
}

struct SignalLifecycle {
    signals: std::collections::HashMap<String, serde_json::Value>,
}

impl SignalLifecycle {
    fn new() -> Self {
        Self {
            signals: std::collections::HashMap::new(),
        }
    }
    
    fn create_signal(&mut self, name: &str, initial_value: &serde_json::Value) -> Result<String, error::Error> {
        if !is_valid_signal_name(name) {
            return Err(error::Error::AddingSignalFailed);
        }
        
        self.signals.insert(name.to_string(), initial_value.clone());
        Ok(name.to_string())
    }
    
    fn update_signal(&mut self, name: &str, new_value: &serde_json::Value) -> Result<messages::ServerSignalUpdate, error::Error> {
        let old_value = self.signals.get(name)
            .ok_or(error::Error::UpdateSignalFailed)?;
        
        let update = messages::ServerSignalUpdate::new_from_json(name.to_string(), old_value, new_value);
        self.signals.insert(name.to_string(), new_value.clone());
        Ok(update)
    }
    
    fn get_signal(&self, name: &str) -> Option<&serde_json::Value> {
        self.signals.get(name)
    }
    
    fn destroy_signal(&mut self, name: &str) {
        self.signals.remove(name);
    }
}

// TDD Example 6: Mock and Stub Patterns
// This demonstrates how to test components that depend on external systems

#[test]
fn test_websocket_connection_mock() {
    // Arrange
    let mut mock_ws = MockWebSocket::new();
    let message = messages::Messages::ServerSignal(
        messages::ServerSignalMessage::Establish("test_signal".to_string())
    );
    
    // Act
    let result = mock_ws.send(&message);
    
    // Assert
    assert!(result.is_ok());
    assert_eq!(mock_ws.sent_messages().len(), 1);
    assert_eq!(mock_ws.sent_messages()[0], message);
}

struct MockWebSocket {
    messages: Vec<messages::Messages>,
}

impl MockWebSocket {
    fn new() -> Self {
        Self {
            messages: Vec::new(),
        }
    }
    
    fn send(&mut self, message: &messages::Messages) -> Result<(), serde_json::Error> {
        self.messages.push(message.clone());
        Ok(())
    }
    
    fn sent_messages(&self) -> &[messages::Messages] {
        &self.messages
    }
}
