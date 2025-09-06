//! TDD tests for Adaptive Transport implementation
//!
//! These tests drive the implementation of adaptive transport
//! that automatically selects the best available transport.

use futures::{SinkExt, StreamExt};
use leptos_ws_pro::transport::{
    ConnectionState, Message, MessageType, Transport, TransportConfig, TransportError,
    adaptive::AdaptiveTransport,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::time::timeout;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct TestMessage {
    id: u32,
    content: String,
    timestamp: u64,
}

/// Start a test WebSocket server for adaptive transport testing
async fn start_test_server() -> (TcpListener, u16) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    (listener, port)
}

/// Run an echo server for testing
async fn run_echo_server(listener: TcpListener) {
    use tokio_tungstenite::accept_async;
    use futures::{StreamExt, SinkExt};

    while let Ok((stream, _)) = listener.accept().await {
        let ws_stream = accept_async(stream).await.unwrap();
        let (mut write, mut read) = ws_stream.split();

        // Echo back messages
        while let Some(msg) = read.next().await {
            let msg = msg.unwrap();
            write.send(msg).await.unwrap();
        }
    }
}

#[tokio::test]
async fn test_adaptive_transport_websocket_selection() {
    // Given: A WebSocket server running on localhost
    let (listener, port) = start_test_server().await;
    run_echo_server(listener).await;

    // When: Adaptive transport connects
    let config = TransportConfig {
        url: format!("ws://127.0.0.1:{}", port),
        ..Default::default()
    };
    let mut transport = AdaptiveTransport::new(config).await.unwrap();
    let result = transport.connect(&format!("ws://127.0.0.1:{}", port)).await;

    // Then: Should select WebSocket and connect successfully
    assert!(result.is_ok());
    assert_eq!(transport.state(), ConnectionState::Connected);
    assert_eq!(transport.selected_transport(), "WebSocket");
}

#[tokio::test]
async fn test_adaptive_transport_capability_detection() {
    // Given: An adaptive transport
    let config = TransportConfig {
        url: "ws://127.0.0.1:8080".to_string(),
        ..Default::default()
    };
    let _transport = AdaptiveTransport::new(config).await.unwrap();

    // When: Checking capabilities
    let capabilities = AdaptiveTransport::detect_capabilities().await;

    // Then: Should detect available transports
    assert!(capabilities.websocket_supported);
    // WebTransport and SSE might not be supported in test environment
    // but the detection should work
}

#[tokio::test]
async fn test_adaptive_transport_fallback_mechanism() {
    // Given: An adaptive transport with fallback enabled
    let config = TransportConfig {
        url: "ws://127.0.0.1:99999".to_string(), // Non-existent server
        ..Default::default()
    };
    let mut transport = AdaptiveTransport::new(config).await.unwrap();

    // When: Trying to connect with fallback
    let result = timeout(
        Duration::from_secs(10),
        transport.connect_with_fallback("ws://127.0.0.1:99999")
    ).await;

    // Then: Should attempt fallback mechanisms
    assert!(result.is_ok()); // Timeout completed
    let connect_result = result.unwrap();
    // Should fail since no server is running, but fallback should be attempted
    assert!(connect_result.is_err());
}

#[tokio::test]
async fn test_adaptive_transport_message_sending() {
    // Given: A connected adaptive transport
    let (listener, port) = start_test_server().await;
    run_echo_server(listener).await;

    let config = TransportConfig {
        url: format!("ws://127.0.0.1:{}", port),
        ..Default::default()
    };
    let mut transport = AdaptiveTransport::new(config).await.unwrap();
    transport.connect(&format!("ws://127.0.0.1:{}", port)).await.unwrap();

    // When: Sending a message
    let message = Message {
        data: "Hello, Adaptive Transport!".as_bytes().to_vec(),
        message_type: MessageType::Text,
    };
    let (mut stream, mut sink) = transport.split();

    // Then: Message should be sent successfully
    let send_result = sink.send(message.clone()).await;
    assert!(send_result.is_ok());

    // And: Should receive the echoed message back
    let received = stream.next().await;
    assert!(received.is_some());
    let received_msg = received.unwrap().unwrap();
    assert_eq!(received_msg, message);
}

#[tokio::test]
async fn test_adaptive_transport_connection_timeout() {
    // Given: An adaptive transport
    let config = TransportConfig {
        url: "ws://127.0.0.1:99999".to_string(),
        ..Default::default()
    };
    let mut transport = AdaptiveTransport::new(config).await.unwrap();

    // When: Trying to connect to non-existent server
    let result = timeout(Duration::from_secs(5), transport.connect("ws://127.0.0.1:99999")).await;

    // Then: Should fail with connection error
    assert!(result.is_ok()); // Timeout completed
    let connect_result = result.unwrap();
    assert!(connect_result.is_err());
    assert!(matches!(
        connect_result.unwrap_err(),
        TransportError::ConnectionFailed(_)
    ));
    assert_eq!(transport.state(), ConnectionState::Disconnected);
}

#[tokio::test]
async fn test_adaptive_transport_disconnect() {
    // Given: A connected adaptive transport
    let (listener, port) = start_test_server().await;
    run_echo_server(listener).await;

    let config = TransportConfig {
        url: format!("ws://127.0.0.1:{}", port),
        ..Default::default()
    };
    let mut transport = AdaptiveTransport::new(config).await.unwrap();
    transport.connect(&format!("ws://127.0.0.1:{}", port)).await.unwrap();
    assert_eq!(transport.state(), ConnectionState::Connected);

    // When: Disconnecting
    let result = transport.disconnect().await;

    // Then: Should disconnect successfully
    assert!(result.is_ok());
    assert_eq!(transport.state(), ConnectionState::Disconnected);
}

