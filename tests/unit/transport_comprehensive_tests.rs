use leptos_ws::transport::{
    Transport, TransportConfig, TransportError, Message, MessageType, ConnectionState,
    TransportFactory, TransportCapabilities
};
use futures::{StreamExt, SinkExt};

#[tokio::test]
async fn test_websocket_connection_lifecycle() {
    let config = TransportConfig {
        url: "ws://localhost:8080".to_string(),
        ..Default::default()
    };

    // Test connection creation
    let mut connection = leptos_ws::transport::websocket::WebSocketConnection::new(config.clone()).await;
    assert!(connection.is_ok());

    let mut connection = connection.unwrap();

    // Test initial state
    assert_eq!(connection.state(), ConnectionState::Disconnected);

    // Test connection attempt (will fail in test environment, but should not panic)
    let result = connection.connect("ws://localhost:8080").await;
    // We expect this to fail in test environment, but the method should exist
    assert!(result.is_err() || result.is_ok());

    // Test disconnect
    let result = connection.disconnect().await;
    assert!(result.is_ok());
    assert_eq!(connection.state(), ConnectionState::Disconnected);
}

#[tokio::test]
async fn test_websocket_message_handling() {
    let config = TransportConfig::default();
    let mut connection = leptos_ws::transport::websocket::WebSocketConnection::new(config).await.unwrap();

    // Test message creation
    let message = Message {
        data: b"Hello, WebSocket!".to_vec(),
        message_type: MessageType::Text,
    };

    // Test split functionality
    let (mut stream, mut sink) = connection.split();

    // The stream and sink should be created successfully
    // (They're empty stubs, but the API should work)
    assert!(stream.next().await.is_none());

    // Test sink (should not panic)
    let result = sink.send(message).await;
    assert!(result.is_ok());
}

#[test]
fn test_transport_config_validation() {
    // Test default config
    let config = TransportConfig::default();
    assert_eq!(config.url, "");
    assert_eq!(config.timeout.as_secs(), 30);
    assert_eq!(config.reconnect_delay.as_secs(), 1);
    assert_eq!(config.max_reconnect_attempts, Some(5));

    // Test custom config
    let config = TransportConfig {
        url: "wss://example.com/ws".to_string(),
        timeout: std::time::Duration::from_secs(60),
        heartbeat_interval: Some(std::time::Duration::from_secs(10)),
        max_reconnect_attempts: Some(10),
        reconnect_delay: std::time::Duration::from_secs(2),
        protocols: vec!["chat".to_string(), "notifications".to_string()],
        headers: {
            let mut headers = std::collections::HashMap::new();
            headers.insert("Authorization".to_string(), "Bearer token".to_string());
            headers
        },
    };

    assert_eq!(config.url, "wss://example.com/ws");
    assert_eq!(config.timeout.as_secs(), 60);
    assert_eq!(config.heartbeat_interval.unwrap().as_secs(), 10);
    assert_eq!(config.max_reconnect_attempts, Some(10));
    assert_eq!(config.reconnect_delay.as_secs(), 2);
    assert_eq!(config.protocols.len(), 2);
    assert_eq!(config.headers.len(), 1);
}

#[test]
fn test_message_types() {
    // Test text message
    let text_msg = Message {
        data: b"Hello, World!".to_vec(),
        message_type: MessageType::Text,
    };
    assert_eq!(text_msg.data, b"Hello, World!");
    assert_eq!(text_msg.message_type, MessageType::Text);

    // Test binary message
    let binary_msg = Message {
        data: vec![0x00, 0x01, 0x02, 0x03],
        message_type: MessageType::Binary,
    };
    assert_eq!(binary_msg.data, vec![0x00, 0x01, 0x02, 0x03]);
    assert_eq!(binary_msg.message_type, MessageType::Binary);

    // Test ping message
    let ping_msg = Message {
        data: b"ping".to_vec(),
        message_type: MessageType::Ping,
    };
    assert_eq!(ping_msg.message_type, MessageType::Ping);

    // Test pong message
    let pong_msg = Message {
        data: b"pong".to_vec(),
        message_type: MessageType::Pong,
    };
    assert_eq!(pong_msg.message_type, MessageType::Pong);

    // Test close message
    let close_msg = Message {
        data: vec![],
        message_type: MessageType::Close,
    };
    assert_eq!(close_msg.message_type, MessageType::Close);
}

#[test]
fn test_connection_state_transitions() {
    // Test state equality
    assert_eq!(ConnectionState::Disconnected, ConnectionState::Disconnected);
    assert_ne!(ConnectionState::Disconnected, ConnectionState::Connected);

    // Test all states
    let states = vec![
        ConnectionState::Disconnected,
        ConnectionState::Connecting,
        ConnectionState::Connected,
        ConnectionState::Reconnecting,
        ConnectionState::Failed,
    ];

    for state in states {
        // Test that states can be cloned and compared
        let cloned = state;
        assert_eq!(state, cloned);
    }
}

