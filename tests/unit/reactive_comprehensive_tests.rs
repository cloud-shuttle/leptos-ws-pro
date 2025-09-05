use leptos_ws::reactive::*;
use leptos_ws::transport::{Message, MessageType, ConnectionState};
use leptos_ws::codec::JsonCodec;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct TestMessage {
    id: u32,
    content: String,
    timestamp: u64,
}

// Use the UserPresence from the reactive module

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct ConnectionMetrics {
    bytes_sent: u64,
    bytes_received: u64,
    messages_sent: u64,
    messages_received: u64,
    connection_uptime: u64,
}

#[test]
fn test_websocket_provider_creation() {
    let provider = WebSocketProvider::new("ws://localhost:8080");
    assert_eq!(provider.url(), "ws://localhost:8080");
}

#[test]
fn test_websocket_provider_with_config() {
    let config = WebSocketConfig {
        url: "ws://localhost:8080".to_string(),
        protocols: vec!["chat".to_string(), "notifications".to_string()],
        heartbeat_interval: Some(30),
        reconnect_interval: Some(5),
        max_reconnect_attempts: Some(10),
        codec: Box::new(JsonCodec::new()),
    };
    
    let provider = WebSocketProvider::with_config(config.clone());
    assert_eq!(provider.config().url, config.url);
    assert_eq!(provider.config().protocols, config.protocols);
    assert_eq!(provider.config().heartbeat_interval, config.heartbeat_interval);
}

#[test]
fn test_websocket_context_creation() {
    let provider = WebSocketProvider::new("ws://localhost:8080");
    let context = WebSocketContext::new(provider);
    
    assert_eq!(context.connection_state(), ConnectionState::Disconnected);
    assert!(!context.is_connected());
}

#[test]
fn test_connection_state_transitions() {
    let provider = WebSocketProvider::new("ws://localhost:8080");
    let context = WebSocketContext::new(provider);
    
    // Initial state
    assert_eq!(context.connection_state(), ConnectionState::Disconnected);
    
    // Simulate connection
    context.set_connection_state(ConnectionState::Connecting);
    assert_eq!(context.connection_state(), ConnectionState::Connecting);
    
    // Simulate connected
    context.set_connection_state(ConnectionState::Connected);
    assert_eq!(context.connection_state(), ConnectionState::Connected);
    assert!(context.is_connected());
    
    // Simulate disconnection
    context.set_connection_state(ConnectionState::Disconnected);
    assert_eq!(context.connection_state(), ConnectionState::Disconnected);
    assert!(!context.is_connected());
}

#[test]
fn test_message_subscription() {
    let provider = WebSocketProvider::new("ws://localhost:8080");
    let context = WebSocketContext::new(provider);
    
    let subscription = context.subscribe_to_messages::<TestMessage>();
    assert!(subscription.is_some());
    
    // Get the subscription signal
    let messages_signal = subscription.unwrap();
    
    // Test message handling
    let test_message = TestMessage {
        id: 1,
        content: "Hello, World!".to_string(),
        timestamp: 1234567890,
    };
    
    let message = Message {
        data: serde_json::to_vec(&test_message).unwrap(),
        message_type: MessageType::Text,
    };
    
    // Simulate receiving a message
    context.handle_message(message.clone());
    
    // Verify message was processed
    let received_messages = context.get_received_messages::<TestMessage>();
    assert_eq!(received_messages.len(), 1);
    assert_eq!(received_messages[0], test_message);
}

#[test]
fn test_message_sending() {
    let provider = WebSocketProvider::new("ws://localhost:8080");
    let context = WebSocketContext::new(provider);
    
    let test_message = TestMessage {
        id: 1,
        content: "Hello, World!".to_string(),
        timestamp: 1234567890,
    };
    
    // Send message
    let result = context.send_message(&test_message);
    assert!(result.is_ok());
    
    // Verify message was queued for sending
    let sent_messages = context.get_sent_messages::<TestMessage>();
    assert_eq!(sent_messages.len(), 1);
    assert_eq!(sent_messages[0], test_message);
}

#[test]
fn test_connection_metrics_tracking() {
    let provider = WebSocketProvider::new("ws://localhost:8080");
    let context = WebSocketContext::new(provider);
    
    // Initial metrics should be zero
    let initial_metrics = context.get_connection_metrics();
    assert_eq!(initial_metrics.bytes_sent, 0);
    assert_eq!(initial_metrics.bytes_received, 0);
    assert_eq!(initial_metrics.messages_sent, 0);
    assert_eq!(initial_metrics.messages_received, 0);
    
    // Simulate sending a message
    let test_message = TestMessage {
        id: 1,
        content: "Test message".to_string(),
        timestamp: 1234567890,
    };
    
    context.send_message(&test_message).unwrap();
    
    // Simulate receiving a message
    let received_message = Message {
        data: serde_json::to_vec(&test_message).unwrap(),
        message_type: MessageType::Text,
    };
    context.handle_message(received_message);
    
    // Check updated metrics
    let updated_metrics = context.get_connection_metrics();
    assert!(updated_metrics.messages_sent > 0);
    assert!(updated_metrics.messages_received > 0);
    assert!(updated_metrics.bytes_sent > 0);
    assert!(updated_metrics.bytes_received > 0);
}

