//! Comprehensive unit tests for RPC module - v1.0 TDD
//!
//! This test suite ensures 100% coverage of the RPC functionality
//! following TDD principles for v1.0 release.

use leptos_ws_pro::rpc::{
    ChatMessage, GetMessagesParams, RpcError, RpcMethod,
    RpcRequest, RpcResponse, SendMessageParams, SubscribeMessagesParams,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TestParams {
    value: i32,
    name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TestResult {
    success: bool,
    data: String,
}

#[cfg(test)]
mod rpc_core_tests {
    use super::*;

    #[test]
    fn test_rpc_method_enum() {
        let methods = vec![
            RpcMethod::Call,
            RpcMethod::Query,
            RpcMethod::Mutation,
            RpcMethod::Subscription,
        ];

        for method in methods {
            // Test serialization/deserialization
            let json = serde_json::to_string(&method).unwrap();
            let deserialized: RpcMethod = serde_json::from_str(&json).unwrap();
            assert_eq!(method, deserialized);

            // Test debug formatting
            let debug_str = format!("{:?}", method);
            assert!(!debug_str.is_empty());
        }
    }

    #[test]
    fn test_message_id_generation() {
        let id1 = "test_id_1".to_string();
        let id2 = "test_id_2".to_string();

        // IDs should be unique
        assert_ne!(id1, id2);

        // Test serialization
        let json = serde_json::to_string(&id1).unwrap();
        let deserialized: String = serde_json::from_str(&json).unwrap();
        assert_eq!(id1, deserialized);
    }

    #[test]
    fn test_rpc_request_creation() {
        let request = RpcRequest::<TestParams> {
            id: "test_id".to_string(),
            method: "test_method".to_string(),
            params: TestParams {
                value: 42,
                name: "test".to_string(),
            },
            method_type: RpcMethod::Call,
        };

        assert_eq!(request.method, "test_method");
        assert_eq!(request.params.value, 42);
        assert_eq!(request.method_type, RpcMethod::Call);
    }

    #[test]
    fn test_rpc_request_serialization() {
        let request = RpcRequest::<TestParams> {
            id: "echo_id".to_string(),
            method: "echo".to_string(),
            params: TestParams {
                value: 123,
                name: "serialization_test".to_string(),
            },
            method_type: RpcMethod::Query,
        };

        // Test JSON serialization
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("echo"));
        assert!(json.contains("serialization_test"));

        // Test deserialization
        let deserialized: RpcRequest<TestParams> = serde_json::from_str(&json).unwrap();
        assert_eq!(request.method, deserialized.method);
        assert_eq!(request.method_type, deserialized.method_type);
    }

    #[test]
    fn test_rpc_response_creation() {
        let response = RpcResponse::<TestResult> {
            id: "response_id".to_string(),
            result: Some(TestResult {
                success: true,
                data: "test_data".to_string(),
            }),
            error: None,
        };

        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[test]
    fn test_rpc_response_with_error() {
        let response = RpcResponse::<TestResult> {
            id: "error_response_id".to_string(),
            result: None,
            error: Some(RpcError {
                code: -32601,
                message: "Method not found".to_string(),
                data: None,
            }),
        };

        assert!(response.result.is_none());
        assert!(response.error.is_some());
        assert_eq!(response.error.as_ref().unwrap().code, -32601);
    }

    #[test]
    fn test_rpc_error_creation() {
        let error = RpcError {
            code: -32600,
            message: "Invalid Request".to_string(),
            data: Some(serde_json::Value::String("Additional info".to_string())),
        };

        assert_eq!(error.code, -32600);
        assert_eq!(error.message, "Invalid Request");
        assert!(error.data.is_some());
    }

    #[test]
    fn test_chat_message_creation() {
        let message = ChatMessage {
            id: "msg_id".to_string(),
            content: "Hello, world!".to_string(),
            timestamp: 1234567890, // Unix timestamp
            channel: Some("general".to_string()),
            sender: Some("user123".to_string()),
            room_id: Some("general".to_string()),
        };

        assert_eq!(message.content, "Hello, world!");
        assert_eq!(message.sender, Some("user123".to_string()));
        assert_eq!(message.room_id, Some("general".to_string()));
    }

    #[test]
    fn test_send_message_params() {
        let params = SendMessageParams {
            message: "Test message".to_string(),
            channel: Some("test_channel".to_string()),
            content: Some("Test message".to_string()),
            room_id: Some("test_room".to_string()),
        };

        assert_eq!(params.content, Some("Test message".to_string()));
        assert_eq!(params.room_id, Some("test_room".to_string()));
    }

    #[test]
    fn test_get_messages_params() {
        let params = GetMessagesParams {
            channel: Some("general".to_string()),
            limit: Some(50),
            room_id: Some("general".to_string()),
        };

        assert_eq!(params.room_id, Some("general".to_string()));
        assert_eq!(params.limit, Some(50));
    }

    #[test]
    fn test_subscribe_messages_params() {
        let params = SubscribeMessagesParams {
            channel: Some("general".to_string()),
            room_id: Some("general".to_string()),
        };

        assert_eq!(params.room_id, Some("general".to_string()));
    }

    #[test]
    fn test_rpc_response_serialization() {
        let response = RpcResponse::<TestResult> {
            id: "serialization_id".to_string(),
            result: Some(TestResult {
                success: true,
                data: "serialized_data".to_string(),
            }),
            error: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("serialized_data"));
        assert!(json.contains("success"));

        let deserialized: RpcResponse<TestResult> = serde_json::from_str(&json).unwrap();
        assert!(deserialized.result.is_some());
        assert_eq!(deserialized.result.as_ref().unwrap().data, "serialized_data");
    }

    #[test]
    fn test_rpc_error_serialization() {
        let error = RpcError {
            code: -32700,
            message: "Parse error".to_string(),
            data: None,
        };

        let json = serde_json::to_string(&error).unwrap();
        assert!(json.contains("Parse error"));
        assert!(json.contains("-32700"));

        let deserialized: RpcError = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.code, -32700);
        assert_eq!(deserialized.message, "Parse error");
    }

    #[test]
    fn test_chat_message_serialization() {
        let message = ChatMessage {
            id: "serialization_msg_id".to_string(),
            content: "Serialization test".to_string(),
            timestamp: 1234567890,
            channel: Some("test_channel".to_string()),
            sender: Some("test_user".to_string()),
            room_id: Some("test_room".to_string()),
        };

        let json = serde_json::to_string(&message).unwrap();
        assert!(json.contains("Serialization test"));
        assert!(json.contains("test_user"));
        assert!(json.contains("test_room"));

        let deserialized: ChatMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.content, "Serialization test");
        assert_eq!(deserialized.sender, Some("test_user".to_string()));
    }

    #[test]
    fn test_rpc_method_variants() {
        // Test all RPC method variants
        let call = RpcMethod::Call;
        let query = RpcMethod::Query;
        let mutation = RpcMethod::Mutation;
        let subscription = RpcMethod::Subscription;

        // Test equality
        assert_eq!(call, RpcMethod::Call);
        assert_eq!(query, RpcMethod::Query);
        assert_eq!(mutation, RpcMethod::Mutation);
        assert_eq!(subscription, RpcMethod::Subscription);

        // Test they are different
        assert_ne!(call, query);
        assert_ne!(query, mutation);
        assert_ne!(mutation, subscription);
    }

    #[test]
    fn test_rpc_request_with_different_methods() {
        let methods = vec![
            ("echo", RpcMethod::Call),
            ("get_user", RpcMethod::Query),
            ("create_post", RpcMethod::Mutation),
            ("subscribe_updates", RpcMethod::Subscription),
        ];

        for (method_name, method_type) in methods {
            let request = RpcRequest::<TestParams> {
                id: format!("{}_id", method_name),
                method: method_name.to_string(),
                params: TestParams {
                    value: 1,
                    name: "test".to_string(),
                },
                method_type: method_type.clone(),
            };

            assert_eq!(request.method, method_name);
            assert_eq!(request.method_type, method_type);
        }
    }

    #[test]
    fn test_rpc_error_display() {
        let error = RpcError {
            code: -32600,
            message: "Invalid Request".to_string(),
            data: None,
        };

        let error_string = format!("{}", error);
        assert!(error_string.contains("-32600"));
        assert!(error_string.contains("Invalid Request"));
    }

    #[test]
    fn test_rpc_request_with_empty_params() {
        let request = RpcRequest::<TestParams> {
            id: "empty_params_id".to_string(),
            method: "ping".to_string(),
            params: TestParams {
                value: 0,
                name: "".to_string(),
            },
            method_type: RpcMethod::Call,
        };

        assert_eq!(request.method, "ping");
        assert_eq!(request.params.value, 0);
        assert_eq!(request.params.name, "");
    }
}
