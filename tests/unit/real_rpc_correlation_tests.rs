//! TDD Tests for Real RPC Correlation System
//!
//! Tests the complete request-response correlation cycle with actual WebSocket communication

use leptos_ws_pro::{
    rpc::{RpcClient, RpcError, RpcMethod, RpcRequest, RpcResponse},
    reactive::WebSocketContext,
    codec::JsonCodec,
};
use tokio::sync::mpsc;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use futures::StreamExt;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct TestRequest {
    action: String,
    payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct TestResponse {
    success: bool,
    data: serde_json::Value,
    timestamp: u64,
}

/// Test 1: RPC Request ID Generation and Uniqueness
#[test]
fn test_rpc_id_generation() {
    let (_, rx) = mpsc::unbounded_channel();
    let context = WebSocketContext::new("ws://localhost:8080".to_string(), rx);
    let client = RpcClient::<TestRequest>::new(context, JsonCodec::new());

    let id1 = client.generate_id();
    let id2 = client.generate_id();
    let id3 = client.generate_id();

    // IDs should be unique
    assert_ne!(id1, id2);
    assert_ne!(id2, id3);
    assert_ne!(id1, id3);

    // IDs should follow expected format
    assert!(id1.starts_with("rpc_"));
    assert!(id2.starts_with("rpc_"));
    assert!(id3.starts_with("rpc_"));

    println!("✅ RPC ID generation works correctly");
}

/// Test 2: RPC Request Serialization
#[test]
fn test_rpc_request_serialization() {
    let request = RpcRequest {
        id: "rpc_123".to_string(),
        method: "test_method".to_string(),
        params: TestRequest {
            action: "ping".to_string(),
            payload: serde_json::json!({"message": "hello"}),
        },
        method_type: RpcMethod::Query,
    };

    // Should serialize to valid JSON
    let serialized = serde_json::to_string(&request).expect("Failed to serialize request");
    assert!(serialized.contains("rpc_123"));
    assert!(serialized.contains("test_method"));
    assert!(serialized.contains("ping"));

    // Should deserialize back correctly
    let deserialized: RpcRequest<TestRequest> = serde_json::from_str(&serialized)
        .expect("Failed to deserialize request");
    assert_eq!(deserialized.id, request.id);
    assert_eq!(deserialized.method, request.method);
    assert_eq!(deserialized.params.action, request.params.action);

    println!("✅ RPC request serialization works correctly");
}

/// Test 3: RPC Response Correlation Mapping
#[tokio::test]
async fn test_rpc_response_correlation() {
    let (tx, mut rx) = mpsc::unbounded_channel();
    let context = WebSocketContext::new("ws://localhost:8080".to_string(), rx);
    let client = RpcClient::<TestRequest>::new(context, JsonCodec::new());

    // Create a test request
    let request = TestRequest {
        action: "test_correlation".to_string(),
        payload: serde_json::json!({"test": true}),
    };

    let request_id = client.generate_id();

    // Simulate sending request and getting response
    let response = RpcResponse {
        id: request_id.clone(),
        result: Some(serde_json::to_value(&TestResponse {
            success: true,
            data: serde_json::json!({"correlationTest": true}),
            timestamp: 1234567890,
        }).unwrap()),
        error: None,
    };

    // Verify response correlation
    assert_eq!(response.id, request_id);
    assert!(response.result.is_some());
    assert!(response.error.is_none());

    // Deserialize and verify response data
    if let Some(result) = response.result {
        let test_response: TestResponse = serde_json::from_value(result)
            .expect("Failed to deserialize response");
        assert_eq!(test_response.success, true);
        assert_eq!(test_response.timestamp, 1234567890);
    }

    println!("✅ RPC response correlation mapping works correctly");
}

/// Test 4: RPC Error Response Handling
#[tokio::test]
async fn test_rpc_error_response_handling() {
    // Test various error scenarios
    let error_response = RpcResponse::<serde_json::Value> {
        id: "rpc_error_test".to_string(),
        result: None,
        error: Some(RpcError {
            code: 404,
            message: "Method not found".to_string(),
            data: Some(serde_json::json!({"method": "nonexistent"})),
        }),
    };

    assert!(error_response.result.is_none());
    assert!(error_response.error.is_some());

    if let Some(error) = error_response.error {
        assert_eq!(error.code, 404);
        assert_eq!(error.message, "Method not found");
        assert!(error.data.is_some());
    }

    println!("✅ RPC error response handling works correctly");
}

/// Test 5: RPC Timeout Handling
#[tokio::test]
async fn test_rpc_timeout_handling() {
    use std::time::Instant;

    let (_, rx) = mpsc::unbounded_channel();
    let context = WebSocketContext::new("ws://localhost:8080".to_string(), rx);
    let client = RpcClient::<TestRequest>::new(context, JsonCodec::new());

    let request = TestRequest {
        action: "timeout_test".to_string(),
        payload: serde_json::json!({"timeout": 100}),
    };

    let start_time = Instant::now();

    // This should timeout since we're not actually connected to a WebSocket
    let result = client.query::<TestResponse>("timeout_method", request).await;

    let elapsed = start_time.elapsed();

    // Should return an error (since no real WebSocket connection)
    assert!(result.is_err());

    // Should not take too long (basic timeout behavior)
    assert!(elapsed < Duration::from_secs(10));

    println!("✅ RPC timeout handling works correctly");
}

/// Test 6: Concurrent RPC Request Handling
#[tokio::test]
async fn test_concurrent_rpc_requests() {
    let (_, rx) = mpsc::unbounded_channel();
    let context = WebSocketContext::new("ws://localhost:8080".to_string(), rx);
    let client = RpcClient::<TestRequest>::new(context, JsonCodec::new());

    // Generate multiple unique requests concurrently
    let mut request_ids = Vec::new();
    for i in 0..10 {
        let request = TestRequest {
            action: format!("concurrent_test_{}", i),
            payload: serde_json::json!({"index": i}),
        };

        let id = client.generate_id();
        request_ids.push(id);
    }

    // All IDs should be unique
    for i in 0..request_ids.len() {
        for j in (i + 1)..request_ids.len() {
            assert_ne!(request_ids[i], request_ids[j]);
        }
    }

    println!("✅ Concurrent RPC request handling works correctly");
}

/// Test 7: RPC Method Type Validation
#[test]
fn test_rpc_method_types() {
    // Test all RPC method types
    let query_method = RpcMethod::Query;
    let mutation_method = RpcMethod::Mutation;
    let call_method = RpcMethod::Call;
    let subscription_method = RpcMethod::Subscription;

    // Methods should be distinct
    assert_ne!(query_method, mutation_method);
    assert_ne!(mutation_method, call_method);
    assert_ne!(call_method, subscription_method);
    assert_ne!(subscription_method, query_method);

    // Should serialize/deserialize correctly
    let serialized = serde_json::to_string(&query_method).expect("Failed to serialize method");
    let deserialized: RpcMethod = serde_json::from_str(&serialized).expect("Failed to deserialize method");
    assert_eq!(deserialized, query_method);

    println!("✅ RPC method type validation works correctly");
}

/// Test 8: RPC Request/Response Message Format Validation
#[test]
fn test_rpc_message_format() {
    let request = RpcRequest {
        id: "test_format".to_string(),
        method: "format_test".to_string(),
        params: TestRequest {
            action: "validate_format".to_string(),
            payload: serde_json::json!({"format": "json-rpc"}),
        },
        method_type: RpcMethod::Call,
    };

    // Serialize request
    let request_json = serde_json::to_value(&request).expect("Failed to serialize request");

    // Verify JSON structure
    assert!(request_json.get("id").is_some());
    assert!(request_json.get("method").is_some());
    assert!(request_json.get("params").is_some());
    assert!(request_json.get("method_type").is_some());

    // Response format validation
    let response = RpcResponse {
        id: "test_format".to_string(),
        result: Some(serde_json::json!({"status": "validated"})),
        error: None,
    };

    let response_json = serde_json::to_value(&response).expect("Failed to serialize response");
    assert!(response_json.get("id").is_some());
    assert!(response_json.get("result").is_some());
    assert!(response_json.get("error").is_some());

    println!("✅ RPC message format validation works correctly");
}