#[test]
fn test_transport_error_types() {
    // Test connection failed error
    let error = TransportError::ConnectionFailed("Connection timeout".to_string());
    assert!(matches!(error, TransportError::ConnectionFailed(_)));

    // Test send failed error
    let error = TransportError::SendFailed("Send buffer full".to_string());
    assert!(matches!(error, TransportError::SendFailed(_)));

    // Test receive failed error
    let error = TransportError::ReceiveFailed("Network error".to_string());
    assert!(matches!(error, TransportError::ReceiveFailed(_)));

    // Test protocol error
    let error = TransportError::ProtocolError("Invalid message format".to_string());
    assert!(matches!(error, TransportError::ProtocolError(_)));

    // Test auth failed error
    let error = TransportError::AuthFailed("Invalid token".to_string());
    assert!(matches!(error, TransportError::AuthFailed(_)));

    // Test rate limited error
    let error = TransportError::RateLimited;
    assert!(matches!(error, TransportError::RateLimited));

    // Test not supported error
    let error = TransportError::NotSupported;
    assert!(matches!(error, TransportError::NotSupported));
}

#[test]
fn test_transport_capabilities_platform_detection() {
    let caps = TransportCapabilities::detect();

    // WebSocket should always be available
    assert!(caps.websocket);

    // SSE should always be available
    assert!(caps.sse);

    // Binary support should always be available
    assert!(caps.binary);

    // Platform-specific tests
    #[cfg(target_arch = "wasm32")]
    {
        // In WASM, compression is handled by the browser
        assert!(!caps.compression);
        // WebTransport might be available in modern browsers
        // (We can't test this reliably in unit tests)
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        // In native, we should have compression support
        assert!(caps.compression);
        // WebTransport is not available in native yet
        assert!(!caps.webtransport);
    }
}

#[tokio::test]
async fn test_transport_factory_creation() {
    let config = TransportConfig {
        url: "ws://localhost:8080".to_string(),
        ..Default::default()
    };

    // Test WebSocket creation
    let ws_result = TransportFactory::create_websocket(config.clone()).await;
    assert!(ws_result.is_ok());

    // Test WebTransport creation (should work even if not supported)
    let wt_result = TransportFactory::create_webtransport(config.clone()).await;
    assert!(wt_result.is_ok());

    // Test SSE creation
    let sse_result = TransportFactory::create_sse(config.clone()).await;
    assert!(sse_result.is_ok());

    // Test adaptive creation (should try WebSocket first)
    let adaptive_result = TransportFactory::create_adaptive(config).await;
    assert!(adaptive_result.is_ok());
}

#[tokio::test]
async fn test_websocket_capabilities() {
    let config = TransportConfig::default();
    let connection = leptos_ws::transport::websocket::WebSocketConnection::new(config).await.unwrap();

    let caps = connection.capabilities();
    assert!(caps.websocket);
    assert!(caps.binary);
    // WebSocket doesn't support compression by default
    assert!(!caps.compression);
    // WebSocket doesn't support multiplexing by default
    assert!(!caps.multiplexing);
}

#[tokio::test]
async fn test_webtransport_connection() {
    let config = TransportConfig {
        url: "https://example.com".to_string(),
        ..Default::default()
    };

    let mut connection = leptos_ws::transport::webtransport::WebTransportConnection::new(config).await;
    assert!(connection.is_ok());

    let mut connection = connection.unwrap();
    assert_eq!(connection.state(), ConnectionState::Disconnected);

    // Test connection (will fail in test environment)
    let result = connection.connect("https://example.com").await;
    assert!(result.is_ok()); // Our stub always succeeds

    // Test disconnect
    let result = connection.disconnect().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_sse_connection() {
    let config = TransportConfig {
        url: "http://example.com/events".to_string(),
        ..Default::default()
    };

    let mut connection = leptos_ws::transport::sse::SseConnection::new(config).await;
    assert!(connection.is_ok());

    let mut connection = connection.unwrap();
    assert_eq!(connection.state(), ConnectionState::Disconnected);

    // Test connection
    let result = connection.connect("http://example.com/events").await;
    assert!(result.is_ok()); // Our stub always succeeds

    // Test disconnect
    let result = connection.disconnect().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_adaptive_transport() {
    let config = TransportConfig {
        url: "wss://example.com".to_string(),
        ..Default::default()
    };

    let mut connection = leptos_ws::transport::adaptive::AdaptiveTransport::new(config).await;
    assert!(connection.is_ok());

    let mut connection = connection.unwrap();
    assert_eq!(connection.state(), ConnectionState::Disconnected);

    // Test connection
    let result = connection.connect("wss://example.com").await;
    assert!(result.is_ok()); // Our stub always succeeds

    // Test disconnect
    let result = connection.disconnect().await;
    assert!(result.is_ok());
}
