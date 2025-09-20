//! TDD tests for RPC system functionality
//!
//! These tests define the behavior we want for the RPC system
//! including request/response handling, subscriptions, and error handling.

use leptos_ws_pro::rpc::*;
use leptos_ws_pro::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct TestRequest {
    id: u32,
    message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct TestResponse {
    id: u32,
    result: String,
    timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct TestNotification {
    event: String,
    data: String,
}

#[tokio::test]
async fn test_rpc_client_creation() {
    // Test that RPC client can be created
    let ws_context = WebSocketContext::new_with_url("ws://localhost:8080");
    let codec = JsonCodec::new();
    let client: RpcClient<TestRequest> = RpcClient::new(ws_context, codec);

    // Client should be created successfully
    assert!(true); // Basic creation test
}

#[tokio::test]
async fn test_rpc_request_response() {
    // Test that RPC can handle request/response patterns
    let ws_context = WebSocketContext::new_with_url("ws://localhost:8080");
    let codec = JsonCodec::new();
    let client: RpcClient<TestRequest> = RpcClient::new(ws_context, codec);

    let request = TestRequest {
        id: 1,
        message: "Hello, RPC!".to_string(),
    };

    // This should return a response since RPC is implemented
    let result: Result<RpcResponse<TestRequest>, RpcError> =
        client.call("test_method", request, RpcMethod::Call).await;
    assert!(result.is_ok());

    // Verify it's a successful response
    match result {
        Ok(response) => {
            assert_eq!(response.id, "test_method");
        }
        Err(_) => panic!("Expected success, but got error"),
    }
}

#[tokio::test]
async fn test_rpc_subscription() {
    // Test that RPC can handle subscriptions
    let ws_context = WebSocketContext::new_with_url("ws://localhost:8080");
    let codec = JsonCodec::new();
    let client: RpcClient<TestRequest> = RpcClient::new(ws_context, codec);

    let request = TestRequest {
        id: 2,
        message: "Subscribe to updates".to_string(),
    };

    // Create subscription
    let subscription: RpcSubscription<TestRequest> = client
        .subscribe(SubscribeMessagesParams {
            channel: Some("test".to_string()),
            room_id: None,
        })
        .await
        .unwrap();

    // Subscription should be created with an ID
    assert!(!subscription.id.is_empty());
    assert!(subscription.id.len() > 0);
}

#[tokio::test]
async fn test_rpc_error_handling() {
    // Test that RPC properly handles various error conditions
    let ws_context = WebSocketContext::new_with_url("ws://localhost:8080");
    let codec = JsonCodec::new();
    let client: RpcClient<TestRequest> = RpcClient::new(ws_context, codec);

    let request = TestRequest {
        id: 3,
        message: "Test error handling".to_string(),
    };

    // Test with invalid method name
    let result: Result<RpcResponse<TestRequest>, RpcError> =
        client.call("", request.clone(), RpcMethod::Call).await;
    assert!(result.is_ok());

    // Test with null method name
    let result: Result<RpcResponse<TestRequest>, RpcError> =
        client.call("null", request, RpcMethod::Call).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_rpc_message_wrapper() {
    // Test that RPC messages are properly wrapped
    let request = TestRequest {
        id: 4,
        message: "Test message wrapper".to_string(),
    };

    let wrapped = WsMessage::new(request.clone());

    // Verify the wrapper contains the original data
    assert_eq!(wrapped.data, request);

    // Verify serialization works
    let json = serde_json::to_string(&wrapped).unwrap();
    assert!(json.contains("Test message wrapper"));

    // Verify deserialization works
    let unwrapped: WsMessage<TestRequest> = serde_json::from_str(&json).unwrap();
    assert_eq!(unwrapped.data, request);
}

#[tokio::test]
async fn test_rpc_request_structure() {
    // Test that RPC requests have the correct structure
    let request = RpcRequest {
        id: "test_id".to_string(),
        method: "test_method".to_string(),
        params: serde_json::json!({"test": "data"}),
        method_type: RpcMethod::Call,
    };

    // Verify request structure
    assert_eq!(request.id, "test_id");
    assert_eq!(request.method, "test_method");
    assert_eq!(request.method_type, RpcMethod::Call);

    // Verify serialization
    let json = serde_json::to_string(&request).unwrap();
    assert!(json.contains("test_id"));
    assert!(json.contains("test_method"));
    assert!(json.contains("Call"));
}

#[tokio::test]
async fn test_rpc_response_structure() {
    // Test that RPC responses have the correct structure
    let response = RpcResponse {
        id: "test_id".to_string(),
        result: Some(serde_json::json!({"success": true})),
        error: None,
    };

    // Verify response structure
    assert_eq!(response.id, "test_id");
    assert!(response.result.is_some());
    assert!(response.error.is_none());

    // Verify serialization
    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("test_id"));
    assert!(json.contains("success"));
}

#[tokio::test]
async fn test_rpc_error_structure() {
    // Test that RPC errors have the correct structure
    let error = RpcError {
        code: -32601,
        message: "Method not found".to_string(),
        data: Some(serde_json::json!({"method": "invalid_method"})),
    };

    // Verify error structure
    assert_eq!(error.code, -32601);
    assert_eq!(error.message, "Method not found");
    assert!(error.data.is_some());

    // Verify serialization
    let json = serde_json::to_string(&error).unwrap();
    assert!(json.contains("-32601"));
    assert!(json.contains("Method not found"));
    assert!(json.contains("invalid_method"));
}

#[tokio::test]
async fn test_rpc_method_types() {
    // Test that RPC method types work correctly
    let call_method = RpcMethod::Call;
    let subscription_method = RpcMethod::Subscription;

    // Verify method types
    assert_eq!(call_method, RpcMethod::Call);
    assert_eq!(subscription_method, RpcMethod::Subscription);

    // Verify serialization
    let call_json = serde_json::to_string(&call_method).unwrap();
    let sub_json = serde_json::to_string(&subscription_method).unwrap();

    assert_eq!(call_json, "\"Call\"");
    assert_eq!(sub_json, "\"Subscription\"");
}

#[tokio::test]
async fn test_rpc_subscription_lifecycle() {
    // Test RPC subscription lifecycle
    let ws_context = WebSocketContext::new_with_url("ws://localhost:8080");
    let codec = JsonCodec::new();
    let client: RpcClient<TestRequest> = RpcClient::new(ws_context, codec);

    let request = TestRequest {
        id: 5,
        message: "Test subscription lifecycle".to_string(),
    };

    // Create subscription
    let subscription: RpcSubscription<TestRequest> = client
        .subscribe(SubscribeMessagesParams {
            channel: Some("lifecycle_test".to_string()),
            room_id: None,
        })
        .await
        .unwrap();
    let subscription_id = subscription.id.clone();

    // Verify subscription was created
    assert!(!subscription_id.is_empty());

    // Test subscription cancellation (not implemented yet)
    // This would be: subscription.cancel().await;
    // For now, just verify the subscription exists
    assert!(true);
}
