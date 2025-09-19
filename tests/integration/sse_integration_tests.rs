//! SSE Integration Tests
//!
//! Tests the SSE transport with real servers to validate
//! end-to-end functionality.

use leptos_ws_pro::transport::{SseConnection, TransportConfig, Message, MessageType, TransportError};
use std::time::Duration;

// Import test servers
mod servers;
use servers::SseServer;

#[tokio::test]
async fn test_sse_connection_creation() {
    // Create SSE connection
    let config = TransportConfig {
        url: "http://127.0.0.1:8080/events".to_string(),
        ..Default::default()
    };

    let connection = SseConnection::new(config).await;
    assert!(connection.is_ok(), "Failed to create SSE connection: {:?}", connection);
}

#[tokio::test]
async fn test_sse_connection_failure() {
    // Create SSE connection
    let config = TransportConfig {
        url: "http://127.0.0.1:9999/events".to_string(), // Non-existent server
        ..Default::default()
    };

    let mut connection = SseConnection::new(config).await.unwrap();

    // Try to connect to non-existent server
    let connect_result = connection.connect("http://127.0.0.1:9999/events").await;
    assert!(connect_result.is_ok()); // Connection attempt succeeds, but will fail internally

    // Wait a bit for the connection to fail
    tokio::time::sleep(Duration::from_millis(100)).await;

    // State should be failed or reconnecting
    let state = connection.state();
    assert!(matches!(state, leptos_ws_pro::transport::ConnectionState::Failed |
                      leptos_ws_pro::transport::ConnectionState::Reconnecting));
}

#[tokio::test]
async fn test_sse_event_subscription() {
    // Create SSE connection
    let config = TransportConfig {
        url: "http://127.0.0.1:8080/events".to_string(),
        ..Default::default()
    };

    let connection = SseConnection::new(config).await.unwrap();

    // Test event subscription
    let subscribe_result = connection.subscribe_to_event_type("test".to_string()).await;
    assert!(subscribe_result.is_ok());

    // Test unsubscription
    let unsubscribe_result = connection.unsubscribe_from_event_type("test".to_string()).await;
    assert!(unsubscribe_result.is_ok());
}

#[tokio::test]
async fn test_sse_heartbeat_configuration() {
    // Create SSE connection
    let config = TransportConfig {
        url: "http://127.0.0.1:8080/events".to_string(),
        ..Default::default()
    };

    let mut connection = SseConnection::new(config).await.unwrap();

    // Test heartbeat configuration
    let heartbeat_config = leptos_ws_pro::transport::sse::config::HeartbeatConfig {
        enabled: true,
        interval: Duration::from_secs(10),
        timeout: Duration::from_secs(30),
        event_type: "heartbeat".to_string(),
    };

    let enable_result = connection.enable_heartbeat(heartbeat_config).await;
    assert!(enable_result.is_ok());
}

#[tokio::test]
async fn test_sse_reconnection_strategy() {
    // Create SSE connection
    let config = TransportConfig {
        url: "http://127.0.0.1:8080/events".to_string(),
        ..Default::default()
    };

    let connection = SseConnection::new(config).await.unwrap();

    // Test reconnection strategy
    let strategy = leptos_ws_pro::transport::sse::config::ReconnectionStrategy::ExponentialBackoff {
        base_delay: Duration::from_secs(1),
        max_delay: Duration::from_secs(30),
        max_attempts: 5,
    };

    connection.set_reconnection_strategy(strategy).await;
}

#[tokio::test]
async fn test_sse_event_parsing() {
    // Create SSE connection
    let config = TransportConfig {
        url: "http://127.0.0.1:8080/events".to_string(),
        ..Default::default()
    };

    let connection = SseConnection::new(config).await.unwrap();

    // Test event parsing
    let event_data = "event: message\ndata: Hello World\nid: 123\n\n";
    let event = connection.parse_sse_event(event_data).unwrap();

    assert_eq!(event.event_type, "message");
    assert_eq!(event.data, "Hello World");
    assert_eq!(event.id, Some("123".to_string()));
}

#[tokio::test]
async fn test_sse_multiline_event_parsing() {
    // Create SSE connection
    let config = TransportConfig {
        url: "http://127.0.0.1:8080/events".to_string(),
        ..Default::default()
    };

    let connection = SseConnection::new(config).await.unwrap();

    // Test multiline event parsing
    let event_data = "event: message\ndata: Line 1\ndata: Line 2\nid: 456\n\n";
    let event = connection.parse_sse_event(event_data).unwrap();

    assert_eq!(event.event_type, "message");
    assert_eq!(event.data, "Line 1\nLine 2");
    assert_eq!(event.id, Some("456".to_string()));
}

