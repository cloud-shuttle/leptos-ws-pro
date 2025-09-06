//! TDD tests for real WebSocket implementation
//!
//! These tests drive the implementation of actual WebSocket connections
//! using tokio-tungstenite, replacing the current simulated implementation.

use futures::{SinkExt, StreamExt};
use leptos_ws_pro::transport::{
    ConnectionState, Message, MessageType, Transport, TransportConfig, TransportError,
    websocket::WebSocketConnection,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::net::TcpListener;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message as WsMessage;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct TestMessage {
    id: u32,
    content: String,
}

/// Helper function to start a test WebSocket server
async fn start_test_server() -> (TcpListener, u16) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    (listener, port)
}

/// Helper function to run a simple echo server
async fn run_echo_server(listener: TcpListener) {
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            let ws_stream = accept_async(stream).await.unwrap();
            let (mut write, mut read) = ws_stream.split();

            // Echo back messages
            while let Some(msg) = read.next().await {
                let msg = msg.unwrap();
                if msg.is_text() || msg.is_binary() {
                    write.send(msg).await.unwrap();
                }
            }
        }
    });
}

#[tokio::test]
async fn test_real_websocket_connection() {
    // Given: A WebSocket server running on localhost
    let (listener, port) = start_test_server().await;
    run_echo_server(listener).await;

    // When: Client connects to the server
    let config = TransportConfig {
        url: format!("ws://127.0.0.1:{}", port),
        ..Default::default()
    };
    let mut client = WebSocketConnection::new(config).await.unwrap();
    let result = client.connect(&format!("ws://127.0.0.1:{}", port)).await;

    // Then: Connection should succeed
    assert!(result.is_ok());
    assert_eq!(client.state(), ConnectionState::Connected);
}

#[tokio::test]
async fn test_websocket_message_sending() {
    // Given: A connected WebSocket client and echo server
    let (listener, port) = start_test_server().await;
    run_echo_server(listener).await;

    let config = TransportConfig {
        url: format!("ws://127.0.0.1:{}", port),
        ..Default::default()
    };
    let mut client = WebSocketConnection::new(config).await.unwrap();
    client
        .connect(&format!("ws://127.0.0.1:{}", port))
        .await
        .unwrap();

    // When: Client sends a text message
    let message = Message {
        data: "Hello, WebSocket!".as_bytes().to_vec(),
        message_type: MessageType::Text,
    };
    let (mut stream, mut sink) = client.split();

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
async fn test_websocket_binary_message() {
    // Given: A connected WebSocket client and echo server
    let (listener, port) = start_test_server().await;
    run_echo_server(listener).await;

    let config = TransportConfig {
        url: format!("ws://127.0.0.1:{}", port),
        ..Default::default()
    };
    let mut client = WebSocketConnection::new(config).await.unwrap();
    client
        .connect(&format!("ws://127.0.0.1:{}", port))
        .await
        .unwrap();

    // When: Client sends a binary message
    let binary_data = vec![0x01, 0x02, 0x03, 0x04];
    let message = Message {
        data: binary_data.clone(),
        message_type: MessageType::Binary,
    };
    let (mut stream, mut sink) = client.split();

    // Then: Binary message should be sent and received
    let send_result = sink.send(message.clone()).await;
    assert!(send_result.is_ok());

    let received = stream.next().await;
    assert!(received.is_some());
    let received_msg = received.unwrap().unwrap();
    assert_eq!(received_msg, message);
}

#[tokio::test]
async fn test_websocket_connection_timeout() {
    // Given: A non-existent server address
    let config = TransportConfig {
        url: "ws://127.0.0.1:99999".to_string(),
        ..Default::default()
    };
    let mut client = WebSocketConnection::new(config).await.unwrap();

    // When: Client tries to connect to non-existent server
    let result = client.connect("ws://127.0.0.1:99999").await;

    // Then: Should fail with connection error
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        TransportError::ConnectionFailed(_)
    ));
    assert_eq!(client.state(), ConnectionState::Disconnected);
}

#[tokio::test]
async fn test_websocket_disconnect() {
    // Given: A connected WebSocket client
    let (listener, port) = start_test_server().await;
    run_echo_server(listener).await;

    let config = TransportConfig {
        url: format!("ws://127.0.0.1:{}", port),
        ..Default::default()
    };
    let mut client = WebSocketConnection::new(config).await.unwrap();
    client
        .connect(&format!("ws://127.0.0.1:{}", port))
        .await
        .unwrap();
    assert_eq!(client.state(), ConnectionState::Connected);

    // When: Client disconnects
    let result = client.disconnect().await;

    // Then: Should disconnect successfully
    assert!(result.is_ok());
    assert_eq!(client.state(), ConnectionState::Disconnected);
}

#[tokio::test]
async fn test_websocket_reconnection() {
    // Given: A WebSocket client that was connected
    let (listener, port) = start_test_server().await;
    run_echo_server(listener).await;

    let config = TransportConfig {
        url: format!("ws://127.0.0.1:{}", port),
        ..Default::default()
    };
    let mut client = WebSocketConnection::new(config).await.unwrap();

    // First connection
    client
        .connect(&format!("ws://127.0.0.1:{}", port))
        .await
        .unwrap();
    assert_eq!(client.state(), ConnectionState::Connected);

    // Disconnect
    client.disconnect().await.unwrap();
    assert_eq!(client.state(), ConnectionState::Disconnected);

    // When: Client reconnects
    let result = client.connect(&format!("ws://127.0.0.1:{}", port)).await;

    // Then: Should reconnect successfully
    assert!(result.is_ok());
    assert_eq!(client.state(), ConnectionState::Connected);
}

#[tokio::test]
async fn test_websocket_serialized_message() {
    // Given: A connected WebSocket client and echo server
    let (listener, port) = start_test_server().await;
    run_echo_server(listener).await;

    let config = TransportConfig {
        url: format!("ws://127.0.0.1:{}", port),
        ..Default::default()
    };
    let mut client = WebSocketConnection::new(config).await.unwrap();
    client
        .connect(&format!("ws://127.0.0.1:{}", port))
        .await
        .unwrap();

    // When: Client sends a serialized message
    let test_msg = TestMessage {
        id: 42,
        content: "Test message".to_string(),
    };
    let json = serde_json::to_string(&test_msg).unwrap();
    let message = Message {
        data: json.as_bytes().to_vec(),
        message_type: MessageType::Text,
    };
    let (mut stream, mut sink) = client.split();

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
        let deserialized: TestMessage = serde_json::from_str(&received_json).unwrap();
        assert_eq!(deserialized, test_msg);
    } else {
        panic!("Expected text message");
    }
}

#[tokio::test]
async fn test_websocket_multiple_messages() {
    // Given: A connected WebSocket client and echo server
    let (listener, port) = start_test_server().await;
    run_echo_server(listener).await;

    let config = TransportConfig {
        url: format!("ws://127.0.0.1:{}", port),
        ..Default::default()
    };
    let mut client = WebSocketConnection::new(config).await.unwrap();
    client
        .connect(&format!("ws://127.0.0.1:{}", port))
        .await
        .unwrap();

    // When: Client sends multiple messages
    let (mut stream, mut sink) = client.split();
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
