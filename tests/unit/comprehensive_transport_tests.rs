//! Comprehensive integration tests for all transport types
//!
//! These tests verify that all transport implementations work together
//! and that the adaptive transport can successfully fallback between them.

// use futures::StreamExt; // TODO: Remove when used
use leptos_ws_pro::transport::{
    adaptive::AdaptiveTransport, sse::SseConnection, websocket::WebSocketConnection,
    webtransport::WebTransportConnection, ConnectionState, Message, MessageType, Transport,
    TransportConfig, TransportError,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::timeout;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct TestMessage {
    id: u32,
    content: String,
    timestamp: u64,
}

/// Test that all transport types can be created successfully
#[tokio::test]
async fn test_all_transport_creation() {
    let config = TransportConfig {
        url: "ws://localhost:8080".to_string(),
        ..Default::default()
    };

    // Test WebSocket creation
    let ws_result = WebSocketConnection::new(config.clone()).await;
    assert!(ws_result.is_ok());

    // Test SSE creation
    let sse_result = SseConnection::new(config.clone()).await;
    assert!(sse_result.is_ok());

    // Test WebTransport creation
    let wt_result = WebTransportConnection::new(config.clone()).await;
    assert!(wt_result.is_ok());

    // Test Adaptive Transport creation
    let adaptive_result = AdaptiveTransport::new(config).await;
    assert!(adaptive_result.is_ok());
}

/// Test that all transport types handle connection failures gracefully
#[tokio::test]
async fn test_all_transport_connection_failures() {
    let config = TransportConfig {
        url: "ws://localhost:99999".to_string(), // Non-existent server
        ..Default::default()
    };

    // Test WebSocket connection failure
    let mut ws = WebSocketConnection::new(config.clone()).await.unwrap();
    let ws_result = timeout(Duration::from_secs(5), ws.connect("ws://localhost:99999")).await;
    assert!(ws_result.is_ok());
    let connect_result = ws_result.unwrap();
    assert!(connect_result.is_err());
    assert_eq!(ws.state(), ConnectionState::Disconnected);

    // Test SSE connection failure
    let mut sse = SseConnection::new(config.clone()).await.unwrap();
    let sse_result = timeout(
        Duration::from_secs(5),
        sse.connect("http://localhost:99999"),
    )
    .await;
    assert!(sse_result.is_ok());
    let connect_result = sse_result.unwrap();
    assert!(connect_result.is_err());
    assert_eq!(sse.state(), ConnectionState::Disconnected);

    // Test WebTransport connection failure
    let mut wt = WebTransportConnection::new(config.clone()).await.unwrap();
    let wt_result = timeout(
        Duration::from_secs(5),
        wt.connect("https://localhost:99999"),
    )
    .await;
    assert!(wt_result.is_ok());
    let connect_result = wt_result.unwrap();
    assert!(connect_result.is_err());
    assert_eq!(wt.state(), ConnectionState::Disconnected);
}

/// Test that all transport types can disconnect properly
#[tokio::test]
async fn test_all_transport_disconnect() {
    let config = TransportConfig {
        url: "ws://localhost:8080".to_string(),
        ..Default::default()
    };

    // Test WebSocket disconnect
    let mut ws = WebSocketConnection::new(config.clone()).await.unwrap();
    let result = ws.disconnect().await;
    assert!(result.is_ok());
    assert_eq!(ws.state(), ConnectionState::Disconnected);

    // Test SSE disconnect
    let mut sse = SseConnection::new(config.clone()).await.unwrap();
    let result = sse.disconnect().await;
    assert!(result.is_ok());
    assert_eq!(sse.state(), ConnectionState::Disconnected);

    // Test WebTransport disconnect
    let mut wt = WebTransportConnection::new(config.clone()).await.unwrap();
    let result = wt.disconnect().await;
    assert!(result.is_ok());
    assert_eq!(wt.state(), ConnectionState::Disconnected);

    // Test Adaptive Transport disconnect
    let mut adaptive = AdaptiveTransport::new(config.clone()).await.unwrap();
    let result = adaptive.disconnect().await;
    assert!(result.is_ok());
    assert_eq!(adaptive.state(), ConnectionState::Disconnected);
}

/// Test that all transport types can split into stream and sink
#[tokio::test]
async fn test_all_transport_split() {
    let config = TransportConfig {
        url: "ws://localhost:8080".to_string(),
        ..Default::default()
    };

    // Test WebSocket split
    let ws = WebSocketConnection::new(config.clone()).await.unwrap();
    let (ws_stream, _ws_sink) = ws.split();
    assert!(ws_stream.size_hint().0 == 0); // Empty stream when not connected

    // Test SSE split
    let sse = SseConnection::new(config.clone()).await.unwrap();
    let (sse_stream, _sse_sink) = sse.split();
    assert!(sse_stream.size_hint().0 == 0); // Empty stream when not connected

    // Test WebTransport split
    let wt = WebTransportConnection::new(config.clone()).await.unwrap();
    let (wt_stream, _wt_sink) = wt.split();
    assert!(wt_stream.size_hint().0 == 0); // Empty stream when not connected

    // Test Adaptive Transport split
    let adaptive = AdaptiveTransport::new(config.clone()).await.unwrap();
    let (adaptive_stream, _adaptive_sink) = adaptive.split();
    assert!(adaptive_stream.size_hint().0 == 0); // Empty stream when not connected
}

/// Test adaptive transport capability detection
#[tokio::test]
async fn test_adaptive_transport_capabilities() {
    let capabilities = AdaptiveTransport::detect_capabilities().await;

    // All transports should be supported now
    assert!(capabilities.websocket_supported);
    assert!(capabilities.sse_supported);
    assert!(capabilities.webtransport_supported);
}

/// Test adaptive transport fallback behavior
#[tokio::test]
async fn test_adaptive_transport_fallback() {
    let config = TransportConfig {
        url: "ws://localhost:99999".to_string(), // Non-existent server
        ..Default::default()
    };

    let mut adaptive = AdaptiveTransport::new(config.clone()).await.unwrap();

    // Should fail to connect to non-existent server
    let result = timeout(
        Duration::from_secs(10),
        adaptive.connect("ws://localhost:99999"),
    )
    .await;
    assert!(result.is_ok());
    let connect_result = result.unwrap();
    assert!(connect_result.is_err());

    // Should have tried all transports and failed
    let metrics = adaptive.get_performance_metrics();
    assert!(metrics.error_count > 0);
    assert_eq!(adaptive.state(), ConnectionState::Disconnected);
}

/// Test adaptive transport performance metrics
#[tokio::test]
async fn test_adaptive_transport_metrics() {
    let config = TransportConfig {
        url: "ws://localhost:8080".to_string(),
        ..Default::default()
    };

    let adaptive = AdaptiveTransport::new(config.clone()).await.unwrap();
    let metrics = adaptive.get_performance_metrics();

    // Initial metrics should be zero
    assert_eq!(metrics.connection_count, 0);
    assert_eq!(metrics.message_count, 0);
    assert_eq!(metrics.error_count, 0);
}

/// Test adaptive transport available transports
#[tokio::test]
async fn test_adaptive_transport_available_transports() {
    let config = TransportConfig {
        url: "ws://localhost:8080".to_string(),
        ..Default::default()
    };

    let adaptive = AdaptiveTransport::new(config.clone()).await.unwrap();
    let transports = adaptive.get_available_transports();

    // Should include all three transport types
    assert!(transports.contains(&"WebSocket".to_string()));
    assert!(transports.contains(&"SSE".to_string()));
    assert!(transports.contains(&"WebTransport".to_string()));
    assert_eq!(transports.len(), 3);
}

/// Test adaptive transport can switch transports
#[tokio::test]
async fn test_adaptive_transport_can_switch() {
    let config = TransportConfig {
        url: "ws://localhost:8080".to_string(),
        ..Default::default()
    };

    let adaptive = AdaptiveTransport::new(config.clone()).await.unwrap();

    // Adaptive transport should always be able to switch
    assert!(adaptive.can_switch_transport());
}

/// Test that all transport types implement the Transport trait correctly
#[tokio::test]
async fn test_transport_trait_implementation() {
    let config = TransportConfig {
        url: "ws://localhost:8080".to_string(),
        ..Default::default()
    };

    // Test that all transports implement the required methods
    let mut ws = WebSocketConnection::new(config.clone()).await.unwrap();
    let mut sse = SseConnection::new(config.clone()).await.unwrap();
    let mut wt = WebTransportConnection::new(config.clone()).await.unwrap();
    let mut adaptive = AdaptiveTransport::new(config.clone()).await.unwrap();

    // All should start disconnected
    assert_eq!(ws.state(), ConnectionState::Disconnected);
    assert_eq!(sse.state(), ConnectionState::Disconnected);
    assert_eq!(wt.state(), ConnectionState::Disconnected);
    assert_eq!(adaptive.state(), ConnectionState::Disconnected);

    // All should be able to disconnect
    assert!(ws.disconnect().await.is_ok());
    assert!(sse.disconnect().await.is_ok());
    assert!(wt.disconnect().await.is_ok());
    assert!(adaptive.disconnect().await.is_ok());

    // All should be able to split
    let ws = WebSocketConnection::new(config.clone()).await.unwrap();
    let sse = SseConnection::new(config.clone()).await.unwrap();
    let wt = WebTransportConnection::new(config.clone()).await.unwrap();
    let adaptive = AdaptiveTransport::new(config.clone()).await.unwrap();

    let (_, _) = ws.split();
    let (_, _) = sse.split();
    let (_, _) = wt.split();
    let (_, _) = adaptive.split();
}

/// Test message type consistency across transports
#[tokio::test]
async fn test_message_type_consistency() {
    // Test that all transports use the same Message type
    let test_message = Message {
        data: b"test".to_vec(),
        message_type: MessageType::Text,
    };

    // This test mainly ensures the types are compatible
    // In a real implementation, we would test actual message passing
    assert_eq!(test_message.data, b"test");
    assert_eq!(test_message.message_type, MessageType::Text);
}

/// Test error type consistency across transports
#[tokio::test]
async fn test_error_type_consistency() {
    // Test that all transports use the same TransportError type
    let connection_error = TransportError::ConnectionFailed("test".to_string());
    let send_error = TransportError::SendFailed("test".to_string());
    let receive_error = TransportError::ReceiveFailed("test".to_string());

    // This test ensures error types are consistent
    match connection_error {
        TransportError::ConnectionFailed(_) => {}
        _ => panic!("Wrong error type"),
    }

    match send_error {
        TransportError::SendFailed(_) => {}
        _ => panic!("Wrong error type"),
    }

    match receive_error {
        TransportError::ReceiveFailed(_) => {}
        _ => panic!("Wrong error type"),
    }
}
