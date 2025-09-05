//! Comprehensive tests for the RPC module
//! 
//! These tests follow TDD principles and cover all aspects of the RPC system:
//! - Request/Response handling
//! - Type safety
//! - Error handling
//! - Service definitions
//! - Client functionality
//! - Subscription handling

use leptos_ws::rpc::*;
use leptos_ws::reactive::{WebSocketProvider, WebSocketContext};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct TestParams {
    value: String,
    count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct TestResult {
    message: String,
    processed_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct TestError {
    code: i32,
    message: String,
}

#[test]
fn test_rpc_method_enum() {
    let query = RpcMethod::Query;
    let mutation = RpcMethod::Mutation;
    let subscription = RpcMethod::Subscription;
    
    assert_eq!(query, RpcMethod::Query);
    assert_eq!(mutation, RpcMethod::Mutation);
    assert_eq!(subscription, RpcMethod::Subscription);
}

#[test]
fn test_rpc_request_creation() {
    let params = TestParams {
        value: "test".to_string(),
        count: 42,
    };
    
    let request = RpcRequest {
        id: "req_123".to_string(),
        method: "test_method".to_string(),
        params,
        method_type: RpcMethod::Query,
    };
    
    assert_eq!(request.id, "req_123");
    assert_eq!(request.method, "test_method");
    assert_eq!(request.params.value, "test");
    assert_eq!(request.params.count, 42);
    assert_eq!(request.method_type, RpcMethod::Query);
}

#[test]
fn test_rpc_response_creation() {
    let result = TestResult {
        message: "success".to_string(),
        processed_count: 1,
    };
    
    let response = RpcResponse {
        id: "req_123".to_string(),
        result: Some(result.clone()),
        error: None,
    };
    
    assert_eq!(response.id, "req_123");
    assert_eq!(response.result, Some(result));
    assert!(response.error.is_none());
}

#[test]
fn test_rpc_response_with_error() {
    let error = RpcError {
        code: 404,
        message: "Not found".to_string(),
        data: None,
    };
    
    let response = RpcResponse::<TestResult> {
        id: "req_123".to_string(),
        result: None,
        error: Some(error.clone()),
    };
    
    assert_eq!(response.id, "req_123");
    assert!(response.result.is_none());
    assert_eq!(response.error, Some(error));
}

#[test]
fn test_rpc_error_creation() {
    let error = RpcError {
        code: 500,
        message: "Internal server error".to_string(),
        data: Some(serde_json::json!({"details": "Database connection failed"})),
    };
    
    assert_eq!(error.code, 500);
    assert_eq!(error.message, "Internal server error");
    assert!(error.data.is_some());
}

#[test]
fn test_rpc_request_serialization() {
    let params = TestParams {
        value: "serialize_test".to_string(),
        count: 100,
    };
    
    let request = RpcRequest {
        id: "serialize_123".to_string(),
        method: "serialize_method".to_string(),
        params,
        method_type: RpcMethod::Mutation,
    };
    
    // Test JSON serialization
    let json = serde_json::to_string(&request).expect("Should serialize to JSON");
    let deserialized: RpcRequest<TestParams> = serde_json::from_str(&json).expect("Should deserialize from JSON");
    
    assert_eq!(deserialized.id, "serialize_123");
    assert_eq!(deserialized.method, "serialize_method");
    assert_eq!(deserialized.params.value, "serialize_test");
    assert_eq!(deserialized.params.count, 100);
    assert_eq!(deserialized.method_type, RpcMethod::Mutation);
}

#[test]
fn test_rpc_response_serialization() {
    let result = TestResult {
        message: "serialize_response".to_string(),
        processed_count: 5,
    };
    
    let response = RpcResponse {
        id: "response_123".to_string(),
        result: Some(result),
        error: None,
    };
    
    // Test JSON serialization
    let json = serde_json::to_string(&response).expect("Should serialize to JSON");
    let deserialized: RpcResponse<TestResult> = serde_json::from_str(&json).expect("Should deserialize from JSON");
    
    assert_eq!(deserialized.id, "response_123");
    assert!(deserialized.result.is_some());
    assert!(deserialized.error.is_none());
}

#[test]
fn test_rpc_client_creation() {
    let provider = WebSocketProvider::new("ws://localhost:8080");
    let context = WebSocketContext::new(provider);
    
    let client: RpcClient<TestParams> = RpcClient::new(context);
    
    // Test that client was created successfully
    assert_eq!(client.next_id.load(std::sync::atomic::Ordering::SeqCst), 1);
}

#[test]
fn test_rpc_client_id_generation() {
    let provider = WebSocketProvider::new("ws://localhost:8080");
    let context = WebSocketContext::new(provider);
    
    let client: RpcClient<TestParams> = RpcClient::new(context);
    
    let id1 = client.generate_id();
    let id2 = client.generate_id();
    let id3 = client.generate_id();
    
    assert_eq!(id1, "rpc_1");
    assert_eq!(id2, "rpc_2");
    assert_eq!(id3, "rpc_3");
}

#[test]
fn test_chat_service_params() {
    let send_params = SendMessageParams {
        room_id: "room_123".to_string(),
        content: "Hello, World!".to_string(),
    };
    
    let get_params = GetMessagesParams {
        room_id: "room_123".to_string(),
        limit: 50,
    };
    
    let subscribe_params = SubscribeMessagesParams {
        room_id: "room_123".to_string(),
    };
    
    assert_eq!(send_params.room_id, "room_123");
    assert_eq!(send_params.content, "Hello, World!");
    assert_eq!(get_params.room_id, "room_123");
    assert_eq!(get_params.limit, 50);
    assert_eq!(subscribe_params.room_id, "room_123");
}

#[test]
fn test_chat_message_creation() {
    let message = ChatMessage {
        id: "msg_123".to_string(),
        room_id: "room_456".to_string(),
        content: "Test message".to_string(),
        sender: "user_789".to_string(),
        timestamp: 1234567890,
    };
    
    assert_eq!(message.id, "msg_123");
    assert_eq!(message.room_id, "room_456");
    assert_eq!(message.content, "Test message");
    assert_eq!(message.sender, "user_789");
    assert_eq!(message.timestamp, 1234567890);
}

#[test]
fn test_message_id_creation() {
    let message_id = MessageId {
        id: "generated_msg_123".to_string(),
    };
    
    assert_eq!(message_id.id, "generated_msg_123");
}

#[test]
fn test_rpc_service_trait_definition() {
    // This test verifies that the RpcService trait is properly defined
    // We can't easily test the trait implementation without a concrete service,
    // but we can verify the trait exists and has the expected methods
    
    trait TestRpcService: RpcService<Context = ()> {}
    
    // This compiles if the trait is properly defined
    assert!(true);
}

#[test]
fn test_rpc_subscription_creation() {
    let provider = WebSocketProvider::new("ws://localhost:8080");
    let context = WebSocketContext::new(provider);
    
    let client = RpcClient::new(context);
    let params = TestParams {
        value: "subscribe_test".to_string(),
        count: 1,
    };
    
    let subscription = client.subscribe::<TestResult>("test_subscription", params);
    
    // Test that subscription was created with correct ID
    assert_eq!(subscription.id, "rpc_1");
}

#[test]
fn test_rpc_error_serialization() {
    let error = RpcError {
        code: 400,
        message: "Bad Request".to_string(),
        data: Some(serde_json::json!({"field": "value"})),
    };
    
    let json = serde_json::to_string(&error).expect("Should serialize to JSON");
    let deserialized: RpcError = serde_json::from_str(&json).expect("Should deserialize from JSON");
    
    assert_eq!(deserialized.code, 400);
    assert_eq!(deserialized.message, "Bad Request");
    assert!(deserialized.data.is_some());
}

#[test]
fn test_rpc_method_serialization() {
    let methods = vec![
        RpcMethod::Query,
        RpcMethod::Mutation,
        RpcMethod::Subscription,
    ];
    
    for method in methods {
        let json = serde_json::to_string(&method).expect("Should serialize to JSON");
        let deserialized: RpcMethod = serde_json::from_str(&json).expect("Should deserialize from JSON");
        assert_eq!(deserialized, method);
    }
}

#[test]
fn test_rpc_request_with_different_types() {
    // Test with string params
    let string_request = RpcRequest {
        id: "str_req".to_string(),
        method: "string_method".to_string(),
        params: "test_string".to_string(),
        method_type: RpcMethod::Query,
    };
    
    // Test with numeric params
    let numeric_request = RpcRequest {
        id: "num_req".to_string(),
        method: "numeric_method".to_string(),
        params: 42u32,
        method_type: RpcMethod::Mutation,
    };
    
    // Test with boolean params
    let bool_request = RpcRequest {
        id: "bool_req".to_string(),
        method: "boolean_method".to_string(),
        params: true,
        method_type: RpcMethod::Subscription,
    };
    
    assert_eq!(string_request.params, "test_string");
    assert_eq!(numeric_request.params, 42);
    assert_eq!(bool_request.params, true);
}

#[test]
fn test_rpc_response_with_different_result_types() {
    // Test with string result
    let string_response = RpcResponse {
        id: "str_resp".to_string(),
        result: Some("success".to_string()),
        error: None,
    };
    
    // Test with numeric result
    let numeric_response = RpcResponse {
        id: "num_resp".to_string(),
        result: Some(100u32),
        error: None,
    };
    
    // Test with boolean result
    let bool_response = RpcResponse {
        id: "bool_resp".to_string(),
        result: Some(false),
        error: None,
    };
    
    assert_eq!(string_response.result, Some("success".to_string()));
    assert_eq!(numeric_response.result, Some(100));
    assert_eq!(bool_response.result, Some(false));
}

#[test]
fn test_rpc_error_codes() {
    let standard_errors = vec![
        RpcError { code: -32700, message: "Parse error".to_string(), data: None },
        RpcError { code: -32600, message: "Invalid Request".to_string(), data: None },
        RpcError { code: -32601, message: "Method not found".to_string(), data: None },
        RpcError { code: -32602, message: "Invalid params".to_string(), data: None },
        RpcError { code: -32603, message: "Internal error".to_string(), data: None },
    ];
    
    for error in standard_errors {
        assert!(error.code < 0); // Standard JSON-RPC error codes are negative
        assert!(!error.message.is_empty());
    }
}

#[test]
fn test_rpc_request_roundtrip() {
    let original_params = TestParams {
        value: "roundtrip_test".to_string(),
        count: 999,
    };
    
    let original_request = RpcRequest {
        id: "roundtrip_123".to_string(),
        method: "roundtrip_method".to_string(),
        params: original_params,
        method_type: RpcMethod::Query,
    };
    
    // Serialize to JSON
    let json = serde_json::to_string(&original_request).expect("Should serialize");
    
    // Deserialize from JSON
    let deserialized_request: RpcRequest<TestParams> = serde_json::from_str(&json).expect("Should deserialize");
    
    // Verify roundtrip
    assert_eq!(original_request.id, deserialized_request.id);
    assert_eq!(original_request.method, deserialized_request.method);
    assert_eq!(original_request.params.value, deserialized_request.params.value);
    assert_eq!(original_request.params.count, deserialized_request.params.count);
    assert_eq!(original_request.method_type, deserialized_request.method_type);
}
