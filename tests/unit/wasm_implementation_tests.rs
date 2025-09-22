//! WASM WebSocket implementation tests
//!
//! These tests verify the WASM WebSocket functionality for browser environments.
//! Note: These tests are designed to work on both WASM and non-WASM targets,
//! with different behavior expected based on the target architecture.

use leptos_ws_pro::transport::{
    websocket::WasmWebSocketConnection, ConnectionState, Message, MessageType, Transport,
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

#[tokio::test]
async fn test_wasm_websocket_creation() {
    // Given: A transport configuration
    let config = TransportConfig {
        url: "ws://localhost:8080".to_string(),
        ..Default::default()
    };

    // When: Creating a WASM WebSocket connection
    let result = WasmWebSocketConnection::new(config).await;

    // Then: Should succeed on WASM targets, fail on non-WASM targets
    #[cfg(target_arch = "wasm32")]
    {
        assert!(result.is_ok());
        let client = result.unwrap();
        assert_eq!(client.state(), ConnectionState::Disconnected);
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(matches!(error, TransportError::NotSupported(_)));
    }
}

#[tokio::test]
async fn test_wasm_websocket_capabilities() {
    // Given: A WASM WebSocket connection
    let config = TransportConfig {
        url: "ws://localhost:8080".to_string(),
        ..Default::default()
    };

    // When: Getting capabilities
    let result = WasmWebSocketConnection::new(config).await;

    // Then: Should return appropriate capabilities based on target
    #[cfg(target_arch = "wasm32")]
    {
        let client = result.unwrap();
        let capabilities = client.capabilities();
        assert!(capabilities.websocket);
        assert!(capabilities.binary);
        assert!(!capabilities.webtransport);
        assert!(!capabilities.sse);
        assert!(!capabilities.compression);
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let client = result.unwrap_err();
        // On non-WASM targets, we expect a NotSupported error
        assert!(matches!(client, TransportError::NotSupported(_)));
    }
}

#[tokio::test]
async fn test_wasm_websocket_connection() {
    // Given: A WASM WebSocket client
    let config = TransportConfig {
        url: "ws://localhost:8080".to_string(),
        ..Default::default()
    };

    // When: Attempting to connect
    let result = WasmWebSocketConnection::new(config).await;

    #[cfg(target_arch = "wasm32")]
    {
        let mut client = result.unwrap();

        // Note: In a real browser environment, this would connect to an actual WebSocket server
        // For testing purposes, we'll test the connection logic without a real server
        let connect_result = client.connect("ws://localhost:8080").await;

        // The connection might fail due to no server, but the logic should work
        match connect_result {
            Ok(_) => {
                assert_eq!(client.state(), ConnectionState::Connected);
            }
            Err(e) => {
                // Connection failed, but should be a connection error, not a not-supported error
                assert!(!matches!(e, TransportError::NotSupported(_)));
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut client = result.unwrap_err();
        // On non-WASM targets, connection should fail with NotSupported
        let connect_result = client.connect("ws://localhost:8080").await;
        assert!(connect_result.is_err());
        let error = connect_result.unwrap_err();
        assert!(matches!(error, TransportError::NotSupported(_)));
    }
}

#[tokio::test]
async fn test_wasm_websocket_connection_timeout() {
    // Given: A WASM WebSocket client configured to connect to a non-existent server
    let config = TransportConfig {
        url: "ws://127.0.0.1:99999".to_string(), // Non-existent port
        ..Default::default()
    };

    // When: Client attempts to connect with a timeout
    let result = WasmWebSocketConnection::new(config).await;

    #[cfg(target_arch = "wasm32")]
    {
        let mut client = result.unwrap();
        let result = timeout(
            Duration::from_secs(5),
            client.connect("ws://127.0.0.1:99999"),
        )
        .await;

        // Then: Should fail with connection error or timeout
        assert!(result.is_ok()); // Timeout completed
        let connect_result = result.unwrap();
        assert!(connect_result.is_err());
        // Connection should fail with any error type except NotSupported
        let error = connect_result.unwrap_err();
        assert!(!matches!(error, TransportError::NotSupported(_)));
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut client = result.unwrap_err();
        let result = timeout(
            Duration::from_secs(5),
            client.connect("ws://127.0.0.1:99999"),
        )
        .await;

        // Then: Should fail with NotSupported error
        assert!(result.is_ok()); // Timeout completed
        let connect_result = result.unwrap();
        assert!(connect_result.is_err());
        let error = connect_result.unwrap_err();
        assert!(matches!(error, TransportError::NotSupported(_)));
    }
}

#[tokio::test]
async fn test_wasm_websocket_message_sending() {
    // Given: A connected WASM WebSocket client
    let config = TransportConfig {
        url: "ws://localhost:8080".to_string(),
        ..Default::default()
    };

    let result = WasmWebSocketConnection::new(config).await;

    #[cfg(target_arch = "wasm32")]
    {
        let client = result.unwrap();

        // Create a test message
        let message = Message {
            data: b"Hello, WASM WebSocket!".to_vec(),
            message_type: MessageType::Text,
        };

        // When: Sending a message
        let send_result = client.send_message(&message).await;

        // Then: Should fail with NotConnected since we haven't connected
        assert!(send_result.is_err());
        let error = send_result.unwrap_err();
        assert!(matches!(error, TransportError::NotConnected));
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let client = result.unwrap_err();

        // Create a test message
        let message = Message {
            data: b"Hello, WASM WebSocket!".to_vec(),
            message_type: MessageType::Text,
        };

        // When: Sending a message
        let send_result = client.send_message(&message).await;

        // Then: Should fail with NotSupported
        assert!(send_result.is_err());
        let error = send_result.unwrap_err();
        assert!(matches!(error, TransportError::NotSupported(_)));
    }
}

#[tokio::test]
async fn test_wasm_websocket_binary_message() {
    // Given: A WASM WebSocket client
    let config = TransportConfig {
        url: "ws://localhost:8080".to_string(),
        ..Default::default()
    };

    let result = WasmWebSocketConnection::new(config).await;

    #[cfg(target_arch = "wasm32")]
    {
        let client = result.unwrap();

        // Create a binary test message
        let binary_data = vec![0x00, 0x01, 0x02, 0x03, 0xFF, 0xFE, 0xFD];
        let message = Message {
            data: binary_data,
            message_type: MessageType::Binary,
        };

        // When: Sending a binary message
        let send_result = client.send_message(&message).await;

        // Then: Should fail with NotConnected since we haven't connected
        assert!(send_result.is_err());
        let error = send_result.unwrap_err();
        assert!(matches!(error, TransportError::NotConnected));
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let client = result.unwrap_err();

        // Create a binary test message
        let binary_data = vec![0x00, 0x01, 0x02, 0x03, 0xFF, 0xFE, 0xFD];
        let message = Message {
            data: binary_data,
            message_type: MessageType::Binary,
        };

        // When: Sending a binary message
        let send_result = client.send_message(&message).await;

        // Then: Should fail with NotSupported
        assert!(send_result.is_err());
        let error = send_result.unwrap_err();
        assert!(matches!(error, TransportError::NotSupported(_)));
    }
}

#[tokio::test]
async fn test_wasm_websocket_serialized_message() {
    // Given: A WASM WebSocket client
    let config = TransportConfig {
        url: "ws://localhost:8080".to_string(),
        ..Default::default()
    };

    let result = WasmWebSocketConnection::new(config).await;

    #[cfg(target_arch = "wasm32")]
    {
        let client = result.unwrap();

        // Create a serialized test message
        let test_msg = TestMessage {
            id: 42,
            content: "WASM test message".to_string(),
            timestamp: 1234567890,
        };
        let json_data = serde_json::to_string(&test_msg).unwrap();
        let message = Message {
            data: json_data.into_bytes(),
            message_type: MessageType::Text,
        };

        // When: Sending a serialized message
        let send_result = client.send_message(&message).await;

        // Then: Should fail with NotConnected since we haven't connected
        assert!(send_result.is_err());
        let error = send_result.unwrap_err();
        assert!(matches!(error, TransportError::NotConnected));
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let client = result.unwrap_err();

        // Create a serialized test message
        let test_msg = TestMessage {
            id: 42,
            content: "WASM test message".to_string(),
            timestamp: 1234567890,
        };
        let json_data = serde_json::to_string(&test_msg).unwrap();
        let message = Message {
            data: json_data.into_bytes(),
            message_type: MessageType::Text,
        };

        // When: Sending a serialized message
        let send_result = client.send_message(&message).await;

        // Then: Should fail with NotSupported
        assert!(send_result.is_err());
        let error = send_result.unwrap_err();
        assert!(matches!(error, TransportError::NotSupported(_)));
    }
}

#[tokio::test]
async fn test_wasm_websocket_split() {
    // Given: A WASM WebSocket client
    let config = TransportConfig {
        url: "ws://localhost:8080".to_string(),
        ..Default::default()
    };

    let result = WasmWebSocketConnection::new(config).await;

    #[cfg(target_arch = "wasm32")]
    {
        let client = result.unwrap();

        // When: Splitting the connection
        let (stream, sink) = client.split();

        // Then: Should return valid stream and sink
        assert!(stream.is_some());
        assert!(sink.is_some());
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let client = result.unwrap_err();

        // When: Splitting the connection
        let (stream, sink) = client.split();

        // Then: Should return empty stream and sink for non-WASM targets
        assert!(stream.is_some());
        assert!(sink.is_some());
    }
}

#[tokio::test]
async fn test_wasm_websocket_disconnect() {
    // Given: A WASM WebSocket client
    let config = TransportConfig {
        url: "ws://localhost:8080".to_string(),
        ..Default::default()
    };

    let result = WasmWebSocketConnection::new(config).await;

    #[cfg(target_arch = "wasm32")]
    {
        let mut client = result.unwrap();

        // When: Disconnecting
        let disconnect_result = client.disconnect().await;

        // Then: Should succeed
        assert!(disconnect_result.is_ok());
        assert_eq!(client.state(), ConnectionState::Disconnected);
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut client = result.unwrap_err();

        // When: Disconnecting
        let disconnect_result = client.disconnect().await;

        // Then: Should fail with NotSupported
        assert!(disconnect_result.is_err());
        let error = disconnect_result.unwrap_err();
        assert!(matches!(error, TransportError::NotSupported(_)));
    }
}

#[tokio::test]
async fn test_wasm_websocket_bidirectional_stream() {
    // Given: A WASM WebSocket client
    let config = TransportConfig {
        url: "ws://localhost:8080".to_string(),
        ..Default::default()
    };

    let result = WasmWebSocketConnection::new(config).await;

    #[cfg(target_arch = "wasm32")]
    {
        let mut client = result.unwrap();

        // When: Creating a bidirectional stream
        let stream_result = client.create_bidirectional_stream().await;

        // Then: Should succeed (no-op for WASM WebSocket)
        assert!(stream_result.is_ok());
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut client = result.unwrap_err();

        // When: Creating a bidirectional stream
        let stream_result = client.create_bidirectional_stream().await;

        // Then: Should fail with NotSupported
        assert!(stream_result.is_err());
        let error = stream_result.unwrap_err();
        assert!(matches!(error, TransportError::NotSupported(_)));
    }
}

#[tokio::test]
async fn test_wasm_websocket_platform_detection() {
    // Given: A WASM WebSocket client
    let config = TransportConfig {
        url: "ws://localhost:8080".to_string(),
        ..Default::default()
    };

    // When: Creating the client
    let result = WasmWebSocketConnection::new(config).await;

    // Then: Should behave differently based on target architecture
    #[cfg(target_arch = "wasm32")]
    {
        assert!(result.is_ok(), "WASM WebSocket should be available on wasm32 targets");
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        assert!(result.is_err(), "WASM WebSocket should not be available on non-wasm32 targets");
        let error = result.unwrap_err();
        assert!(matches!(error, TransportError::NotSupported(_)));
    }
}
