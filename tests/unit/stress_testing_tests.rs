//! TDD tests for Stress Testing and Edge Cases
//!
//! These tests verify that the library handles extreme conditions,
//! edge cases, and failure scenarios gracefully.

use futures::{SinkExt, StreamExt};
use leptos_ws_pro::transport::{
    adaptive::AdaptiveTransport, sse::SseConnection, websocket::WebSocketConnection,
    webtransport::WebTransportConnection, ConnectionState, Message, MessageType, Transport,
    TransportConfig, TransportError,
};
use std::time::Duration;
use tokio::time::timeout;

#[tokio::test]
async fn test_rapid_connect_disconnect_cycle() {
    // Given: A WebSocket client
    let config = TransportConfig {
        url: "ws://127.0.0.1:99999/ws".to_string(), // Non-existent server
        ..Default::default()
    };

    let mut client = WebSocketConnection::new(config).await.unwrap();

    // When: Rapidly connecting and disconnecting
    for i in 0..100 {
        let connect_result = client.connect("ws://127.0.0.1:99999/ws").await;
        assert!(connect_result.is_err(), "Connection {} should fail", i);
        assert_eq!(client.state(), ConnectionState::Disconnected);

        let disconnect_result = client.disconnect().await;
        assert!(disconnect_result.is_ok(), "Disconnect {} should succeed", i);
        assert_eq!(client.state(), ConnectionState::Disconnected);
    }

    // Then: Client should still be functional
    assert_eq!(client.state(), ConnectionState::Disconnected);
}

#[tokio::test]
async fn test_invalid_url_handling() {
    // Given: Various invalid URLs
    let invalid_urls = vec![
        "invalid-url",
        "http://",
        "ws://",
        "https://",
        "ftp://example.com",
        "ws://[invalid-ipv6",
        "ws://example.com:99999/ws",
        "",
        "   ",
        "ws://example.com:0/ws",
    ];

    let config = TransportConfig {
        url: "ws://example.com/ws".to_string(),
        ..Default::default()
    };

    // When: Attempting to connect to invalid URLs
    for (i, invalid_url) in invalid_urls.iter().enumerate() {
        let mut client = WebSocketConnection::new(config.clone()).await.unwrap();

        let result = client.connect(invalid_url).await;
        assert!(result.is_err(), "URL {} should fail: {}", i, invalid_url);
        assert_eq!(client.state(), ConnectionState::Disconnected);
    }
}

#[tokio::test]
async fn test_extreme_message_sizes() {
    // Given: A WebSocket client (will fail to connect, but we can test message creation)
    let config = TransportConfig {
        url: "ws://127.0.0.1:99999/ws".to_string(),
        ..Default::default()
    };

    let mut client = WebSocketConnection::new(config).await.unwrap();

    // When: Creating messages with extreme sizes
    let extreme_sizes = vec![
        0,                 // Empty message
        1,                 // Single byte
        1024,              // 1KB
        1024 * 1024,       // 1MB
        10 * 1024 * 1024,  // 10MB
        100 * 1024 * 1024, // 100MB
    ];

    for size in extreme_sizes {
        let large_data = vec![0x42; size];
        let message = Message {
            data: large_data,
            message_type: MessageType::Binary,
        };

        // Should be able to create the message (even if we can't send it)
        assert_eq!(message.data.len(), size);
        assert_eq!(message.message_type, MessageType::Binary);
    }

    // Then: Client should still be functional
    assert_eq!(client.state(), ConnectionState::Disconnected);
}

#[tokio::test]
async fn test_concurrent_transport_creation() {
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

    // When: Creating many transports concurrently
    let num_transports = 1000;
    let mut handles = Vec::new();

    for i in 0..num_transports {
        let config = match i % 3 {
            0 => ws_config.clone(),
            1 => sse_config.clone(),
            _ => wt_config.clone(),
        };

        let handle = tokio::spawn(async move {
            match i % 3 {
                0 => {
                    let client = WebSocketConnection::new(config).await;
                    assert!(client.is_ok(), "WebSocket creation failed for {}", i);
                    client.unwrap().state()
                }
                1 => {
                    let client = SseConnection::new(config).await;
                    assert!(client.is_ok(), "SSE creation failed for {}", i);
                    client.unwrap().state()
                }
                _ => {
                    let client = WebTransportConnection::new(config).await;
                    assert!(client.is_ok(), "WebTransport creation failed for {}", i);
                    client.unwrap().state()
                }
            }
        });

        handles.push(handle);
    }

    // Then: All transports should be created successfully
    for (i, handle) in handles.into_iter().enumerate() {
        let result = handle.await;
        assert!(result.is_ok(), "Transport {} creation panicked", i);
        assert_eq!(result.unwrap(), ConnectionState::Disconnected);
    }
}

