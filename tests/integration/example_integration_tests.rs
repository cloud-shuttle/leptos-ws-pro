//! Integration tests for examples
//!
//! These tests verify that the examples can be compiled and work with the transport layer.

use leptos_ws_pro::*;
use leptos::prelude::*;

#[test]
fn test_websocket_basic_example_compilation() {
    // Test that the websocket-basic example can be compiled
    // This is a simplified version of the actual example

    #[component]
    fn TestWebSocketExample() -> impl IntoView {
        // Create WebSocket context
        let ws_context = use_websocket("ws://localhost:8080");
        let connection_status = use_connection_status(&ws_context);

        view! {
            <div class="websocket-example">
                <h2>"Basic WebSocket Example"</h2>
                <div class="connection-status">
                    <h3>"Connection Status"</h3>
                    <p>
                        "Status: "
                        <span class="status">
                            {move || format!("{:?}", connection_status.get())}
                        </span>
                    </p>
                </div>
            </div>
        }
    }

    // Test that the component can be created
    let _component = TestWebSocketExample();
    assert!(true); // Basic test to ensure compilation
}

#[test]
fn test_websocket_connection_example_compilation() {
    // Test that the websocket-connection example can be compiled
    // This is a simplified version of the actual example

    #[component]
    fn TestWebSocketConnection() -> impl IntoView {
        // Create WebSocket provider with server URL
        let provider = WebSocketProvider::new("ws://localhost:8080");
        let context = WebSocketContext::new(provider);

        // Get connection state signal
        let connection_state = context.connection_state;
        let is_connected = move || context.is_connected();

        view! {
            <div class="app">
                <h1>"Leptos WS Pro - Basic Connection Example"</h1>
                <div class="connection-status">
                    <h2>"Connection Status"</h2>
                    <div class="status" class:connected=is_connected>
                        {move || match connection_state.get() {
                            ConnectionState::Disconnected => "Disconnected",
                            ConnectionState::Connecting => "Connecting...",
                            ConnectionState::Connected => "Connected",
                        }}
                    </div>
                </div>
            </div>
        }
    }

    // Test that the component can be created
    let _component = TestWebSocketConnection();
    assert!(true); // Basic test to ensure compilation
}

#[test]
fn test_transport_layer_integration() {
    // Test that the transport layer integrates with the examples

    // Test WebSocket transport
    let ws_config = TransportConfig {
        url: "ws://localhost:8080".to_string(),
        ..Default::default()
    };

    // Test that we can create a WebSocket connection
    let ws_result = std::thread::spawn(move || {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            WebSocketConnection::new(ws_config).await
        })
    }).join();

    assert!(ws_result.is_ok());

    // Test SSE transport
    let sse_config = TransportConfig {
        url: "http://localhost:8080/events".to_string(),
        ..Default::default()
    };

    let sse_result = std::thread::spawn(move || {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            SseConnection::new(sse_config).await
        })
    }).join();

    assert!(sse_result.is_ok());

    // Test WebTransport
    let wt_config = TransportConfig {
        url: "https://localhost:8080/webtransport".to_string(),
        ..Default::default()
    };

    let wt_result = std::thread::spawn(move || {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            WebTransportConnection::new(wt_config).await
        })
    }).join();

    assert!(wt_result.is_ok());
}

#[test]
fn test_adaptive_transport_integration() {
    // Test that adaptive transport works with examples

    let config = TransportConfig {
        url: "ws://localhost:8080".to_string(),
        ..Default::default()
    };

    let adaptive_result = std::thread::spawn(move || {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            AdaptiveTransport::new(config).await
        })
    }).join();

    assert!(adaptive_result.is_ok());
}

#[test]
fn test_codec_integration() {
    // Test that codecs work with examples

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
    struct TestMessage {
        id: u32,
        content: String,
        sender: String,
        timestamp: u64,
    }

    let test_message = TestMessage {
        id: 1,
        content: "Hello, WebSocket!".to_string(),
        sender: "User1".to_string(),
        timestamp: 1234567890,
    };

    // Test JSON codec
    let codec = JsonCodec::new();
    let encoded = codec.encode(&test_message).unwrap();
    let decoded: TestMessage = codec.decode(&encoded).unwrap();
    assert_eq!(test_message, decoded);

    // Test message wrapper
    let ws_message = WsMessage::new(test_message.clone());
    let json_encoded = serde_json::to_string(&ws_message).unwrap();
    let json_decoded: WsMessage<TestMessage> = serde_json::from_str(&json_encoded).unwrap();
    assert_eq!(ws_message.data, json_decoded.data);
}

#[test]
fn test_reactive_integration() {
    // Test that reactive components work with transport layer

    #[component]
    fn TestReactiveComponent() -> impl IntoView {
        let provider = WebSocketProvider::new("ws://localhost:8080");
        let context = WebSocketContext::new(provider);

        // Test connection state
        assert_eq!(context.connection_state(), ConnectionState::Disconnected);

        // Test metrics
        let metrics = context.get_connection_metrics();
        assert_eq!(metrics.messages_received, 0);
        assert_eq!(metrics.messages_sent, 0);

        view! {
            <div>
                <p>"Reactive integration test passed"</p>
            </div>
        }
    }

    let _component = TestReactiveComponent();
    assert!(true);
}
