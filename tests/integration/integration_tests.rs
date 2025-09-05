//! Integration tests for leptos_ws WebSocket functionality

use leptos_ws::*;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct TestMessage {
    id: u32,
    content: String,
}

impl TestMessage {
    fn new(id: u32, content: &str) -> Self {
        Self {
            id,
            content: content.to_string(),
        }
    }
}

#[tokio::test]
async fn test_server_signal_update_creation() {
    // Arrange
    let old_message = TestMessage::new(1, "old content");
    let new_message = TestMessage::new(1, "new content");

    // Act
    let update = messages::ServerSignalUpdate::new("test_signal", &old_message, &new_message).unwrap();

    // Assert
    assert_eq!(update.name, "test_signal");
    assert!(!update.patch.0.is_empty());
}

#[tokio::test]
async fn test_server_signal_update_from_json() {
    // Arrange
    let old_json = json!({"id": 1, "content": "old"});
    let new_json = json!({"id": 1, "content": "new"});

    // Act
    let update = messages::ServerSignalUpdate::new_from_json("test_signal", &old_json, &new_json);

    // Assert
    assert_eq!(update.name, "test_signal");
    assert!(!update.patch.0.is_empty());
}

#[tokio::test]
async fn test_messages_roundtrip_serialization() {
    // Test Establish message
    let establish_msg = messages::Messages::ServerSignal(
        messages::ServerSignalMessage::Establish("test_signal".to_string())
    );
    
    let serialized = serde_json::to_string(&establish_msg).unwrap();
    let deserialized: messages::Messages = serde_json::from_str(&serialized).unwrap();
    assert_eq!(establish_msg, deserialized);

    // Test EstablishResponse message
    let response_msg = messages::Messages::ServerSignal(
        messages::ServerSignalMessage::EstablishResponse((
            "test_signal".to_string(),
            json!({"value": 42}),
        ))
    );
    
    let serialized = serde_json::to_string(&response_msg).unwrap();
    let deserialized: messages::Messages = serde_json::from_str(&serialized).unwrap();
    assert_eq!(response_msg, deserialized);

    // Test Update message
    let old_json = json!({"value": 10});
    let new_json = json!({"value": 20});
    let update = messages::ServerSignalUpdate::new_from_json("test_signal", &old_json, &new_json);
    let update_msg = messages::Messages::ServerSignal(
        messages::ServerSignalMessage::Update(update)
    );
    
    let serialized = serde_json::to_string(&update_msg).unwrap();
    let deserialized: messages::Messages = serde_json::from_str(&serialized).unwrap();
    assert_eq!(update_msg, deserialized);
}

#[tokio::test]
async fn test_error_handling() {
    // Test error creation and conversion
    let error = error::Error::MissingServerSignals;
    assert_eq!(error.to_string(), "No ServerSignals in State");

    let error = error::Error::AddingSignalFailed;
    assert_eq!(error.to_string(), "Could not add ServerSignal to ServerSignals");

    let error = error::Error::UpdateSignalFailed;
    assert_eq!(error.to_string(), "Could not update Signal");

    // Test serialization error conversion
    let json_error = serde_json::Error::io(std::io::Error::new(std::io::ErrorKind::Other, "Test serialization error"));
    let error: error::Error = json_error.into();
    match error {
        error::Error::SerializationFailed(serde_error) => {
            assert!(serde_error.to_string().contains("Test serialization error"));
        }
        _ => panic!("Expected SerializationFailed variant"),
    }
}