#[tokio::test]
async fn test_memory_pressure_handling() {
    // Given: A WebSocket client
    let config = TransportConfig {
        url: "ws://127.0.0.1:99999/ws".to_string(),
        ..Default::default()
    };

    let mut client = WebSocketConnection::new(config).await.unwrap();

    // When: Creating many large messages to test memory pressure
    let num_messages = 10000;
    let message_size = 10000; // 10KB per message

    let mut messages = Vec::new();
    for i in 0..num_messages {
        let large_data = vec![0x42; message_size];
        let message = Message {
            data: large_data,
            message_type: MessageType::Binary,
        };
        messages.push(message);

        // Every 1000 messages, verify client is still functional
        if i % 1000 == 0 {
            assert_eq!(client.state(), ConnectionState::Disconnected);
        }
    }

    // Then: Should handle memory pressure gracefully
    assert_eq!(messages.len(), num_messages);
    assert_eq!(client.state(), ConnectionState::Disconnected);

    // Verify all messages have correct size
    for message in &messages {
        assert_eq!(message.data.len(), message_size);
    }
}

#[tokio::test]
async fn test_error_recovery() {
    // Given: A WebSocket client
    let config = TransportConfig {
        url: "ws://127.0.0.1:99999/ws".to_string(),
        ..Default::default()
    };

    let mut client = WebSocketConnection::new(config).await.unwrap();

    // When: Attempting various operations that should fail
    let connect_result = client.connect("ws://127.0.0.1:99999/ws").await;
    assert!(connect_result.is_err());
    assert_eq!(client.state(), ConnectionState::Disconnected);

    // Disconnect should still work even when not connected
    let disconnect_result = client.disconnect().await;
    assert!(disconnect_result.is_ok());
    assert_eq!(client.state(), ConnectionState::Disconnected);

    // Try to connect again
    let connect_result2 = client.connect("ws://127.0.0.1:99999/ws").await;
    assert!(connect_result2.is_err());
    assert_eq!(client.state(), ConnectionState::Disconnected);

    // Then: Client should be in a consistent state
    assert_eq!(client.state(), ConnectionState::Disconnected);
}

#[tokio::test]
async fn test_adaptive_transport_stress() {
    // Given: Adaptive transport configuration
    let config = TransportConfig {
        url: "ws://127.0.0.1:99999/ws".to_string(),
        ..Default::default()
    };

    // When: Creating many adaptive transport instances
    let num_instances = 500;
    let mut adaptive_transports = Vec::new();

    for _ in 0..num_instances {
        let transport = AdaptiveTransport::new(config.clone()).await.unwrap();
        adaptive_transports.push(transport);
    }

    // Then: All instances should be created successfully
    assert_eq!(adaptive_transports.len(), num_instances);

    // All should be in disconnected state
    for transport in &adaptive_transports {
        assert_eq!(transport.state(), ConnectionState::Disconnected);
    }

    // Test that they can still be used
    for mut transport in adaptive_transports {
        let result = transport.connect("ws://127.0.0.1:99999/ws").await;
        assert!(result.is_err()); // Should fail to connect
        assert_eq!(transport.state(), ConnectionState::Disconnected);
    }
}

#[tokio::test]
async fn test_transport_config_edge_cases() {
    // Given: Various edge case configurations
    let edge_case_configs = vec![
        TransportConfig {
            url: "".to_string(),
            ..Default::default()
        },
        TransportConfig {
            url: "   ".to_string(),
            ..Default::default()
        },
        TransportConfig {
            url: "ws://example.com/ws".to_string(),
            ..Default::default()
        },
    ];

    // When: Creating transports with edge case configurations
    for (i, config) in edge_case_configs.iter().enumerate() {
        let ws_client = WebSocketConnection::new(config.clone()).await;
        let sse_client = SseConnection::new(config.clone()).await;
        let wt_client = WebTransportConnection::new(config.clone()).await;

        // Then: Should handle edge cases gracefully
        match i {
            0 | 1 => {
                // Empty or whitespace URLs should still create clients
                assert!(
                    ws_client.is_ok(),
                    "WebSocket creation failed for config {}",
                    i
                );
                assert!(sse_client.is_ok(), "SSE creation failed for config {}", i);
                assert!(
                    wt_client.is_ok(),
                    "WebTransport creation failed for config {}",
                    i
                );
            }
            _ => {
                // Valid URLs should definitely work
                assert!(
                    ws_client.is_ok(),
                    "WebSocket creation failed for config {}",
                    i
                );
                assert!(sse_client.is_ok(), "SSE creation failed for config {}", i);
                assert!(
                    wt_client.is_ok(),
                    "WebTransport creation failed for config {}",
                    i
                );
            }
        }
    }
}

#[tokio::test]
async fn test_timeout_handling() {
    // Given: A WebSocket client
    let config = TransportConfig {
        url: "ws://127.0.0.1:99999/ws".to_string(),
        ..Default::default()
    };

    let mut client = WebSocketConnection::new(config).await.unwrap();

    // When: Testing timeout scenarios
    let connect_future = client.connect("ws://127.0.0.1:99999/ws");
    let timeout_result = timeout(Duration::from_millis(100), connect_future).await;

    // Then: Should either timeout or fail quickly
    match timeout_result {
        Ok(connect_result) => {
            // Connection failed quickly (expected)
            assert!(connect_result.is_err(), "Connection should fail");
        }
        Err(_) => {
            // Connection timed out (also acceptable)
            // This means the connection attempt took longer than 100ms
        }
    }
    assert_eq!(client.state(), ConnectionState::Disconnected);

    // Disconnect should still work after timeout
    let disconnect_result = client.disconnect().await;
    assert!(disconnect_result.is_ok());
    assert_eq!(client.state(), ConnectionState::Disconnected);
}
