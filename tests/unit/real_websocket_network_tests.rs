//! TDD tests for Real WebSocket Network Implementation
//!
//! These tests verify actual network connectivity using tokio-tungstenite,
//! replacing simulated connections with real WebSocket functionality.

use leptos_ws_pro::transport::{
    ConnectionState, Message, MessageType, Transport, TransportConfig, TransportError,
    websocket::WebSocketConnection,
};
use futures::{SinkExt, StreamExt};
use std::time::Duration;
use tokio::time::timeout;

/// Test WebSocket server for real network testing
async fn start_test_websocket_server() -> (tokio::net::TcpListener, u16) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    (listener, port)
}

/// Run a simple WebSocket echo server
async fn run_websocket_echo_server(listener: tokio::net::TcpListener) {
    use tokio_tungstenite::accept_async;
    
    while let Ok((stream, _)) = listener.accept().await {
        let ws_stream = accept_async(stream).await.unwrap();
        let (mut write, mut read) = ws_stream.split();
        
        // Echo back all messages
        while let Some(msg) = read.next().await {
            if let Ok(msg) = msg {
                if write.send(msg).await.is_err() {
                    break;
                }
            }
        }
    }
}

#[tokio::test]
async fn test_real_websocket_connection() {
    // Given: A real WebSocket server and client
    let (listener, port) = start_test_websocket_server().await;
    let server_handle = tokio::spawn(run_websocket_echo_server(listener));
    
    let config = TransportConfig {
        url: format!("ws://127.0.0.1:{}", port),
        ..Default::default()
    };
    
    let mut client = WebSocketConnection::new(config).await.unwrap();
    
    // When: Connecting to the real server
    let result = client.connect(&format!("ws://127.0.0.1:{}", port)).await;
    
    // Then: Should connect successfully
    assert!(result.is_ok(), "Failed to connect to real WebSocket server: {:?}", result);
    assert_eq!(client.state(), ConnectionState::Connected);
    
    // Cleanup
    let _ = client.disconnect().await;
    server_handle.abort();
}

#[tokio::test]
async fn test_real_websocket_message_exchange() {
    // Given: A real WebSocket server and connected client
    let (listener, port) = start_test_websocket_server().await;
    let server_handle = tokio::spawn(run_websocket_echo_server(listener));
    
    let config = TransportConfig {
        url: format!("ws://127.0.0.1:{}", port),
        ..Default::default()
    };
    
    let mut client = WebSocketConnection::new(config).await.unwrap();
    client.connect(&format!("ws://127.0.0.1:{}", port)).await.unwrap();
    
    let (mut stream, mut sink) = client.split();
    
    // When: Sending a message to the real server
    let test_message = Message {
        data: b"Hello, Real WebSocket!".to_vec(),
        message_type: MessageType::Text,
    };
    
    sink.send(test_message.clone()).await.unwrap();
    
    // Then: Should receive the echoed message back
    let received = timeout(Duration::from_secs(5), stream.next()).await;
    assert!(received.is_ok(), "Timeout waiting for echoed message");
    
    let received_msg = received.unwrap().unwrap().unwrap();
    assert_eq!(received_msg.data, test_message.data);
    assert_eq!(received_msg.message_type, test_message.message_type);
    
    // Cleanup - drop the split components
    drop(stream);
    drop(sink);
    server_handle.abort();
}

#[tokio::test]
async fn test_real_websocket_binary_message() {
    // Given: A real WebSocket server and connected client
    let (listener, port) = start_test_websocket_server().await;
    let server_handle = tokio::spawn(run_websocket_echo_server(listener));
    
    let config = TransportConfig {
        url: format!("ws://127.0.0.1:{}", port),
        ..Default::default()
    };
    
    let mut client = WebSocketConnection::new(config).await.unwrap();
    client.connect(&format!("ws://127.0.0.1:{}", port)).await.unwrap();
    
    let (mut stream, mut sink) = client.split();
    
    // When: Sending a binary message
    let binary_data = vec![0x01, 0x02, 0x03, 0x04, 0x05];
    let test_message = Message {
        data: binary_data.clone(),
        message_type: MessageType::Binary,
    };
    
    sink.send(test_message.clone()).await.unwrap();
    
    // Then: Should receive the echoed binary message back
    let received = timeout(Duration::from_secs(5), stream.next()).await;
    assert!(received.is_ok(), "Timeout waiting for echoed binary message");
    
    let received_msg = received.unwrap().unwrap().unwrap();
    assert_eq!(received_msg.data, binary_data);
    assert_eq!(received_msg.message_type, MessageType::Binary);
    
    // Cleanup - drop the split components
    drop(stream);
    drop(sink);
    server_handle.abort();
}

