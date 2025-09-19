//! WebSocket Integration Tests
//!
//! Tests the WebSocket transport with real servers to validate
//! end-to-end functionality.

use leptos_ws_pro::transport::{WebSocketConnection, TransportConfig, Message, MessageType, TransportError};
use std::time::Duration;

// Import test servers
mod servers;
use servers::EchoServer;

#[tokio::test]
async fn test_websocket_echo_integration() {
    // Start echo server
    let server = EchoServer::new(8080).await.unwrap();
    let server_handle = tokio::spawn(async move {
        server.start().await.unwrap();
    });

    // Wait for server to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Create WebSocket client
    let config = TransportConfig {
        url: "ws://127.0.0.1:8080".to_string(),
        ..Default::default()
    };

    let mut client = WebSocketConnection::new(config).await.unwrap();

    // Connect to server
    let connect_result = client.connect("ws://127.0.0.1:8080").await;
    assert!(connect_result.is_ok(), "Failed to connect: {:?}", connect_result);

    // Test text message echo
    let test_message = Message {
        data: b"Hello, WebSocket!".to_vec(),
        message_type: MessageType::Text,
    };

    let send_result = client.send_message(&test_message).await;
    assert!(send_result.is_ok(), "Failed to send message: {:?}", send_result);

    // Receive echo response
    let response = client.receive_message().await;
    assert!(response.is_ok(), "Failed to receive message: {:?}", response);

    let received_message = response.unwrap();
    assert_eq!(received_message.data, test_message.data);
    assert_eq!(received_message.message_type, MessageType::Text);

    // Disconnect
    let disconnect_result = client.disconnect().await;
    assert!(disconnect_result.is_ok(), "Failed to disconnect: {:?}", disconnect_result);

    // Cleanup
    server_handle.abort();
}

#[tokio::test]
async fn test_websocket_binary_integration() {
    // Start echo server
    let server = EchoServer::new(8081).await.unwrap();
    let server_handle = tokio::spawn(async move {
        server.start().await.unwrap();
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Create WebSocket client
    let config = TransportConfig {
        url: "ws://127.0.0.1:8081".to_string(),
        ..Default::default()
    };

    let mut client = WebSocketConnection::new(config).await.unwrap();

    // Connect to server
    let connect_result = client.connect("ws://127.0.0.1:8081").await;
    assert!(connect_result.is_ok(), "Failed to connect: {:?}", connect_result);

    // Test binary message echo
    let binary_data = vec![0x00, 0x01, 0x02, 0x03, 0x04, 0x05];
    let test_message = Message {
        data: binary_data.clone(),
        message_type: MessageType::Binary,
    };

    let send_result = client.send_message(&test_message).await;
    assert!(send_result.is_ok(), "Failed to send binary message: {:?}", send_result);

    // Receive echo response
    let response = client.receive_message().await;
    assert!(response.is_ok(), "Failed to receive binary message: {:?}", response);

    let received_message = response.unwrap();
    assert_eq!(received_message.data, binary_data);
    assert_eq!(received_message.message_type, MessageType::Binary);

    // Disconnect
    let disconnect_result = client.disconnect().await;
    assert!(disconnect_result.is_ok(), "Failed to disconnect: {:?}", disconnect_result);

    // Cleanup
    server_handle.abort();
}

#[tokio::test]
async fn test_websocket_connection_failure() {
    // Create WebSocket client
    let config = TransportConfig {
        url: "ws://127.0.0.1:9999".to_string(), // Non-existent server
        ..Default::default()
    };

    let mut client = WebSocketConnection::new(config).await.unwrap();

    // Try to connect to non-existent server
    let connect_result = client.connect("ws://127.0.0.1:9999").await;
    assert!(connect_result.is_err(), "Should fail to connect to non-existent server");

    let error = connect_result.unwrap_err();
    assert!(matches!(error, TransportError::ConnectionFailed(_)));
}

#[tokio::test]
async fn test_websocket_send_without_connection() {
    // Create WebSocket client without connecting
    let config = TransportConfig {
        url: "ws://127.0.0.1:8080".to_string(),
        ..Default::default()
    };

    let client = WebSocketConnection::new(config).await.unwrap();

    // Try to send message without connection
    let test_message = Message {
        data: b"test".to_vec(),
        message_type: MessageType::Text,
    };

    let send_result = client.send_message(&test_message).await;
    assert!(send_result.is_err(), "Should fail to send without connection");

    let error = send_result.unwrap_err();
    assert!(matches!(error, TransportError::ConnectionFailed(_)));
}

#[tokio::test]
async fn test_websocket_receive_without_connection() {
    // Create WebSocket client without connecting
    let config = TransportConfig {
        url: "ws://127.0.0.1:8080".to_string(),
        ..Default::default()
    };

    let client = WebSocketConnection::new(config).await.unwrap();

    // Try to receive message without connection
    let receive_result = client.receive_message().await;
    assert!(receive_result.is_err(), "Should fail to receive without connection");

    let error = receive_result.unwrap_err();
    assert!(matches!(error, TransportError::ConnectionFailed(_)));
}

#[tokio::test]
async fn test_websocket_multiple_messages() {
    // Start echo server
    let server = EchoServer::new(8082).await.unwrap();
    let server_handle = tokio::spawn(async move {
        server.start().await.unwrap();
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Create WebSocket client
    let config = TransportConfig {
        url: "ws://127.0.0.1:8082".to_string(),
        ..Default::default()
    };

    let mut client = WebSocketConnection::new(config).await.unwrap();

    // Connect to server
    let connect_result = client.connect("ws://127.0.0.1:8082").await;
    assert!(connect_result.is_ok(), "Failed to connect: {:?}", connect_result);

    // Send multiple messages
    for i in 0..5 {
        let test_message = Message {
            data: format!("Message {}", i).into_bytes(),
            message_type: MessageType::Text,
        };

        let send_result = client.send_message(&test_message).await;
        assert!(send_result.is_ok(), "Failed to send message {}: {:?}", i, send_result);

        // Receive echo response
        let response = client.receive_message().await;
        assert!(response.is_ok(), "Failed to receive message {}: {:?}", i, response);

        let received_message = response.unwrap();
        assert_eq!(received_message.data, test_message.data);
    }

    // Disconnect
    let disconnect_result = client.disconnect().await;
    assert!(disconnect_result.is_ok(), "Failed to disconnect: {:?}", disconnect_result);

    // Cleanup
    server_handle.abort();
}

#[tokio::test]
async fn test_websocket_capabilities() {
    // Create WebSocket client
    let config = TransportConfig {
        url: "ws://127.0.0.1:8080".to_string(),
        ..Default::default()
    };

    let client = WebSocketConnection::new(config).await.unwrap();

    // Test capabilities
    let capabilities = client.capabilities();
    assert!(capabilities.websocket);
    assert!(capabilities.binary);
    assert!(!capabilities.webtransport);
    assert!(!capabilities.sse);
}

#[tokio::test]
async fn test_websocket_state_management() {
    // Create WebSocket client
    let config = TransportConfig {
        url: "ws://127.0.0.1:8080".to_string(),
        ..Default::default()
    };

    let mut client = WebSocketConnection::new(config).await.unwrap();

    // Initial state should be disconnected
    assert!(matches!(client.state(), leptos_ws_pro::transport::ConnectionState::Disconnected));

    // Try to connect to non-existent server (should fail)
    let connect_result = client.connect("ws://127.0.0.1:9999").await;
    assert!(connect_result.is_err());

    // State should be back to disconnected after failed connection
    assert!(matches!(client.state(), leptos_ws_pro::transport::ConnectionState::Disconnected));
}