#[tokio::test]
async fn test_json_patch_operations() {
    // Test simple value change
    let old_json = json!({"value": 10, "name": "test"});
    let new_json = json!({"value": 20, "name": "test"});
    
    let update = messages::ServerSignalUpdate::new_from_json("test_signal", &old_json, &new_json);
    assert_eq!(update.name, "test_signal");
    assert!(!update.patch.0.is_empty());

    // Test no changes
    let update = messages::ServerSignalUpdate::new_from_json("test_signal", &old_json, &old_json);
    assert_eq!(update.name, "test_signal");
    assert!(update.patch.0.is_empty());

    // Test adding new field
    let old_json = json!({"value": 10});
    let new_json = json!({"value": 10, "name": "test"});
    
    let update = messages::ServerSignalUpdate::new_from_json("test_signal", &old_json, &new_json);
    assert_eq!(update.name, "test_signal");
    assert!(!update.patch.0.is_empty());

    // Test removing field
    let old_json = json!({"value": 10, "name": "test"});
    let new_json = json!({"value": 10});
    
    let update = messages::ServerSignalUpdate::new_from_json("test_signal", &old_json, &new_json);
    assert_eq!(update.name, "test_signal");
    assert!(!update.patch.0.is_empty());
}

#[tokio::test]
async fn test_complex_data_structures() {
    // Test with nested objects
    let old_json = json!({
        "user": {
            "id": 1,
            "name": "John",
            "settings": {
                "theme": "dark",
                "notifications": true
            }
        },
        "timestamp": "2024-01-01T00:00:00Z"
    });

    let new_json = json!({
        "user": {
            "id": 1,
            "name": "John Doe",
            "settings": {
                "theme": "light",
                "notifications": false
            }
        },
        "timestamp": "2024-01-01T00:00:00Z"
    });

    let update = messages::ServerSignalUpdate::new_from_json("user_signal", &old_json, &new_json);
    assert_eq!(update.name, "user_signal");
    assert!(!update.patch.0.is_empty());

    // Test with arrays
    let old_json = json!({
        "items": [1, 2, 3],
        "count": 3
    });

    let new_json = json!({
        "items": [1, 2, 3, 4],
        "count": 4
    });

    let update = messages::ServerSignalUpdate::new_from_json("items_signal", &old_json, &new_json);
    assert_eq!(update.name, "items_signal");
    assert!(!update.patch.0.is_empty());
}

#[tokio::test]
async fn test_message_ordering() {
    // Test that messages maintain proper ordering
    let messages = vec![
        messages::Messages::ServerSignal(
            messages::ServerSignalMessage::Establish("signal1".to_string())
        ),
        messages::Messages::ServerSignal(
            messages::ServerSignalMessage::Establish("signal2".to_string())
        ),
        messages::Messages::ServerSignal(
            messages::ServerSignalMessage::EstablishResponse((
                "signal1".to_string(),
                json!({"value": 1}),
            ))
        ),
    ];

    for (i, msg) in messages.iter().enumerate() {
        let serialized = serde_json::to_string(msg).unwrap();
        let deserialized: messages::Messages = serde_json::from_str(&serialized).unwrap();
        assert_eq!(msg, &deserialized, "Message {} should roundtrip correctly", i);
    }
}

#[tokio::test]
async fn test_error_recovery() {
    // Test that we can handle malformed JSON gracefully
    let malformed_json = r#"{"invalid": json}"#;
    let result: Result<messages::Messages, _> = serde_json::from_str(malformed_json);
    assert!(result.is_err());

    // Test that we can handle missing fields
    let incomplete_json = r#"{"type": "ServerSignal"}"#;
    let result: Result<messages::Messages, _> = serde_json::from_str(incomplete_json);
    assert!(result.is_err());
}

#[tokio::test]
async fn test_concurrent_operations() {
    // Test that we can handle concurrent message creation
    let handles: Vec<_> = (0..10)
        .map(|i| {
            tokio::spawn(async move {
                let old_json = json!({"value": i});
                let new_json = json!({"value": i + 1});
                messages::ServerSignalUpdate::new_from_json(
                    format!("signal_{}", i),
                    &old_json,
                    &new_json,
                )
            })
        })
        .collect();

    let results = futures::future::join_all(handles).await;
    
    for (i, result) in results.into_iter().enumerate() {
        let update = result.unwrap();
        assert_eq!(update.name, format!("signal_{}", i));
        assert!(!update.patch.0.is_empty());
    }
}
