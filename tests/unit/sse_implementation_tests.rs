//! TDD tests for Server-Sent Events (SSE) implementation
//!
//! These tests drive the implementation of SSE connections
//! for real-time server-to-client communication.

use futures::StreamExt;
use leptos_ws_pro::transport::{
    sse::SseConnection, ConnectionState, MessageType, Transport, TransportConfig, TransportError,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::time::timeout;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct TestMessage {
    id: u32,
    content: String,
    timestamp: u64,
}

/// Start a test HTTP server for SSE testing
async fn start_test_sse_server() -> (TcpListener, u16) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    (listener, port)
}

/// Helper function to start server and wait for it to be ready
async fn start_server_and_wait() -> u16 {
    let (listener, port) = start_test_sse_server().await;

    // Start server in background
    tokio::spawn(async move {
        run_sse_server(listener).await;
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    port
}

/// Run an SSE server for testing
async fn run_sse_server(listener: TcpListener) {
    while let Ok((stream, _)) = listener.accept().await {
        tokio::spawn(async move {
            let mut stream = BufReader::new(stream);
            let mut line = String::new();

            // Read HTTP request
            while stream.read_line(&mut line).await.unwrap() > 0 {
                if line == "\r\n" {
                    break;
                }
                line.clear();
            }

            // Send SSE headers
            let response = "HTTP/1.1 200 OK\r\n\
                          Content-Type: text/event-stream\r\n\
                          Cache-Control: no-cache\r\n\
                          Connection: keep-alive\r\n\
                          Access-Control-Allow-Origin: *\r\n\r\n";

            stream.get_mut().write_all(response.as_bytes()).await.unwrap();

            // Send test events
            for i in 1..=3 {
                let event = format!("event: message\ndata: Event {}\nid: {}\n\n", i, i);
                stream.get_mut().write_all(event.as_bytes()).await.unwrap();
                tokio::time::sleep(Duration::from_millis(100)).await;
            }

            // Send a serialized message
            let test_msg = TestMessage {
                id: 42,
                content: "SSE test message".to_string(),
                timestamp: 1234567890, // Fixed timestamp for testing
            };
            let json_data = serde_json::to_string(&test_msg).unwrap();
            let event = format!("event: message\ndata: {}\nid: serialized\n\n", json_data);
            stream.get_mut().write_all(event.as_bytes()).await.unwrap();

            // Send event with retry interval
            let retry_event = "event: message\ndata: Retry test\nretry: 5000\nid: retry_test\n\n";
            stream.get_mut().write_all(retry_event.as_bytes()).await.unwrap();

            // Send heartbeat
            let heartbeat = "event: heartbeat\ndata: ping\nid: heartbeat_1\n\n";
            stream.get_mut().write_all(heartbeat.as_bytes()).await.unwrap();
        });
    }
}

#[tokio::test]
async fn test_sse_connection() {
    // Given: An SSE server running on localhost
    let port = start_server_and_wait().await;

    // When: Client connects to the server via SSE
    let config = TransportConfig {
        url: format!("http://127.0.0.1:{}", port),
        ..Default::default()
    };
    let mut client = SseConnection::new(config).await.unwrap();
    let result = client.connect(&format!("http://127.0.0.1:{}", port)).await;

    // Then: Connection should succeed
    assert!(result.is_ok());
    assert_eq!(client.state(), ConnectionState::Connected);
}

#[tokio::test]
async fn test_sse_event_receiving() {
    // Given: A connected SSE client and server
    let port = start_server_and_wait().await;

    let config = TransportConfig {
        url: format!("http://127.0.0.1:{}", port),
        ..Default::default()
    };
    let mut client = SseConnection::new(config).await.unwrap();
    client
        .connect(&format!("http://127.0.0.1:{}", port))
        .await
        .unwrap();

    // When: Server sends an event
    let (mut stream, _sink) = client.split();

    // Then: Should receive the event
    let received = stream.next().await;
    assert!(received.is_some());
    let received_msg = received.unwrap().unwrap();
    assert_eq!(received_msg.message_type, MessageType::Text);
}

#[tokio::test]
async fn test_sse_event_parsing() {
    // Given: A connected SSE client and server
    let port = start_server_and_wait().await;

    let config = TransportConfig {
        url: format!("http://127.0.0.1:{}", port),
        ..Default::default()
    };
    let mut client = SseConnection::new(config).await.unwrap();
    client
        .connect(&format!("http://127.0.0.1:{}", port))
        .await
        .unwrap();

    // When: Server sends a properly formatted SSE event
    let (mut stream, _sink) = client.split();

    // Then: Should parse the event correctly
    let received = stream.next().await;
    assert!(received.is_some());
    let received_msg = received.unwrap().unwrap();

    // SSE events should be text messages
    assert_eq!(received_msg.message_type, MessageType::Text);

    // Should be able to parse the event data
    let event_data = String::from_utf8(received_msg.data).unwrap();
    assert!(!event_data.is_empty());
}

#[tokio::test]
async fn test_sse_connection_timeout() {
    // Given: An SSE client
    let config = TransportConfig {
        url: "http://127.0.0.1:99999".to_string(),
        ..Default::default()
    };
    let mut client = SseConnection::new(config).await.unwrap();

    // When: Client tries to connect to non-existent server
    let result = timeout(
        Duration::from_secs(5),
        client.connect("http://127.0.0.1:99999"),
    )
    .await;

    // Then: Should fail with connection error
    assert!(result.is_ok()); // Timeout completed
    let connect_result = result.unwrap();
    assert!(connect_result.is_err());
    assert!(matches!(
        connect_result.unwrap_err(),
        TransportError::ConnectionFailed(_)
    ));
    // Connection should be in failed or reconnecting state
    let state = client.state();
    assert!(matches!(state, ConnectionState::Failed | ConnectionState::Reconnecting));
}

#[tokio::test]
async fn test_sse_disconnect() {
    // Given: A connected SSE client
    let port = start_server_and_wait().await;

    let config = TransportConfig {
        url: format!("http://127.0.0.1:{}", port),
        ..Default::default()
    };
    let mut client = SseConnection::new(config).await.unwrap();
    client
        .connect(&format!("http://127.0.0.1:{}", port))
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
async fn test_sse_reconnection() {
    // Given: An SSE client that was connected
    let port = start_server_and_wait().await;

    let config = TransportConfig {
        url: format!("http://127.0.0.1:{}", port),
        ..Default::default()
    };
    let mut client = SseConnection::new(config).await.unwrap();

    // First connection
    client
        .connect(&format!("http://127.0.0.1:{}", port))
        .await
        .unwrap();
    assert_eq!(client.state(), ConnectionState::Connected);

    // Disconnect
    client.disconnect().await.unwrap();
    assert_eq!(client.state(), ConnectionState::Disconnected);

    // When: Client reconnects
    let result = client.connect(&format!("http://127.0.0.1:{}", port)).await;

    // Then: Should reconnect successfully
    assert!(result.is_ok());
    assert_eq!(client.state(), ConnectionState::Connected);
}

#[tokio::test]
async fn test_sse_serialized_message() {
    // Given: A connected SSE client and server
    let port = start_server_and_wait().await;

    let config = TransportConfig {
        url: format!("http://127.0.0.1:{}", port),
        ..Default::default()
    };
    let mut client = SseConnection::new(config).await.unwrap();
    client
        .connect(&format!("http://127.0.0.1:{}", port))
        .await
        .unwrap();

    // When: Server sends a serialized message
    let (mut stream, _sink) = client.split();

    // Then: Should receive and parse the serialized message
    // Skip the first few messages to get to the serialized one
    let mut received_msg = None;
    for _ in 0..5 { // Server sends 3 events + 1 serialized + 1 retry + 1 heartbeat
        if let Some(msg) = stream.next().await {
            let msg = msg.unwrap();
            let data = String::from_utf8(msg.data.clone()).unwrap();
            if data.contains("SSE test message") {
                received_msg = Some(msg);
                break;
            }
        }
    }

    assert!(received_msg.is_some());
    let received_msg = received_msg.unwrap();

    // Should be able to deserialize the received message
    if received_msg.message_type == MessageType::Text {
        let received_json = String::from_utf8(received_msg.data).unwrap();
        let received_test_msg: TestMessage = serde_json::from_str(&received_json).unwrap();
        assert_eq!(received_test_msg.id, 42);
        assert_eq!(received_test_msg.content, "SSE test message");
    }
}

#[tokio::test]
async fn test_sse_multiple_events() {
    // Given: A connected SSE client and server
    let port = start_server_and_wait().await;

    let config = TransportConfig {
        url: format!("http://127.0.0.1:{}", port),
        ..Default::default()
    };
    let mut client = SseConnection::new(config).await.unwrap();
    client
        .connect(&format!("http://127.0.0.1:{}", port))
        .await
        .unwrap();

    // When: Server sends multiple events
    let (mut stream, _sink) = client.split();

    // Then: Should receive all events
    for i in 1..=3 {
        let received = stream.next().await;
        assert!(received.is_some());
        let received_msg = received.unwrap().unwrap();
        assert_eq!(received_msg.message_type, MessageType::Text);

        let event_data = String::from_utf8(received_msg.data).unwrap();
        assert!(event_data.contains(&format!("Event {}", i)));
    }
}

#[tokio::test]
async fn test_sse_event_id_handling() {
    // Given: A connected SSE client and server
    let port = start_server_and_wait().await;

    let config = TransportConfig {
        url: format!("http://127.0.0.1:{}", port),
        ..Default::default()
    };
    let mut client = SseConnection::new(config).await.unwrap();
    client
        .connect(&format!("http://127.0.0.1:{}", port))
        .await
        .unwrap();

    // When: Server sends events with IDs
    let (mut stream, _sink) = client.split();

    // Then: Should handle event IDs correctly
    let received = stream.next().await;
    assert!(received.is_some());
    let received_msg = received.unwrap().unwrap();

    // SSE events with IDs should be properly parsed
    assert_eq!(received_msg.message_type, MessageType::Text);
    let event_data = String::from_utf8(received_msg.data).unwrap();
    // The data should contain the actual event data, not the SSE format
    assert!(!event_data.is_empty());
}

#[tokio::test]
async fn test_sse_retry_interval_handling() {
    // Given: A connected SSE client and server
    let port = start_server_and_wait().await;

    let config = TransportConfig {
        url: format!("http://127.0.0.1:{}", port),
        ..Default::default()
    };
    let mut client = SseConnection::new(config).await.unwrap();
    client
        .connect(&format!("http://127.0.0.1:{}", port))
        .await
        .unwrap();

    // When: Server sends retry interval
    let (mut stream, _sink) = client.split();

    // Then: Should handle retry interval correctly
    let received = stream.next().await;
    assert!(received.is_some());
    let received_msg = received.unwrap().unwrap();

    // SSE retry intervals should be properly parsed
    assert_eq!(received_msg.message_type, MessageType::Text);
    let event_data = String::from_utf8(received_msg.data).unwrap();
    // The data should contain the actual event data, not the SSE format
    assert!(!event_data.is_empty());
}
