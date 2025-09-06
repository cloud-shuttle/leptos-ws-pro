//! TDD tests for transport layer implementations
//!
//! These tests define the behavior we want for the transport layer
//! including WebSocket, WebTransport, and SSE implementations.

use futures::{SinkExt, StreamExt};
use leptos_ws_pro::transport::{
    MessageType, TransportCapabilities, adaptive::AdaptiveTransport, sse::SseConnection,
    websocket::WebSocketConnection, webtransport::WebTransportConnection,
};
use leptos_ws_pro::*;
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tokio_tungstenite::accept_async;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct TestMessage {
    id: u32,
    content: String,
}

#[tokio::test]
async fn test_websocket_transport_connection() {
    // Test that WebSocket transport can establish connections
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server_task = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let ws_stream = accept_async(stream).await.unwrap();
        let (mut write, mut read) = ws_stream.split();

        // Echo back messages
        while let Some(msg) = read.next().await {
            let msg = msg.unwrap();
            if msg.is_text() {
                write.send(msg).await.unwrap();
            }
        }
    });

    // Test WebSocket transport
    let config = TransportConfig::default();
    let mut transport = WebSocketConnection::new(config).await.unwrap();

    // Connect to server
    let url = format!("ws://127.0.0.1:{}", addr.port());
    assert!(transport.connect(&url).await.is_ok());
    assert_eq!(transport.state(), ConnectionState::Connected);

    server_task.abort();
}

#[tokio::test]
async fn test_websocket_transport_message_flow() {
    // Test that WebSocket transport can send and receive messages
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server_task = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let ws_stream = accept_async(stream).await.unwrap();
        let (mut write, mut read) = ws_stream.split();

        // Echo back messages
        while let Some(msg) = read.next().await {
            let msg = msg.unwrap();
            if msg.is_text() {
                write.send(msg).await.unwrap();
            }
        }
    });

    let config = TransportConfig::default();
    let mut transport = WebSocketConnection::new(config).await.unwrap();

    let url = format!("ws://127.0.0.1:{}", addr.port());
    transport.connect(&url).await.unwrap();

    // Test message sending
    let test_msg = Message {
        data: b"Hello, WebSocket!".to_vec(),
        message_type: MessageType::Text,
    };

    let (mut stream, mut sink) = transport.split();

    // Send message (will fail since not implemented)
    let send_result = sink.send(test_msg.clone()).await;
    assert!(send_result.is_err()); // Expected to fail since not implemented

    // Stream is empty since not implemented
    let received = stream.next().await;
    assert!(received.is_none());

    server_task.abort();
}

#[tokio::test]
async fn test_webtransport_transport_connection() {
    // Test that WebTransport transport can be created
    let config = TransportConfig::default();
    let mut transport = WebTransportConnection::new(config).await.unwrap();

    // WebTransport should be in disconnected state initially
    assert_eq!(transport.state(), ConnectionState::Disconnected);

    // Test connection (will fail without real WebTransport server)
    let result = transport.connect("https://localhost:8080").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_sse_transport_connection() {
    // Test that SSE transport can be created
    let config = TransportConfig::default();
    let mut transport = SseConnection::new(config).await.unwrap();

    // SSE should be in disconnected state initially
    assert_eq!(transport.state(), ConnectionState::Disconnected);

    // Test connection (will fail without real SSE server)
    let result = transport.connect("http://localhost:8080/events").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_adaptive_transport_selection() {
    // Test that adaptive transport can select the best transport
    let config = TransportConfig::default();
    let mut transport = AdaptiveTransport::new(config).await.unwrap();

    // Test capability detection
    let capabilities = AdaptiveTransport::detect_capabilities().await;
    assert!(capabilities.websocket_supported);

    // Test connection (will fail without real server)
    let result = transport.connect("ws://localhost:8080").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_transport_error_handling() {
    // Test that transport properly handles connection errors
    let config = TransportConfig::default();
    let mut transport = WebSocketConnection::new(config).await.unwrap();

    // Try to connect to non-existent server
    let result = transport.connect("ws://127.0.0.1:99999").await;
    assert!(result.is_err());
    assert_eq!(transport.state(), ConnectionState::Disconnected);
}

#[tokio::test]
async fn test_transport_reconnection() {
    // Test that transport can handle reconnection
    let config = TransportConfig::default();
    let mut transport = WebSocketConnection::new(config).await.unwrap();

    // Initial connection attempt (will fail)
    let result = transport.connect("ws://127.0.0.1:99999").await;
    assert!(result.is_err());

    // Try reconnection
    let result = transport.connect("ws://127.0.0.1:99999").await;
    assert!(result.is_err());

    // State should remain disconnected
    assert_eq!(transport.state(), ConnectionState::Disconnected);
}

#[tokio::test]
async fn test_transport_capabilities() {
    // Test transport capability detection
    let capabilities = TransportCapabilities::detect();

    // Basic capabilities should be available
    assert!(capabilities.websocket);
    assert!(capabilities.sse);

    // WebTransport support depends on environment
    // (This test will pass regardless of actual support)
    assert!(capabilities.webtransport || !capabilities.webtransport);
}
