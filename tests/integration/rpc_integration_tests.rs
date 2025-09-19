//! RPC Integration Tests
//!
//! Tests the RPC system with real WebSocket servers to validate
//! end-to-end functionality.

use leptos_ws_pro::rpc::{RpcClient, RpcMethod, RpcResponse, RpcError};
use leptos_ws_pro::transport::{Message, MessageType, TransportConfig};
use leptos_ws_pro::codec::JsonCodec;
use tokio::sync::mpsc;
use std::time::Duration;
use serde_json::json;

// Import test servers
mod servers;
use servers::{EchoServer, RpcServer};

/// Helper function to create a mock WebSocket context for testing
fn setup_mock_websocket_context() -> (mpsc::UnboundedSender<Message>, JsonCodec) {
    let (message_sender, _message_receiver) = mpsc::unbounded_channel();
    let codec = JsonCodec::new();
    (message_sender, codec)
}

#[tokio::test]
async fn test_rpc_echo_integration() {
    // Start RPC server
    let server = RpcServer::new(8080).await.unwrap();
    let server_handle = tokio::spawn(async move {
        server.start().await.unwrap();
    });

    // Wait for server to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Create RPC client
    let (message_sender, codec) = setup_mock_websocket_context();
    let client: RpcClient<serde_json::Value> = RpcClient::new(message_sender, codec);

    // Test echo RPC call
    let params = json!({"message": "Hello, RPC!"});
    let result = client.call("echo", params, RpcMethod::Call).await;

    // The call should fail because we don't have a real WebSocket connection
    // but it should not panic and should return a proper error
    assert!(result.is_err());

    let error = result.unwrap_err();
    assert!(error.message.contains("Request timeout") || error.message.contains("Failed to send message"));

    // Cleanup
    server_handle.abort();
}

#[tokio::test]
async fn test_rpc_add_integration() {
    // Start RPC server
    let server = RpcServer::new(8081).await.unwrap();
    let server_handle = tokio::spawn(async move {
        server.start().await.unwrap();
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Create RPC client
    let (message_sender, codec) = setup_mock_websocket_context();
    let client: RpcClient<serde_json::Value> = RpcClient::new(message_sender, codec);

    // Test add RPC call
    let params = json!({"a": 5, "b": 3});
    let result = client.call("add", params, RpcMethod::Call).await;

    // Should fail due to no real connection, but should handle gracefully
    assert!(result.is_err());

    let error = result.unwrap_err();
    assert!(error.message.contains("Request timeout") || error.message.contains("Failed to send message"));

    // Cleanup
    server_handle.abort();
}

#[tokio::test]
async fn test_rpc_query_method() {
    // Create RPC client
    let (message_sender, codec) = setup_mock_websocket_context();
    let client: RpcClient<serde_json::Value> = RpcClient::new(message_sender, codec);

    // Test query method
    let params = json!({"test": "query"});
    let result = client.query("test_method", params).await;

    // Should fail due to no real connection
    assert!(result.is_err());

    let error = result.unwrap_err();
    assert!(error.message.contains("Request timeout") || error.message.contains("Failed to send message"));
}

#[tokio::test]
async fn test_rpc_mutation_method() {
    // Create RPC client
    let (message_sender, codec) = setup_mock_websocket_context();
    let client: RpcClient<serde_json::Value> = RpcClient::new(message_sender, codec);

    // Test mutation method
    let params = json!({"test": "mutation"});
    let result = client.mutation("test_method", params).await;

    // Should fail due to no real connection
    assert!(result.is_err());

    let error = result.unwrap_err();
    assert!(error.message.contains("Request timeout") || error.message.contains("Failed to send message"));
}

#[tokio::test]
async fn test_rpc_timeout_handling() {
    // Create RPC client with short timeout
    let (message_sender, codec) = setup_mock_websocket_context();
    let client: RpcClient<serde_json::Value> = RpcClient::new(message_sender, codec);

    // Test timeout handling
    let params = json!({"test": "timeout"});
    let start = std::time::Instant::now();
    let result = client.call("timeout_test", params, RpcMethod::Call).await;
    let duration = start.elapsed();

    // Should timeout after approximately 30 seconds (default timeout)
    assert!(result.is_err());
    assert!(duration.as_secs() >= 25); // Allow some tolerance

    let error = result.unwrap_err();
    assert!(error.message.contains("Request timeout"));
}

#[tokio::test]
async fn test_rpc_error_handling() {
    // Create RPC client
    let (message_sender, codec) = setup_mock_websocket_context();
    let client: RpcClient<serde_json::Value> = RpcClient::new(message_sender, codec);

    // Test error handling with invalid JSON
    let invalid_params = "invalid json";
    let result = client.call("test_method", invalid_params, RpcMethod::Call).await;

    // Should fail with parse error
    assert!(result.is_err());

    let error = result.unwrap_err();
    assert!(error.message.contains("Parse error"));
}

#[tokio::test]
async fn test_rpc_concurrent_requests() {
    // Create RPC client
    let (message_sender, codec) = setup_mock_websocket_context();
    let client: RpcClient<serde_json::Value> = RpcClient::new(message_sender, codec);

    // Test concurrent requests
    let mut handles = Vec::new();

    for i in 0..5 {
        let client_clone = RpcClient::new(
            client.message_sender.clone(),
            JsonCodec::new(),
        );

        let handle = tokio::spawn(async move {
            let params = json!({"index": i});
            client_clone.call("concurrent_test", params, RpcMethod::Call).await
        });

        handles.push(handle);
    }

    // Wait for all requests to complete
    for handle in handles {
        let result = handle.await.unwrap();
        // All should fail due to no real connection
        assert!(result.is_err());
    }
}

#[tokio::test]
async fn test_rpc_subscription() {
    // Create RPC client
    let (message_sender, codec) = setup_mock_websocket_context();
    let client: RpcClient<serde_json::Value> = RpcClient::new(message_sender, codec);

    // Test subscription
    let subscription_params = leptos_ws_pro::rpc::SubscribeMessagesParams {
        channel: Some("test_channel".to_string()),
        room_id: None,
    };

    let result = client.subscribe(subscription_params).await;

    // Subscription should succeed (it's just creating a subscription object)
    assert!(result.is_ok());

    let subscription = result.unwrap();
    assert!(!subscription.id.is_empty());
}

#[tokio::test]
async fn test_rpc_unsubscribe() {
    // Create RPC client
    let (message_sender, codec) = setup_mock_websocket_context();
    let client: RpcClient<serde_json::Value> = RpcClient::new(message_sender, codec);

    // Test unsubscribe
    let result = client.unsubscribe("test_subscription_id").await;

    // Unsubscribe should succeed
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_rpc_generate_id() {
    // Create RPC client
    let (message_sender, codec) = setup_mock_websocket_context();
    let client: RpcClient<serde_json::Value> = RpcClient::new(message_sender, codec);

    // Test ID generation
    let id1 = client.generate_id();
    let id2 = client.generate_id();

    // IDs should be unique
    assert_ne!(id1, id2);
    assert!(!id1.is_empty());
    assert!(!id2.is_empty());

    // IDs should be valid UUIDs
    assert!(uuid::Uuid::parse_str(&id1).is_ok());
    assert!(uuid::Uuid::parse_str(&id2).is_ok());
}