#[tokio::test]
async fn test_sse_send_message_not_supported() {
    // Create SSE connection
    let config = TransportConfig {
        url: "http://127.0.0.1:8080/events".to_string(),
        ..Default::default()
    };

    let connection = SseConnection::new(config).await.unwrap();

    // Test that sending messages is not supported
    let test_message = Message {
        data: b"test".to_vec(),
        message_type: MessageType::Text,
    };

    let send_result = connection.send_message(&test_message).await;
    assert!(send_result.is_err());

    let error = send_result.unwrap_err();
    assert!(matches!(error, TransportError::NotSupported(_)));
}

#[tokio::test]
async fn test_sse_bidirectional_stream_not_supported() {
    // Create SSE connection
    let config = TransportConfig {
        url: "http://127.0.0.1:8080/events".to_string(),
        ..Default::default()
    };

    let mut connection = SseConnection::new(config).await.unwrap();

    // Test that bidirectional streams are not supported
    let stream_result = connection.create_bidirectional_stream().await;
    assert!(stream_result.is_err());

    let error = stream_result.unwrap_err();
    assert!(matches!(error, TransportError::NotSupported(_)));
}

#[tokio::test]
async fn test_sse_state_management() {
    // Create SSE connection
    let config = TransportConfig {
        url: "http://127.0.0.1:8080/events".to_string(),
        ..Default::default()
    };

    let mut connection = SseConnection::new(config).await.unwrap();

    // Test initial state
    assert!(matches!(connection.state(), leptos_ws_pro::transport::ConnectionState::Disconnected));

    // Test connection attempt
    let connect_result = connection.connect("http://127.0.0.1:9999/events").await;
    assert!(connect_result.is_ok());

    // Wait a bit for the connection to process
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Test disconnect
    let disconnect_result = connection.disconnect().await;
    assert!(disconnect_result.is_ok());

    // State should be disconnected
    assert!(matches!(connection.state(), leptos_ws_pro::transport::ConnectionState::Disconnected));
}

#[tokio::test]
async fn test_sse_event_handlers() {
    // Create SSE connection
    let config = TransportConfig {
        url: "http://127.0.0.1:8080/events".to_string(),
        ..Default::default()
    };

    let connection = SseConnection::new(config).await.unwrap();

    // Test event handler registration
    let handler = |_msg: Message| {
        println!("Event received!");
    };

    connection.register_event_handler("test".to_string(), handler).await;
}

#[tokio::test]
async fn test_sse_connection_loss_simulation() {
    // Create SSE connection
    let config = TransportConfig {
        url: "http://127.0.0.1:8080/events".to_string(),
        ..Default::default()
    };

    let mut connection = SseConnection::new(config).await.unwrap();

    // Test connection loss simulation
    connection.simulate_connection_loss().await;

    // State should be failed
    assert!(matches!(connection.state(), leptos_ws_pro::transport::ConnectionState::Failed));
}

#[tokio::test]
async fn test_sse_reconnect() {
    // Create SSE connection
    let config = TransportConfig {
        url: "http://127.0.0.1:8080/events".to_string(),
        ..Default::default()
    };

    let mut connection = SseConnection::new(config).await.unwrap();

    // Test reconnect (will fail due to no URL set)
    let reconnect_result = connection.reconnect().await;
    assert!(reconnect_result.is_err());

    let error = reconnect_result.unwrap_err();
    assert!(matches!(error, TransportError::ConnectionFailed(_)));
}

#[tokio::test]
async fn test_sse_heartbeat_receive() {
    // Create SSE connection
    let config = TransportConfig {
        url: "http://127.0.0.1:8080/events".to_string(),
        ..Default::default()
    };

    let connection = SseConnection::new(config).await.unwrap();

    // Test heartbeat receive
    let heartbeat = connection.receive_heartbeat().await.unwrap();
    assert_eq!(heartbeat.event_type, "heartbeat");
    assert_eq!(heartbeat.data, "ping");
    assert_eq!(heartbeat.id, Some("heartbeat_1".to_string()));
}

#[tokio::test]
async fn test_sse_event_receive() {
    // Create SSE connection
    let config = TransportConfig {
        url: "http://127.0.0.1:8080/events".to_string(),
        ..Default::default()
    };

    let connection = SseConnection::new(config).await.unwrap();

    // Test event receive
    let event = connection.receive_event("test").await.unwrap();
    assert_eq!(event.event_type, "test");
    assert_eq!(event.data, "test_data");
    assert_eq!(event.id, Some("test_id".to_string()));
}

#[tokio::test]
async fn test_sse_subscription_check() {
    // Create SSE connection
    let config = TransportConfig {
        url: "http://127.0.0.1:8080/events".to_string(),
        ..Default::default()
    };

    let connection = SseConnection::new(config).await.unwrap();

    // Test subscription check
    let is_subscribed = connection.is_subscribed_to_event_type("test").await;
    assert!(!is_subscribed);

    // Subscribe and check again
    connection.subscribe_to_event_type("test".to_string()).await.unwrap();
    let is_subscribed = connection.is_subscribed_to_event_type("test").await;
    assert!(is_subscribed);
}
