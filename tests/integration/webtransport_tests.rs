use leptos_ws_pro::{
    transport::webtransport::WebTransportConnection,
    transport::{
        ConnectionState, Transport, TransportCapabilities, TransportConfig, TransportError,
    },
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestMessage {
    id: u32,
    content: String,
    timestamp: u64,
}

#[tokio::test]
async fn test_webtransport_connection() {
    // Test WebTransport connection (will fail without real server, but tests the logic)
    let mut config = TransportConfig::default();
    config.url = "https://localhost:8080".to_string();
    config.protocols = vec!["webtransport".to_string()];
    config.heartbeat_interval = Some(Duration::from_secs(30));
    config.max_reconnect_attempts = Some(3);
    config.reconnect_delay = Duration::from_secs(5);

    let mut connection = WebTransportConnection::new(config).await.unwrap();

    // Test connection attempt
    let result = connection.connect("https://localhost:8080").await;
    // This will fail since no WebTransport server is running, but tests the real connection logic
    assert!(
        result.is_err(),
        "Expected WebTransport connection to fail without server: {:?}",
        result
    );

    // Verify the error is a real WebTransport connection error
    match result {
        Err(TransportError::ConnectionFailed(msg)) => {
            // Accept any connection failure message since we don't have a real server
            assert!(
                !msg.is_empty(),
                "Expected non-empty error message, got: {}",
                msg
            );
        }
        _ => panic!("Expected ConnectionFailed error, got: {:?}", result),
    }
}

#[tokio::test]
async fn test_webtransport_capabilities() {
    // Test WebTransport capability detection
    let capabilities = TransportCapabilities::detect();

    // WebTransport availability depends on platform
    // On native platforms, it's not yet available
    // On WASM platforms, it should be available
    #[cfg(target_arch = "wasm32")]
    {
        assert!(
            capabilities.webtransport,
            "WebTransport should be detected as available on WASM"
        );
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        assert!(
            !capabilities.webtransport,
            "WebTransport should not be available on native platforms yet"
        );
    }

    // Verify other capabilities
    assert!(
        capabilities.websocket,
        "WebSocket should always be available"
    );
    assert!(capabilities.sse, "SSE should always be available");
}

#[tokio::test]
async fn test_webtransport_stream_multiplexing() {
    // Test WebTransport stream multiplexing capabilities
    let mut config = TransportConfig::default();
    config.url = "https://localhost:8080".to_string();
    config.protocols = vec!["webtransport".to_string()];
    config.heartbeat_interval = Some(Duration::from_secs(30));
    config.max_reconnect_attempts = Some(3);
    config.reconnect_delay = Duration::from_secs(5);

    let connection = WebTransportConnection::new(config).await.unwrap();

    // Test stream creation (will fail without connection, but tests the API)
    let result = connection.create_stream().await;
    assert!(
        result.is_err(),
        "Expected stream creation to fail without connection: {:?}",
        result
    );

    // Test stream multiplexing
    let result = connection.create_multiplexed_streams(3).await;
    assert!(
        result.is_err(),
        "Expected multiplexed stream creation to fail without connection: {:?}",
        result
    );
}

#[tokio::test]
async fn test_webtransport_http3_integration() {
    // Test WebTransport HTTP/3 integration
    let mut config = TransportConfig::default();
    config.url = "https://localhost:8080".to_string();
    config.protocols = vec!["webtransport".to_string()];
    config.heartbeat_interval = Some(Duration::from_secs(30));
    config.max_reconnect_attempts = Some(3);
    config.reconnect_delay = Duration::from_secs(5);

    let connection = WebTransportConnection::new(config).await.unwrap();

    // Test HTTP/3 connection setup
    let result = connection.setup_http3_connection().await;
    assert!(
        result.is_err(),
        "Expected HTTP/3 setup to fail without server: {:?}",
        result
    );
}

#[tokio::test]
async fn test_webtransport_fallback_to_websocket() {
    // Test WebTransport fallback to WebSocket when WebTransport is not available
    let mut config = TransportConfig::default();
    config.url = "ws://localhost:8080".to_string(); // Use WebSocket URL
    config.protocols = vec!["webtransport".to_string(), "websocket".to_string()];
    config.heartbeat_interval = Some(Duration::from_secs(30));
    config.max_reconnect_attempts = Some(3);
    config.reconnect_delay = Duration::from_secs(5);

    let mut connection = WebTransportConnection::new(config).await.unwrap();

    // Test fallback mechanism
    let result = connection.connect_with_fallback().await;
    // This should attempt WebTransport first, then fallback to WebSocket
    assert!(
        result.is_err(),
        "Expected fallback connection to fail without server: {:?}",
        result
    );
}

#[tokio::test]
async fn test_webtransport_message_sending() {
    // Test WebTransport message sending
    let mut config = TransportConfig::default();
    config.url = "https://localhost:8080".to_string();
    config.protocols = vec!["webtransport".to_string()];
    config.heartbeat_interval = Some(Duration::from_secs(30));
    config.max_reconnect_attempts = Some(3);
    config.reconnect_delay = Duration::from_secs(5);

    let connection = WebTransportConnection::new(config).await.unwrap();

    let test_msg = TestMessage {
        id: 1,
        content: "Hello, WebTransport!".to_string(),
        timestamp: chrono::Utc::now().timestamp() as u64,
    };

    // Test sending message without connection
    let result = connection.send_message(&test_msg).await;
    assert!(
        result.is_err(),
        "Expected send to fail without connection: {:?}",
        result
    );
}

#[tokio::test]
async fn test_webtransport_message_receiving() {
    // Test WebTransport message receiving
    let mut config = TransportConfig::default();
    config.url = "https://localhost:8080".to_string();
    config.protocols = vec!["webtransport".to_string()];
    config.heartbeat_interval = Some(Duration::from_secs(30));
    config.max_reconnect_attempts = Some(3);
    config.reconnect_delay = Duration::from_secs(5);

    let connection = WebTransportConnection::new(config).await.unwrap();

    // Test receiving message without connection
    let result: Result<TestMessage, TransportError> = connection.receive_message().await;
    assert!(
        result.is_err(),
        "Expected receive to fail without connection: {:?}",
        result
    );
}

#[tokio::test]
async fn test_webtransport_connection_state() {
    // Test WebTransport connection state tracking
    let mut config = TransportConfig::default();
    config.url = "https://localhost:8080".to_string();
    config.protocols = vec!["webtransport".to_string()];
    config.heartbeat_interval = Some(Duration::from_secs(30));
    config.max_reconnect_attempts = Some(3);
    config.reconnect_delay = Duration::from_secs(5);

    let mut connection = WebTransportConnection::new(config).await.unwrap();

    // Initially should be disconnected
    assert_eq!(connection.state(), ConnectionState::Disconnected);

    // Attempt connection (will fail)
    let _result = connection.connect("https://localhost:8080").await;

    // Should still be disconnected after failed connection
    assert_eq!(connection.state(), ConnectionState::Disconnected);
}

#[tokio::test]
async fn test_webtransport_reconnection() {
    // Test WebTransport reconnection logic
    let mut config = TransportConfig::default();
    config.url = "https://localhost:8080".to_string();
    config.protocols = vec!["webtransport".to_string()];
    config.heartbeat_interval = Some(Duration::from_secs(30));
    config.max_reconnect_attempts = Some(3);
    config.reconnect_delay = Duration::from_secs(5);

    let mut connection = WebTransportConnection::new(config).await.unwrap();

    // Test reconnection attempt
    let result = connection.reconnect().await;
    assert!(
        result.is_err(),
        "Expected reconnection to fail without server: {:?}",
        result
    );

    // Test reconnection with backoff
    let result = connection.reconnect_with_backoff().await;
    assert!(
        result.is_err(),
        "Expected reconnection with backoff to fail without server: {:?}",
        result
    );
}

#[tokio::test]
async fn test_webtransport_performance_optimization() {
    // Test WebTransport performance optimization features
    let mut config = TransportConfig::default();
    config.url = "https://localhost:8080".to_string();
    config.protocols = vec!["webtransport".to_string()];
    config.heartbeat_interval = Some(Duration::from_secs(30));
    config.max_reconnect_attempts = Some(3);
    config.reconnect_delay = Duration::from_secs(5);

    let connection = WebTransportConnection::new(config).await.unwrap();

    // Test performance metrics
    let metrics = connection.get_performance_metrics();
    assert_eq!(metrics.connection_count, 0);
    assert_eq!(metrics.message_count, 0);
    assert_eq!(metrics.error_count, 0);

    // Test optimization settings
    let result = connection.optimize_for_latency().await;
    assert!(
        result.is_ok(),
        "Latency optimization should succeed: {:?}",
        result
    );

    let result = connection.optimize_for_throughput().await;
    assert!(
        result.is_ok(),
        "Throughput optimization should succeed: {:?}",
        result
    );
}