#[tokio::test]
async fn test_real_websocket_connection_failure() {
    // Given: A client trying to connect to a non-existent server
    let config = TransportConfig {
        url: "ws://127.0.0.1:99999".to_string(), // Non-existent port
        ..Default::default()
    };
    
    let mut client = WebSocketConnection::new(config).await.unwrap();
    
    // When: Attempting to connect to non-existent server
    let result = client.connect("ws://127.0.0.1:99999").await;
    
    // Then: Should fail with appropriate error
    assert!(result.is_err(), "Expected connection to fail");
    assert_eq!(client.state(), ConnectionState::Disconnected);
    
    match result.unwrap_err() {
        TransportError::ConnectionFailed(msg) => {
            assert!(msg.contains("connection") || msg.contains("refused") || msg.contains("timeout"));
        }
        _ => panic!("Expected ConnectionFailed error"),
    }
}

#[tokio::test]
async fn test_real_websocket_reconnection() {
    // Given: A WebSocket server that we can start/stop
    let (listener, port) = start_test_websocket_server().await;
    let server_handle = tokio::spawn(run_websocket_echo_server(listener));
    
    let config = TransportConfig {
        url: format!("ws://127.0.0.1:{}", port),
        ..Default::default()
    };
    
    let mut client = WebSocketConnection::new(config).await.unwrap();
    
    // When: Connecting, disconnecting, and reconnecting
    assert!(client.connect(&format!("ws://127.0.0.1:{}", port)).await.is_ok());
    assert_eq!(client.state(), ConnectionState::Connected);
    
    assert!(client.disconnect().await.is_ok());
    assert_eq!(client.state(), ConnectionState::Disconnected);
    
    assert!(client.connect(&format!("ws://127.0.0.1:{}", port)).await.is_ok());
    assert_eq!(client.state(), ConnectionState::Connected);
    
    // Cleanup
    let _ = client.disconnect().await;
    server_handle.abort();
}

#[tokio::test]
async fn test_real_websocket_multiple_messages() {
    // Given: A real WebSocket server and connected client
    let (listener, port) = start_test_websocket_server().await;
    let server_handle = tokio::spawn(run_websocket_echo_server(listener));
    
    let config = TransportConfig {
        url: format!("ws://127.0.0.1:{}", port),
        ..Default::default()
    };
    
    let mut client = WebSocketConnection::new(config).await.unwrap();
    client.connect(&format!("ws://127.0.0.1:{}", port)).await.unwrap();
    
    let (mut stream, mut sink) = client.split();
    
    // When: Sending multiple messages
    let messages = vec![
        Message { data: b"Message 1".to_vec(), message_type: MessageType::Text },
        Message { data: b"Message 2".to_vec(), message_type: MessageType::Text },
        Message { data: b"Message 3".to_vec(), message_type: MessageType::Text },
    ];
    
    for msg in &messages {
        sink.send(msg.clone()).await.unwrap();
    }
    
    // Then: Should receive all echoed messages back
    for expected_msg in &messages {
        let received = timeout(Duration::from_secs(5), stream.next()).await;
        assert!(received.is_ok(), "Timeout waiting for message");
        
        let received_msg = received.unwrap().unwrap().unwrap();
        assert_eq!(received_msg.data, expected_msg.data);
    }
    
    // Cleanup - drop the split components
    drop(stream);
    drop(sink);
    server_handle.abort();
}

#[tokio::test]
async fn test_real_websocket_large_message() {
    // Given: A real WebSocket server and connected client
    let (listener, port) = start_test_websocket_server().await;
    let server_handle = tokio::spawn(run_websocket_echo_server(listener));
    
    let config = TransportConfig {
        url: format!("ws://127.0.0.1:{}", port),
        ..Default::default()
    };
    
    let mut client = WebSocketConnection::new(config).await.unwrap();
    client.connect(&format!("ws://127.0.0.1:{}", port)).await.unwrap();
    
    let (mut stream, mut sink) = client.split();
    
    // When: Sending a large message (64KB)
    let large_data = vec![0x42; 65536]; // 64KB of data
    let test_message = Message {
        data: large_data.clone(),
        message_type: MessageType::Binary,
    };
    
    sink.send(test_message.clone()).await.unwrap();
    
    // Then: Should receive the echoed large message back
    let received = timeout(Duration::from_secs(10), stream.next()).await;
    assert!(received.is_ok(), "Timeout waiting for large message");
    
    let received_msg = received.unwrap().unwrap().unwrap();
    assert_eq!(received_msg.data.len(), 65536);
    assert_eq!(received_msg.data, large_data);
    
    // Cleanup - drop the split components
    drop(stream);
    drop(sink);
    server_handle.abort();
}