#[tokio::test]
async fn test_adaptive_transport_reconnection() {
    // Given: An adaptive transport that was connected
    let (listener, port) = start_test_server().await;
    run_echo_server(listener).await;

    let config = TransportConfig {
        url: format!("ws://127.0.0.1:{}", port),
        ..Default::default()
    };
    let mut transport = AdaptiveTransport::new(config).await.unwrap();

    // First connection
    transport.connect(&format!("ws://127.0.0.1:{}", port)).await.unwrap();
    assert_eq!(transport.state(), ConnectionState::Connected);

    // Disconnect
    transport.disconnect().await.unwrap();
    assert_eq!(transport.state(), ConnectionState::Disconnected);

    // When: Reconnecting
    let result = transport.connect(&format!("ws://127.0.0.1:{}", port)).await;

    // Then: Should reconnect successfully
    assert!(result.is_ok());
    assert_eq!(transport.state(), ConnectionState::Connected);
}

#[tokio::test]
async fn test_adaptive_transport_serialized_message() {
    // Given: A connected adaptive transport
    let (listener, port) = start_test_server().await;
    run_echo_server(listener).await;

    let config = TransportConfig {
        url: format!("ws://127.0.0.1:{}", port),
        ..Default::default()
    };
    let mut transport = AdaptiveTransport::new(config).await.unwrap();
    transport.connect(&format!("ws://127.0.0.1:{}", port)).await.unwrap();

    // When: Sending a serialized message
    let test_msg = TestMessage {
        id: 42,
        content: "Adaptive transport test message".to_string(),
        timestamp: 1234567890,
    };
    let json = serde_json::to_string(&test_msg).unwrap();
    let message = Message {
        data: json.as_bytes().to_vec(),
        message_type: MessageType::Text,
    };
    let (mut stream, mut sink) = transport.split();

    // Then: Should send and receive the serialized message
    let send_result = sink.send(message.clone()).await;
    assert!(send_result.is_ok());

    let received = stream.next().await;
    assert!(received.is_some());
    let received_msg = received.unwrap().unwrap();
    assert_eq!(received_msg, message);

    // And: Should be able to deserialize the received message
    if received_msg.message_type == MessageType::Text {
        let received_json = String::from_utf8(received_msg.data).unwrap();
        let received_test_msg: TestMessage = serde_json::from_str(&received_json).unwrap();
        assert_eq!(received_test_msg, test_msg);
    }
}

#[tokio::test]
async fn test_adaptive_transport_multiple_messages() {
    // Given: A connected adaptive transport
    let (listener, port) = start_test_server().await;
    run_echo_server(listener).await;

    let config = TransportConfig {
        url: format!("ws://127.0.0.1:{}", port),
        ..Default::default()
    };
    let mut transport = AdaptiveTransport::new(config).await.unwrap();
    transport.connect(&format!("ws://127.0.0.1:{}", port)).await.unwrap();

    // When: Sending multiple messages
    let (mut stream, mut sink) = transport.split();
    let messages = vec![
        Message {
            data: "Message 1".as_bytes().to_vec(),
            message_type: MessageType::Text,
        },
        Message {
            data: "Message 2".as_bytes().to_vec(),
            message_type: MessageType::Text,
        },
        Message {
            data: "Message 3".as_bytes().to_vec(),
            message_type: MessageType::Text,
        },
    ];

    // Send all messages
    for message in &messages {
        let send_result = sink.send(message.clone()).await;
        assert!(send_result.is_ok());
    }

    // Then: Should receive all messages back
    for expected_message in &messages {
        let received = stream.next().await;
        assert!(received.is_some());
        let received_msg = received.unwrap().unwrap();
        assert_eq!(received_msg, *expected_message);
    }
}

#[tokio::test]
async fn test_adaptive_transport_performance_monitoring() {
    // Given: A connected adaptive transport
    let (listener, port) = start_test_server().await;
    run_echo_server(listener).await;

    let config = TransportConfig {
        url: format!("ws://127.0.0.1:{}", port),
        ..Default::default()
    };
    let mut transport = AdaptiveTransport::new(config).await.unwrap();
    transport.connect(&format!("ws://127.0.0.1:{}", port)).await.unwrap();

    // When: Monitoring performance
    let metrics = transport.get_performance_metrics();

    // Then: Should have performance metrics
    assert!(metrics.connection_count >= 1);
    // Metrics should be non-negative (u64 is always >= 0)
    // assert!(metrics.message_count >= 0);
    // assert!(metrics.error_count >= 0);
}

#[tokio::test]
async fn test_adaptive_transport_dynamic_switching() {
    // Given: An adaptive transport with multiple transport options
    let config = TransportConfig {
        url: "ws://127.0.0.1:8080".to_string(),
        ..Default::default()
    };
    let transport = AdaptiveTransport::new(config).await.unwrap();

    // When: Checking if dynamic switching is supported
    let can_switch = transport.can_switch_transport();

    // Then: Should support dynamic switching
    assert!(can_switch);

    // When: Getting available transports
    let available = transport.get_available_transports();

    // Then: Should have WebSocket available at minimum
    assert!(available.contains(&"WebSocket".to_string()));
}
