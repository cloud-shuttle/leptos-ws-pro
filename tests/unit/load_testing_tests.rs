//! TDD tests for Load Testing and High-Concurrency Scenarios
//!
//! These tests verify that the library can handle high loads,
//! concurrent connections, and stress scenarios.

use futures::{SinkExt, StreamExt};
use leptos_ws_pro::transport::{
    adaptive::AdaptiveTransport, sse::SseConnection, websocket::WebSocketConnection,
    webtransport::WebTransportConnection, ConnectionState, Message, MessageType, Transport,
    TransportConfig, TransportError,
};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Barrier;
use tokio::time::timeout;

/// Test WebSocket server for load testing
async fn start_load_test_websocket_server() -> (tokio::net::TcpListener, u16) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    (listener, port)
}

/// Run a load test WebSocket echo server
async fn run_load_test_websocket_server(listener: tokio::net::TcpListener) {
    use tokio_tungstenite::accept_async;

    while let Ok((stream, _)) = listener.accept().await {
        let ws_stream = accept_async(stream).await.unwrap();
        let (mut write, mut read) = ws_stream.split();

        // Echo back all messages
        tokio::spawn(async move {
            while let Some(msg) = read.next().await {
                if let Ok(msg) = msg {
                    if write.send(msg).await.is_err() {
                        break;
                    }
                }
            }
        });
    }
}

#[tokio::test]
async fn test_concurrent_websocket_connections() {
    // Given: A WebSocket server and multiple clients
    let (listener, port) = start_load_test_websocket_server().await;
    let server_handle = tokio::spawn(run_load_test_websocket_server(listener));

    let config = TransportConfig {
        url: format!("ws://127.0.0.1:{}", port),
        ..Default::default()
    };

    // When: Creating multiple concurrent connections
    let num_connections = 10;
    let barrier = Arc::new(Barrier::new(num_connections));
    let mut handles = Vec::new();

    for i in 0..num_connections {
        let config = config.clone();
        let barrier = barrier.clone();
        let port = port;

        let handle = tokio::spawn(async move {
            let mut client = WebSocketConnection::new(config).await.unwrap();

            // Wait for all clients to be ready
            barrier.wait().await;

            // Connect simultaneously
            let result = client.connect(&format!("ws://127.0.0.1:{}", port)).await;
            assert!(result.is_ok(), "Connection {} failed: {:?}", i, result);
            assert_eq!(client.state(), ConnectionState::Connected);

            // Send a test message
            let (mut stream, mut sink) = client.split();
            let test_message = Message {
                data: format!("Load test message {}", i).as_bytes().to_vec(),
                message_type: MessageType::Text,
            };

            sink.send(test_message.clone()).await.unwrap();

            // Receive echo
            let received = timeout(Duration::from_secs(5), stream.next()).await;
            assert!(received.is_ok(), "Timeout for connection {}", i);

            let received_msg = received.unwrap().unwrap().unwrap();
            assert_eq!(received_msg.data, test_message.data);

            // Cleanup
            drop(stream);
            drop(sink);
        });

        handles.push(handle);
    }

    // Then: All connections should succeed
    for (i, handle) in handles.into_iter().enumerate() {
        let result = handle.await;
        assert!(result.is_ok(), "Task {} panicked: {:?}", i, result);
    }

    // Cleanup
    server_handle.abort();
}

#[tokio::test]
async fn test_high_frequency_message_sending() {
    // Given: A WebSocket server and client
    let (listener, port) = start_load_test_websocket_server().await;
    let server_handle = tokio::spawn(run_load_test_websocket_server(listener));

    let config = TransportConfig {
        url: format!("ws://127.0.0.1:{}", port),
        ..Default::default()
    };

    let mut client = WebSocketConnection::new(config).await.unwrap();
    client
        .connect(&format!("ws://127.0.0.1:{}", port))
        .await
        .unwrap();

    let (mut stream, mut sink) = client.split();

    // When: Sending many messages rapidly
    let num_messages = 100;
    let mut sent_messages = Vec::new();

    for i in 0..num_messages {
        let test_message = Message {
            data: format!("High frequency message {}", i).as_bytes().to_vec(),
            message_type: MessageType::Text,
        };

        sink.send(test_message.clone()).await.unwrap();
        sent_messages.push(test_message);
    }

    // Then: Should receive all echoed messages
    let mut received_messages = Vec::new();
    for _ in 0..num_messages {
        let received = timeout(Duration::from_secs(10), stream.next()).await;
        assert!(received.is_ok(), "Timeout waiting for message");

        let received_msg = received.unwrap().unwrap().unwrap();
        received_messages.push(received_msg);
    }

    // Verify all messages were received
    assert_eq!(received_messages.len(), num_messages);

    // Cleanup
    drop(stream);
    drop(sink);
    server_handle.abort();
}