#[test]
fn test_presence_tracking() {
    let provider = WebSocketProvider::new("ws://localhost:8080");
    let context = WebSocketContext::new(provider);
    
    // Initial presence should be empty
    let initial_presence = context.get_presence();
    assert!(initial_presence.is_empty());
    
    // Add user presence
    let user_presence = leptos_ws::reactive::UserPresence {
        user_id: "user123".to_string(),
        status: "online".to_string(),
        last_seen: 1234567890,
    };
    
    context.update_presence("user123", user_presence.clone());
    
    // Check presence was updated
    let updated_presence = context.get_presence();
    assert_eq!(updated_presence.len(), 1);
    assert_eq!(updated_presence.get("user123").unwrap(), &user_presence);
    
    // Update presence
    let updated_user_presence = leptos_ws::reactive::UserPresence {
        user_id: "user123".to_string(),
        status: "away".to_string(),
        last_seen: 1234567891,
    };
    
    context.update_presence("user123", updated_user_presence.clone());
    
    // Check presence was updated
    let final_presence = context.get_presence();
    assert_eq!(final_presence.len(), 1);
    assert_eq!(final_presence.get("user123").unwrap(), &updated_user_presence);
}

#[test]
fn test_heartbeat_functionality() {
    let config = WebSocketConfig {
        url: "ws://localhost:8080".to_string(),
        protocols: vec![],
        heartbeat_interval: Some(30),
        reconnect_interval: Some(5),
        max_reconnect_attempts: Some(10),
        codec: Box::new(JsonCodec::new()),
    };
    
    let provider = WebSocketProvider::with_config(config);
    let context = WebSocketContext::new(provider);
    
    // Check heartbeat is configured
    assert!(context.heartbeat_interval().is_some());
    assert_eq!(context.heartbeat_interval().unwrap(), 30);
    
    // Simulate heartbeat
    let heartbeat_result = context.send_heartbeat();
    assert!(heartbeat_result.is_ok());
    
    // Check that heartbeat was sent
    let sent_messages = context.get_sent_messages::<serde_json::Value>();
    assert!(!sent_messages.is_empty());
    
    // Verify the heartbeat message structure
    let heartbeat_msg = &sent_messages[0];
    assert_eq!(heartbeat_msg["type"], "ping");
    assert!(heartbeat_msg["timestamp"].is_number());
}

#[test]
fn test_reconnection_logic() {
    let config = WebSocketConfig {
        url: "ws://localhost:8080".to_string(),
        protocols: vec![],
        heartbeat_interval: Some(30),
        reconnect_interval: Some(5),
        max_reconnect_attempts: Some(3),
        codec: Box::new(JsonCodec::new()),
    };
    
    let provider = WebSocketProvider::with_config(config);
    let context = WebSocketContext::new(provider);
    
    // Check reconnection settings
    assert_eq!(context.reconnect_interval(), 5);
    assert_eq!(context.max_reconnect_attempts(), 3);
    
    // Simulate connection failure
    context.set_connection_state(ConnectionState::Disconnected);
    
    // Trigger reconnection
    let reconnect_result = context.attempt_reconnection();
    assert!(reconnect_result.is_ok());
    
    // Check reconnection attempt was recorded
    assert!(context.reconnection_attempts() > 0);
}

#[test]
fn test_message_batching() {
    let provider = WebSocketProvider::new("ws://localhost:8080");
    let context = WebSocketContext::new(provider);
    
    // Send multiple messages quickly
    let messages = vec![
        TestMessage { id: 1, content: "Message 1".to_string(), timestamp: 1 },
        TestMessage { id: 2, content: "Message 2".to_string(), timestamp: 2 },
        TestMessage { id: 3, content: "Message 3".to_string(), timestamp: 3 },
    ];
    
    for message in &messages {
        context.send_message(message).unwrap();
    }
    
    // Check all messages were queued
    let sent_messages = context.get_sent_messages::<TestMessage>();
    assert_eq!(sent_messages.len(), 3);
    
    // Process batched messages
    let batch_result = context.process_message_batch();
    assert!(batch_result.is_ok());
}

#[test]
fn test_error_handling() {
    let provider = WebSocketProvider::new("ws://localhost:8080");
    let context = WebSocketContext::new(provider);
    
    // Test sending message when disconnected
    let test_message = TestMessage {
        id: 1,
        content: "Test".to_string(),
        timestamp: 1234567890,
    };
    
    // Should queue message even when disconnected
    let result = context.send_message(&test_message);
    assert!(result.is_ok());
    
    // Test handling invalid message
    let invalid_message = Message {
        data: vec![0xFF, 0xFE, 0xFD], // Invalid JSON
        message_type: MessageType::Text,
    };
    
    context.handle_message(invalid_message);
    
    // Should handle gracefully without crashing
    let received_messages = context.get_received_messages::<TestMessage>();
    assert_eq!(received_messages.len(), 0); // No valid messages received
}

