//! TDD tests for real WebSocket connections
//!
//! These tests define the behavior we want for actual WebSocket network connections.

use futures::{SinkExt, StreamExt};
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
async fn test_websocket_connection_establishment() {
    // Test that we can establish a real WebSocket connection
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    // Start server task
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

    // Test client connection
    let ws_context =
        WebSocketContext::new_with_url(&("ws://127.0.0.1:".to_string() + &addr.port().to_string()));

    // This should establish a real connection
    assert!(ws_context.connect().await.is_ok());
    assert_eq!(ws_context.connection_state(), ConnectionState::Connected);

    server_task.abort();
}

#[tokio::test]
async fn test_websocket_message_sending() {
    // Test that we can send messages over a real WebSocket connection
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

    let ws_context =
        WebSocketContext::new_with_url(&("ws://127.0.0.1:".to_string() + &addr.port().to_string()));
    ws_context.connect().await.unwrap();

    // Send a test message
    let test_msg = TestMessage {
        id: 1,
        content: "Hello, WebSocket!".to_string(),
    };

    let result = ws_context.send_message(&test_msg).await;
    assert!(result.is_ok());

    server_task.abort();
}

#[tokio::test]
async fn test_websocket_message_receiving() {
    // Test that we can receive messages from a real WebSocket connection
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server_task = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let ws_stream = accept_async(stream).await.unwrap();
        let (mut write, mut read) = ws_stream.split();

        // Send a test message to client
        let test_msg = TestMessage {
            id: 42,
            content: "Server says hello!".to_string(),
        };
        let msg_text = serde_json::to_string(&test_msg).unwrap();
        write
            .send(tokio_tungstenite::tungstenite::Message::Text(
                msg_text.into(),
            ))
            .await
            .unwrap();

        // Echo back any received messages
        while let Some(msg) = read.next().await {
            let msg = msg.unwrap();
            if msg.is_text() {
                write.send(msg).await.unwrap();
            }
        }
    });

    let ws_context =
        WebSocketContext::new_with_url(&("ws://127.0.0.1:".to_string() + &addr.port().to_string()));
    ws_context.connect().await.unwrap();

    // Wait for and receive the test message
    let received_msg: TestMessage = ws_context.receive_message().await.unwrap();
    assert_eq!(received_msg.id, 42);
    assert_eq!(received_msg.content, "Server says hello!");

    server_task.abort();
}

#[tokio::test]
async fn test_websocket_connection_errors() {
    // Test that connection errors are handled properly
    let ws_context = WebSocketContext::new_with_url("ws://127.0.0.1:99999");

    // This should fail with a connection error
    let result = ws_context.connect().await;
    assert!(result.is_err());
    assert_eq!(ws_context.connection_state(), ConnectionState::Disconnected);
}

#[tokio::test]
async fn test_websocket_reconnection() {
    // Test that we can reconnect after a connection is lost
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let ws_context =
        WebSocketContext::new_with_url(&("ws://127.0.0.1:".to_string() + &addr.port().to_string()));

    // First connection
    assert!(ws_context.connect().await.is_ok());
    assert_eq!(ws_context.connection_state(), ConnectionState::Connected);

    // Simulate connection loss
    let _ = ws_context.disconnect().await;
    assert_eq!(ws_context.connection_state(), ConnectionState::Disconnected);

    // Reconnect
    assert!(ws_context.connect().await.is_ok());
    assert_eq!(ws_context.connection_state(), ConnectionState::Connected);
}
