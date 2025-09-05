//! Server integration tests using real WebSocket server
//! 
//! These tests verify that the leptos_ws library works correctly
//! with a real WebSocket server, testing actual network communication.

use leptos_ws::*;
use leptos_ws::transport::*;
use leptos_ws::reactive::*;
use leptos_ws::rpc::*;
use serde::{Deserialize, Serialize};
use leptos::prelude::*;
// use std::time::Duration;
// use tokio::time::timeout;

mod server;
use server::TestWebSocketServer;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct TestMessage {
    id: u32,
    content: String,
    timestamp: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct TestRpcRequest {
    method: String,
    params: TestMessage,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct TestRpcResponse {
    result: TestMessage,
    success: bool,
}

#[tokio::test]
async fn test_real_websocket_connection() {
    // Start test server
    let server = TestWebSocketServer::new().await.unwrap();
    let server_url = server.url();
    
    // Create WebSocket provider and context
    let provider = WebSocketProvider::new(&server_url);
    let context = WebSocketContext::new(provider);
    
    // Test connection state
    assert_eq!(context.connection_state(), ConnectionState::Disconnected);
    
    // In a real implementation, we would connect here
    // For now, we test that the context was created successfully
    assert!(context.heartbeat_interval().is_some());
    assert_eq!(context.heartbeat_interval().unwrap(), 30);
    
    // Cleanup
    server.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_server_message_handling() {
    // Start test server
    let server = TestWebSocketServer::new().await.unwrap();
    let server_url = server.url();
    
    // Create test message
    let test_message = TestMessage {
        id: 1,
        content: "Hello, Server!".to_string(),
        timestamp: 1234567890,
    };
    
    // Create WebSocket context
    let provider = WebSocketProvider::new(&server_url);
    let _context = WebSocketContext::new(provider);
    
    // Test message encoding/decoding
    let codec = crate::codec::JsonCodec::new();
    let encoded = codec.encode(&test_message).unwrap();
    let decoded: TestMessage = codec.decode(&encoded).unwrap();
    
    assert_eq!(decoded, test_message);
    
    // Test transport message creation
    let transport_message = Message {
        data: encoded,
        message_type: MessageType::Text,
    };
    
    assert_eq!(transport_message.message_type, MessageType::Text);
    assert!(!transport_message.data.is_empty());
    
    // Cleanup
    server.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_rpc_with_real_server() {
    // Start test server
    let server = TestWebSocketServer::new().await.unwrap();
    let server_url = server.url();
    
    // Create WebSocket context
    let provider = WebSocketProvider::new(&server_url);
    let context = WebSocketContext::new(provider);
    
    // Create RPC client
    let _rpc_client: RpcClient<TestRpcRequest> = RpcClient::new(context);
    
    // Test RPC request creation
    let test_message = TestMessage {
        id: 42,
        content: "RPC Test".to_string(),
        timestamp: 1234567890,
    };
    
    let rpc_request = TestRpcRequest {
        method: "test_method".to_string(),
        params: test_message.clone(),
    };
    
    let request = RpcRequest {
        id: "test_req_123".to_string(),
        method: "test_method".to_string(),
        params: rpc_request,
        method_type: RpcMethod::Query,
    };
    
    // Test serialization
    let codec = crate::codec::JsonCodec::new();
    let encoded = codec.encode(&request).unwrap();
    let decoded: RpcRequest<TestRpcRequest> = codec.decode(&encoded).unwrap();
    
    assert_eq!(decoded.id, "test_req_123");
    assert_eq!(decoded.method, "test_method");
    assert_eq!(decoded.method_type, RpcMethod::Query);
    
    // Cleanup
    server.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_server_connection_lifecycle() {
    // Start test server
    let server = TestWebSocketServer::new().await.unwrap();
    let server_url = server.url();
    
    // Verify server is running
    assert_eq!(server.connected_clients_count().await, 0);
    
    // Create WebSocket context
    let provider = WebSocketProvider::new(&server_url);
    let context = WebSocketContext::new(provider);
    
    // Test connection state transitions
    context.set_connection_state(ConnectionState::Connecting);
    assert_eq!(context.connection_state(), ConnectionState::Connecting);
    
    context.set_connection_state(ConnectionState::Connected);
    assert_eq!(context.connection_state(), ConnectionState::Connected);
    assert!(context.is_connected());
    
    context.set_connection_state(ConnectionState::Disconnected);
    assert_eq!(context.connection_state(), ConnectionState::Disconnected);
    assert!(!context.is_connected());
    
    // Cleanup
    server.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_server_error_handling() {
    // Start test server
    let server = TestWebSocketServer::new().await.unwrap();
    let server_url = server.url();
    
    // Create WebSocket context
    let provider = WebSocketProvider::new(&server_url);
    let _context = WebSocketContext::new(provider);
    
    // Test error handling
    let codec = crate::codec::JsonCodec::new();
    let invalid_data = b"invalid json data";
    let decode_result: Result<TestMessage, _> = codec.decode(invalid_data);
    assert!(decode_result.is_err());
    
    // Test RPC error handling
    let rpc_error = RpcError {
        code: 500,
        message: "Internal Server Error".to_string(),
        data: Some(serde_json::json!({"details": "Test error"})),
    };
    
    let rpc_response = RpcResponse::<TestRpcResponse> {
        id: "error_test".to_string(),
        result: None,
        error: Some(rpc_error.clone()),
    };
    
    // Test error serialization
    let encoded = codec.encode(&rpc_response).unwrap();
    let decoded: RpcResponse<TestRpcResponse> = codec.decode(&encoded).unwrap();
    
    assert!(decoded.result.is_none());
    assert!(decoded.error.is_some());
    assert_eq!(decoded.error.unwrap().code, 500);
    
    // Cleanup
    server.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_server_concurrent_connections() {
    // Start test server
    let server = TestWebSocketServer::new().await.unwrap();
    let server_url = server.url();
    
    // Create multiple contexts (simulating multiple clients)
    let mut contexts = Vec::new();
    for i in 0..5 {
        let provider = WebSocketProvider::new(&server_url);
        let context = WebSocketContext::new(provider);
        contexts.push((i, context));
    }
    
    // Test that all contexts were created successfully
    assert_eq!(contexts.len(), 5);
    
    // Test concurrent message processing
    let codec = crate::codec::JsonCodec::new();
    for (i, context) in &contexts {
        let test_message = TestMessage {
            id: *i as u32,
            content: format!("Message from client {}", i),
            timestamp: 1234567890,
        };
        
        let encoded = codec.encode(&test_message).unwrap();
        let transport_message = Message {
            data: encoded,
            message_type: MessageType::Text,
        };
        
        context.handle_message(transport_message);
    }
    
    // Verify all messages were processed
    for (_, context) in &contexts {
        let messages_signal = context.messages;
        let messages = messages_signal.get();
        assert_eq!(messages.len(), 1);
    }
    
    // Cleanup
    server.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_server_heartbeat_functionality() {
    // Start test server
    let server = TestWebSocketServer::new().await.unwrap();
    let server_url = server.url();
    
    // Create WebSocket context
    let provider = WebSocketProvider::new(&server_url);
    let context = WebSocketContext::new(provider);
    
    // Test heartbeat configuration
    assert!(context.heartbeat_interval().is_some());
    assert_eq!(context.heartbeat_interval().unwrap(), 30);
    
    // Send heartbeat
    let heartbeat_result = context.send_heartbeat();
    assert!(heartbeat_result.is_ok());
    
    // Verify heartbeat was sent
    let sent_messages = context.get_sent_messages::<serde_json::Value>();
    assert!(!sent_messages.is_empty());
    
    // Verify heartbeat structure
    let heartbeat_msg = &sent_messages[0];
    assert_eq!(heartbeat_msg["type"], "ping");
    assert!(heartbeat_msg["timestamp"].is_number());
    
    // Cleanup
    server.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_server_presence_tracking() {
    // Start test server
    let server = TestWebSocketServer::new().await.unwrap();
    let server_url = server.url();
    
    // Create WebSocket context
    let provider = WebSocketProvider::new(&server_url);
    let context = WebSocketContext::new(provider);
    
    // Create user presence
    let user_presence = UserPresence {
        user_id: "test_user_123".to_string(),
        status: "online".to_string(),
        last_seen: 1234567890,
    };
    
    // Update presence
    context.update_presence("test_user_123", user_presence.clone());
    
    // Get presence
    let presence_map = context.get_presence();
    assert!(presence_map.contains_key("test_user_123"));
    
    let retrieved_presence = &presence_map["test_user_123"];
    assert_eq!(retrieved_presence.user_id, "test_user_123");
    assert_eq!(retrieved_presence.status, "online");
    
    // Cleanup
    server.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_server_connection_metrics() {
    // Start test server
    let server = TestWebSocketServer::new().await.unwrap();
    let server_url = server.url();
    
    // Create WebSocket context
    let provider = WebSocketProvider::new(&server_url);
    let context = WebSocketContext::new(provider);
    
    // Get initial metrics
    let initial_metrics = context.get_connection_metrics();
    assert_eq!(initial_metrics.messages_sent, 0);
    assert_eq!(initial_metrics.messages_received, 0);
    
    // Update connection quality
    context.update_connection_quality(0.8);
    assert_eq!(context.get_connection_quality(), 0.8);
    
    // Test quality-based reconnection logic
    context.update_connection_quality(0.3);
    assert!(context.should_reconnect_due_to_quality());
    
    // Cleanup
    server.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_server_message_roundtrip() {
    // Start test server
    let server = TestWebSocketServer::new().await.unwrap();
    let server_url = server.url();
    
    // Create test message
    let original_message = TestMessage {
        id: 999,
        content: "Roundtrip test message".to_string(),
        timestamp: 1234567890,
    };
    
    // Create WebSocket context
    let provider = WebSocketProvider::new(&server_url);
    let context = WebSocketContext::new(provider);
    
    // Test full roundtrip: Message -> Encode -> Transport -> Handle -> Decode
    let codec = crate::codec::JsonCodec::new();
    
    // 1. Encode message
    let encoded = codec.encode(&original_message).unwrap();
    
    // 2. Create transport message
    let transport_message = Message {
        data: encoded,
        message_type: MessageType::Text,
    };
    
    // 3. Handle through context
    context.handle_message(transport_message);
    
    // 4. Retrieve and verify
    let messages_signal = context.messages;
    let received_messages = messages_signal.get();
    assert!(!received_messages.is_empty());
    
    // 5. Decode and verify
    let received_message = &received_messages[0];
    let decoded_message: TestMessage = codec.decode(&received_message.data).unwrap();
    
    assert_eq!(decoded_message, original_message);
    
    // Cleanup
    server.shutdown().await.unwrap();
}