#[test]
fn test_concurrent_operations() {
    let provider = WebSocketProvider::new("ws://localhost:8080");
    let context = WebSocketContext::new(provider);
    
    // Test concurrent message sending
    let handles: Vec<_> = (0..10)
        .map(|i| {
            let context = context.clone();
            std::thread::spawn(move || {
                let message = TestMessage {
                    id: i,
                    content: format!("Message {}", i),
                    timestamp: i as u64,
                };
                context.send_message(&message)
            })
        })
        .collect();
    
    // Wait for all threads to complete
    for handle in handles {
        assert!(handle.join().unwrap().is_ok());
    }
    
    // Check all messages were sent
    let sent_messages = context.get_sent_messages::<TestMessage>();
    assert_eq!(sent_messages.len(), 10);
}

#[test]
fn test_connection_lifecycle() {
    let provider = WebSocketProvider::new("ws://localhost:8080");
    let context = WebSocketContext::new(provider);
    
    // Initial state
    assert_eq!(context.connection_state(), ConnectionState::Disconnected);
    
    // Connect
    let connect_result = context.connect();
    assert!(connect_result.is_ok());
    assert_eq!(context.connection_state(), ConnectionState::Connected);
    
    // Send message while connected
    let test_message = TestMessage {
        id: 1,
        content: "Hello".to_string(),
        timestamp: 1234567890,
    };
    
    let send_result = context.send_message(&test_message);
    assert!(send_result.is_ok());
    
    // Disconnect
    let disconnect_result = context.disconnect();
    assert!(disconnect_result.is_ok());
    assert_eq!(context.connection_state(), ConnectionState::Disconnected);
}

#[test]
fn test_message_filtering() {
    let provider = WebSocketProvider::new("ws://localhost:8080");
    let context = WebSocketContext::new(provider);
    
    // Set up message filter
    context.set_message_filter(|msg: &Message| {
        // Simplified filter - just check if it's a text message
        matches!(msg.message_type, MessageType::Text)
    });
    
    // Send messages with different IDs
    let messages = vec![
        TestMessage { id: 1, content: "Odd".to_string(), timestamp: 1 },
        TestMessage { id: 2, content: "Even".to_string(), timestamp: 2 },
        TestMessage { id: 3, content: "Odd".to_string(), timestamp: 3 },
        TestMessage { id: 4, content: "Even".to_string(), timestamp: 4 },
    ];
    
    for message in &messages {
        let msg = Message {
            data: serde_json::to_vec(message).unwrap(),
            message_type: MessageType::Text,
        };
        context.handle_message(msg);
    }
    
    // All text messages should be received (simplified filter)
    let received_messages = context.get_received_messages::<TestMessage>();
    assert_eq!(received_messages.len(), 4); // All messages are text messages
}

#[test]
fn test_connection_quality_monitoring() {
    let provider = WebSocketProvider::new("ws://localhost:8080");
    let context = WebSocketContext::new(provider);
    
    // Simulate connection quality metrics
    context.update_connection_quality(0.95); // 95% quality
    
    let quality = context.get_connection_quality();
    assert_eq!(quality, 0.95);
    
    // Simulate poor connection
    context.update_connection_quality(0.3); // 30% quality
    
    let poor_quality = context.get_connection_quality();
    assert_eq!(poor_quality, 0.3);
    
    // Check if reconnection is triggered for poor quality
    let should_reconnect = context.should_reconnect_due_to_quality();
    assert!(should_reconnect);
}

#[test]
fn test_message_acknowledgment() {
    let provider = WebSocketProvider::new("ws://localhost:8080");
    let context = WebSocketContext::new(provider);
    
    // Send message with acknowledgment
    let test_message = TestMessage {
        id: 1,
        content: "Ack test".to_string(),
        timestamp: 1234567890,
    };
    
    let ack_result = context.send_message_with_ack(&test_message);
    assert!(ack_result.is_ok());
    
    // Simulate acknowledgment
    let ack_id = ack_result.unwrap();
    context.acknowledge_message(ack_id);
    
    // Check acknowledgment was recorded
    let acked_messages = context.get_acknowledged_messages();
    assert!(acked_messages.contains(&ack_id));
}

#[test]
fn test_connection_pooling() {
    let provider = WebSocketProvider::new("ws://localhost:8080");
    let context = WebSocketContext::new(provider);
    
    // Test connection pooling
    let pool_size = context.get_connection_pool_size();
    assert!(pool_size >= 1);
    
    // Test getting connection from pool
    let connection = context.get_connection_from_pool();
    assert!(connection.is_some());
    
    // Test returning connection to pool
    let return_result = context.return_connection_to_pool(connection.unwrap());
    assert!(return_result.is_ok());
}