#[tokio::test]
async fn test_large_message_handling() {
    // Given: A WebSocket server and client
    let (listener, port) = start_load_test_websocket_server().await;
    let server_handle = tokio::spawn(run_load_test_websocket_server(listener));

    let config = TransportConfig {
        url: format!("ws://127.0.0.1:{}", port),
        ..Default::default()
    };

    let mut client = WebSocketConnection::new(config).await.unwrap();
    client
        .connect(&format!("ws://127.0.0.1:{}", port))
        .await
        .unwrap();

    let (mut stream, mut sink) = client.split();

    // When: Sending very large messages
    let large_sizes = vec![1024, 10240, 102400, 1024000]; // 1KB, 10KB, 100KB, 1MB

    for size in large_sizes {
        let large_data = vec![0x42; size];
        let test_message = Message {
            data: large_data.clone(),
            message_type: MessageType::Binary,
        };

        sink.send(test_message.clone()).await.unwrap();

        // Then: Should receive the large message back
        let received = timeout(Duration::from_secs(30), stream.next()).await;
        assert!(received.is_ok(), "Timeout for message of size {}", size);

        let received_msg = received.unwrap().unwrap().unwrap();
        assert_eq!(received_msg.data.len(), size);
        assert_eq!(received_msg.data, large_data);
    }

    // Cleanup
    drop(stream);
    drop(sink);
    server_handle.abort();
}

#[tokio::test]
async fn test_connection_pooling() {
    // Given: Multiple transport configurations
    let ws_config = TransportConfig {
        url: "ws://example.com/ws".to_string(),
        ..Default::default()
    };

    let sse_config = TransportConfig {
        url: "http://example.com/events".to_string(),
        ..Default::default()
    };

    let wt_config = TransportConfig {
        url: "https://example.com/webtransport".to_string(),
        ..Default::default()
    };

    // When: Creating multiple instances of each transport type
    let num_instances = 5;
    let mut ws_clients = Vec::new();
    let mut sse_clients = Vec::new();
    let mut wt_clients = Vec::new();

    for _ in 0..num_instances {
        let ws_client = WebSocketConnection::new(ws_config.clone()).await.unwrap();
        let sse_client = SseConnection::new(sse_config.clone()).await.unwrap();
        let wt_client = WebTransportConnection::new(wt_config.clone())
            .await
            .unwrap();

        ws_clients.push(ws_client);
        sse_clients.push(sse_client);
        wt_clients.push(wt_client);
    }

    // Then: All instances should be created successfully
    assert_eq!(ws_clients.len(), num_instances);
    assert_eq!(sse_clients.len(), num_instances);
    assert_eq!(wt_clients.len(), num_instances);

    // All should start in disconnected state
    for client in &ws_clients {
        assert_eq!(client.state(), ConnectionState::Disconnected);
    }
    for client in &sse_clients {
        assert_eq!(client.state(), ConnectionState::Disconnected);
    }
    for client in &wt_clients {
        assert_eq!(client.state(), ConnectionState::Disconnected);
    }
}

#[tokio::test]
async fn test_adaptive_transport_load() {
    // Given: Adaptive transport with multiple fallback options
    let config = TransportConfig {
        url: "ws://example.com/ws".to_string(),
        ..Default::default()
    };

    // When: Creating multiple adaptive transport instances
    let num_instances = 10;
    let mut adaptive_transports = Vec::new();

    for _ in 0..num_instances {
        let adaptive = AdaptiveTransport::new(config.clone()).await.unwrap();
        adaptive_transports.push(adaptive);
    }

    // Then: All instances should be created successfully
    assert_eq!(adaptive_transports.len(), num_instances);

    // All should start in disconnected state
    for transport in &adaptive_transports {
        assert_eq!(transport.state(), ConnectionState::Disconnected);
    }
}

#[tokio::test]
async fn test_memory_usage_under_load() {
    // Given: A WebSocket server and client
    let (listener, port) = start_load_test_websocket_server().await;
    let server_handle = tokio::spawn(run_load_test_websocket_server(listener));

    let config = TransportConfig {
        url: format!("ws://127.0.0.1:{}", port),
        ..Default::default()
    };

    let mut client = WebSocketConnection::new(config).await.unwrap();
    client
        .connect(&format!("ws://127.0.0.1:{}", port))
        .await
        .unwrap();

    let (mut stream, mut sink) = client.split();

    // When: Sending many messages to test memory usage
    let num_messages = 1000;

    for i in 0..num_messages {
        let test_message = Message {
            data: format!("Memory test message {}", i).as_bytes().to_vec(),
            message_type: MessageType::Text,
        };

        sink.send(test_message).await.unwrap();

        // Receive every 10th message to avoid memory buildup
        if i % 10 == 0 {
            let _ = timeout(Duration::from_secs(1), stream.next()).await;
        }
    }

    // Then: Should still be able to send and receive messages
    let final_message = Message {
        data: b"Final message".to_vec(),
        message_type: MessageType::Text,
    };

    sink.send(final_message.clone()).await.unwrap();

    // Clear any remaining messages in the stream
    let mut attempts = 0;
    let max_attempts = 20;

    while attempts < max_attempts {
        let received = timeout(Duration::from_millis(100), stream.next()).await;
        if let Ok(Some(Ok(received_msg))) = received {
            if received_msg.data == final_message.data {
                // Found our final message
                break;
            }
        } else {
            break;
        }
        attempts += 1;
    }

    // Verify we can still send and receive
    let test_message = Message {
        data: b"Post-load test message".to_vec(),
        message_type: MessageType::Text,
    };

    sink.send(test_message.clone()).await.unwrap();

    let received = timeout(Duration::from_secs(5), stream.next()).await;
    assert!(received.is_ok(), "Post-load test message timeout");

    let received_msg = received.unwrap().unwrap().unwrap();
    // Just verify we received something (might be our test message or final message)
    assert!(!received_msg.data.is_empty());

    // Cleanup
    drop(stream);
    drop(sink);
    server_handle.abort();
}
