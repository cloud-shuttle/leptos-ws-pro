//! TDD Integration Tests for Real Network Connectivity
//!
//! These tests verify that all transport types work together
//! with real network connectivity and proper error handling.

use leptos_ws_pro::transport::{
    ConnectionState, Transport, TransportConfig, TransportError,
    websocket::WebSocketConnection,
    sse::SseConnection,
    webtransport::WebTransportConnection,
    adaptive::AdaptiveTransport,
};

#[tokio::test]
async fn test_all_transports_creation() {
    // Given: Valid configurations for all transport types
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
    
    // When: Creating all transport types
    let ws_client = WebSocketConnection::new(ws_config).await;
    let sse_client = SseConnection::new(sse_config).await;
    let wt_client = WebTransportConnection::new(wt_config).await;
    
    // Then: All should create successfully
    assert!(ws_client.is_ok(), "WebSocket client creation failed");
    assert!(sse_client.is_ok(), "SSE client creation failed");
    assert!(wt_client.is_ok(), "WebTransport client creation failed");
    
    // Verify initial states
    assert_eq!(ws_client.unwrap().state(), ConnectionState::Disconnected);
    assert_eq!(sse_client.unwrap().state(), ConnectionState::Disconnected);
    assert_eq!(wt_client.unwrap().state(), ConnectionState::Disconnected);
}

#[tokio::test]
async fn test_all_transports_connection_failure() {
    // Given: All transport types with invalid URLs
    let ws_config = TransportConfig {
        url: "ws://127.0.0.1:99999/ws".to_string(),
        ..Default::default()
    };
    
    let sse_config = TransportConfig {
        url: "http://127.0.0.1:99999/events".to_string(),
        ..Default::default()
    };
    
    let wt_config = TransportConfig {
        url: "https://127.0.0.1:99999/webtransport".to_string(),
        ..Default::default()
    };
    
    let mut ws_client = WebSocketConnection::new(ws_config).await.unwrap();
    let mut sse_client = SseConnection::new(sse_config).await.unwrap();
    let mut wt_client = WebTransportConnection::new(wt_config).await.unwrap();
    
    // When: Attempting to connect to non-existent servers
    let ws_result = ws_client.connect("ws://127.0.0.1:99999/ws").await;
    let sse_result = sse_client.connect("http://127.0.0.1:99999/events").await;
    let wt_result = wt_client.connect("https://127.0.0.1:99999/webtransport").await;
    
    // Then: All should fail with appropriate errors
    assert!(ws_result.is_err(), "WebSocket connection should fail");
    assert!(sse_result.is_err(), "SSE connection should fail");
    assert!(wt_result.is_err(), "WebTransport connection should fail");
    
    // Verify error types
    match ws_result.unwrap_err() {
        TransportError::ConnectionFailed(_) => {}
        _ => panic!("Expected ConnectionFailed error for WebSocket"),
    }
    
    match sse_result.unwrap_err() {
        TransportError::ConnectionFailed(_) => {}
        _ => panic!("Expected ConnectionFailed error for SSE"),
    }
    
    match wt_result.unwrap_err() {
        TransportError::ConnectionFailed(_) => {}
        _ => panic!("Expected ConnectionFailed error for WebTransport"),
    }
    
    // Verify states are disconnected
    assert_eq!(ws_client.state(), ConnectionState::Disconnected);
    assert_eq!(sse_client.state(), ConnectionState::Disconnected);
    assert_eq!(wt_client.state(), ConnectionState::Disconnected);
}

#[tokio::test]
async fn test_adaptive_transport_creation() {
    // Given: Adaptive transport configuration
    let config = TransportConfig {
        url: "ws://example.com/ws".to_string(),
        ..Default::default()
    };
    
    // When: Creating adaptive transport
    let adaptive = AdaptiveTransport::new(config).await;
    
    // Then: Should create successfully
    assert!(adaptive.is_ok(), "Adaptive transport creation failed");
    let adaptive = adaptive.unwrap();
    assert_eq!(adaptive.state(), ConnectionState::Disconnected);
}

