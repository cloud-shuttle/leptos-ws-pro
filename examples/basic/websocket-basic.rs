//! Basic WebSocket example demonstrating core functionality
//! 
//! This example shows how to use the basic WebSocket functionality
//! with JSON codec for message serialization.

use leptos_ws_pro::{
    JsonCodec, WsMessage, Codec,
    use_websocket, use_connection_status
};
use leptos::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct ChatMessage {
    id: u32,
    content: String,
    sender: String,
    timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct User {
    id: u32,
    name: String,
    online: bool,
}

#[component]
pub fn BasicWebSocketExample() -> impl IntoView {
    // Create WebSocket context
    let ws_context = use_websocket("ws://localhost:8080");
    let connection_status = use_connection_status(&ws_context);
    
    // Test data - create inside closures to avoid move issues

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
            
            <div class="codec-demo">
                <h3>"JSON Codec Demo"</h3>
                <button 
                    on:click=move |_| {
                        // Test JSON codec
                        let test_message = ChatMessage {
                            id: 1,
                            content: "Hello, WebSocket!".to_string(),
                            sender: "User1".to_string(),
                            timestamp: 1234567890,
                        };
                        let codec = JsonCodec::new();
                        let encoded = codec.encode(&test_message).unwrap();
                        let decoded: ChatMessage = codec.decode(&encoded).unwrap();
                        println!("Encoded: {:?}", encoded);
                        println!("Decoded: {:?}", decoded);
                        assert_eq!(test_message, decoded);
                        println!("✅ JSON codec test passed!");
                    }
                >
                    "Test JSON Codec"
                </button>
            </div>
            
            <div class="message-demo">
                <h3>"Message Demo"</h3>
                <button 
                    on:click=move |_| {
                        // Test message wrapper
                        let test_message = ChatMessage {
                            id: 1,
                            content: "Hello, WebSocket!".to_string(),
                            sender: "User1".to_string(),
                            timestamp: 1234567890,
                        };
                        let ws_message = WsMessage::new(test_message.clone());
                        let json_encoded = serde_json::to_string(&ws_message).unwrap();
                        let json_decoded: WsMessage<ChatMessage> = serde_json::from_str(&json_encoded).unwrap();
                        println!("WS Message: {:?}", ws_message);
                        println!("JSON: {}", json_encoded);
                        assert_eq!(ws_message.data, json_decoded.data);
                        println!("✅ Message wrapper test passed!");
                    }
                >
                    "Test Message Wrapper"
                </button>
            </div>
            
            <div class="context-demo">
                <h3>"WebSocket Context Demo"</h3>
                <button 
                    on:click=move |_| {
                        // Test context functionality
                        let ws_context = use_websocket("ws://localhost:8080");
                        println!("Is connected: {}", ws_context.is_connected());
                        println!("Connection state: {:?}", ws_context.connection_state());
                        println!("Heartbeat interval: {:?}", ws_context.heartbeat_interval());
                        println!("Max reconnect attempts: {}", ws_context.max_reconnect_attempts());
                        println!("✅ Context demo completed!");
                    }
                >
                    "Test Context"
                </button>
            </div>
            
            <div class="metrics-demo">
                <h3>"Metrics Demo"</h3>
                <button 
                    on:click=move |_| {
                        // Test metrics
                        let ws_context = use_websocket("ws://localhost:8080");
                        let metrics = ws_context.get_connection_metrics();
                        println!("Connection metrics: {:?}", metrics);
                        println!("Connection quality: {}", ws_context.get_connection_quality());
                        println!("✅ Metrics demo completed!");
                    }
                >
                    "Show Metrics"
                </button>
            </div>
        </div>
    }
}

fn main() {
    leptos::mount::mount_to_body(BasicWebSocketExample)
}