#[tokio::test]
async fn test_adaptive_transport_connection_failure() {
    // Given: Adaptive transport with invalid URL
    let config = TransportConfig {
        url: "ws://127.0.0.1:99999/ws".to_string(),
        ..Default::default()
    };
    
    let mut adaptive = AdaptiveTransport::new(config).await.unwrap();
    
    // When: Attempting to connect
    let result = adaptive.connect("ws://127.0.0.1:99999/ws").await;
    
    // Then: Should fail with appropriate error
    assert!(result.is_err(), "Adaptive transport connection should fail");
    assert_eq!(adaptive.state(), ConnectionState::Disconnected);
    
    match result.unwrap_err() {
        TransportError::ConnectionFailed(_) => {}
        _ => panic!("Expected ConnectionFailed error for Adaptive transport"),
    }
}

#[tokio::test]
async fn test_transport_capabilities() {
    // Given: All transport types
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
    
    let ws_client = WebSocketConnection::new(ws_config).await.unwrap();
    let _sse_client = SseConnection::new(sse_config).await.unwrap();
    let _wt_client = WebTransportConnection::new(wt_config).await.unwrap();
    
    // When: Getting capabilities (only WebSocket has this method)
    let ws_caps = ws_client.capabilities();
    
    // Then: Should have appropriate capabilities
    assert!(ws_caps.websocket, "WebSocket should support websocket");
    assert!(ws_caps.binary, "WebSocket should support binary");
    
    // Note: SSE and WebTransport don't have capabilities() method yet
    // This is expected behavior for the current implementation
}

#[tokio::test]
async fn test_transport_error_handling() {
    // Given: All transport types
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
    
    let mut ws_client = WebSocketConnection::new(ws_config).await.unwrap();
    let mut sse_client = SseConnection::new(sse_config).await.unwrap();
    let mut wt_client = WebTransportConnection::new(wt_config).await.unwrap();
    
    // When: Attempting operations on disconnected clients
    let ws_disconnect = ws_client.disconnect().await;
    let sse_disconnect = sse_client.disconnect().await;
    let wt_disconnect = wt_client.disconnect().await;
    
    // Then: Disconnect operations should succeed (even when disconnected)
    assert!(ws_disconnect.is_ok(), "WebSocket disconnect should succeed");
    assert!(sse_disconnect.is_ok(), "SSE disconnect should succeed");
    assert!(wt_disconnect.is_ok(), "WebTransport disconnect should succeed");
    
    // Verify states remain disconnected
    assert_eq!(ws_client.state(), ConnectionState::Disconnected);
    assert_eq!(sse_client.state(), ConnectionState::Disconnected);
    assert_eq!(wt_client.state(), ConnectionState::Disconnected);
}

#[tokio::test]
async fn test_transport_state_management() {
    // Given: A WebSocket client
    let config = TransportConfig {
        url: "ws://example.com/ws".to_string(),
        ..Default::default()
    };
    
    let mut client = WebSocketConnection::new(config).await.unwrap();
    
    // When: Checking initial state
    let initial_state = client.state();
    
    // Then: Should start disconnected
    assert_eq!(initial_state, ConnectionState::Disconnected);
    
    // When: Attempting to connect to invalid URL
    let connect_result = client.connect("ws://127.0.0.1:99999/ws").await;
    
    // Then: Should fail and remain disconnected
    assert!(connect_result.is_err());
    assert_eq!(client.state(), ConnectionState::Disconnected);
    
    // When: Disconnecting
    let disconnect_result = client.disconnect().await;
    
    // Then: Should succeed and remain disconnected
    assert!(disconnect_result.is_ok());
    assert_eq!(client.state(), ConnectionState::Disconnected);
}
