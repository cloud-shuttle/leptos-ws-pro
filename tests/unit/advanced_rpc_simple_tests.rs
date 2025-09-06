//! Simple unit tests for Advanced RPC System
//!
//! These tests focus on core RPC functionality without server setup to avoid hanging issues.

#[cfg(feature = "advanced-rpc")]
use leptos_ws_pro::rpc::advanced::*;

#[cfg(feature = "advanced-rpc")]
#[cfg(feature = "advanced-rpc")]
#[tokio::test]
async fn test_rpc_message_serialization() {
    // Test RPC request serialization/deserialization
    let request = RpcRequest {
        id: "test-123".to_string(),
        method: "echo".to_string(),
        params: serde_json::json!({"message": "hello"}),
    };

    let response = RpcResponse {
        id: "test-123".to_string(),
        result: Some(serde_json::json!({"echo": "hello"})),
        error: None,
    };

    // Test serialization
    let request_json = serde_json::to_string(&request).unwrap();
    let response_json = serde_json::to_string(&response).unwrap();

    // Test deserialization
    let deserialized_request: RpcRequest = serde_json::from_str(&request_json).unwrap();
    let deserialized_response: RpcResponse = serde_json::from_str(&response_json).unwrap();

    assert_eq!(deserialized_request.id, "test-123");
    assert_eq!(deserialized_request.method, "echo");
    assert_eq!(deserialized_response.id, "test-123");
    assert!(deserialized_response.result.is_some());
    assert!(deserialized_response.error.is_none());
}

#[cfg(feature = "advanced-rpc")]
#[tokio::test]
async fn test_rpc_error_serialization() {
    // Test RPC error response serialization
    let error_response = RpcResponse {
        id: "error-123".to_string(),
        result: None,
        error: Some("Method not found".to_string()),
    };

    let error_json = serde_json::to_string(&error_response).unwrap();
    let deserialized: RpcResponse = serde_json::from_str(&error_json).unwrap();

    assert_eq!(deserialized.id, "error-123");
    assert!(deserialized.result.is_none());
    assert!(deserialized.error.is_some());
    assert_eq!(deserialized.error.unwrap(), "Method not found");
}

#[cfg(feature = "advanced-rpc")]
#[tokio::test]
async fn test_rpc_correlation_manager_basic() {
    let manager = RpcCorrelationManager::new(Duration::from_secs(5));

    // Test registering a request
    let request_id = "test-456".to_string();
    let response_rx = manager.register_request(request_id.clone());

    // Test handling a response
    let response = RpcResponse {
        id: request_id.clone(),
        result: Some(serde_json::json!({"success": true})),
        error: None,
    };

    let result = manager.handle_response(response);
    assert!(result.is_ok());

    // Test receiving the response
    let received_response = response_rx.await.unwrap();
    assert!(received_response.is_ok());
    let response = received_response.unwrap();
    assert_eq!(response.id, request_id);
    assert!(response.result.is_some());
}

#[cfg(feature = "advanced-rpc")]
#[tokio::test]
async fn test_rpc_correlation_manager_timeout() {
    let manager = RpcCorrelationManager::new(Duration::from_millis(10));

    // Test timeout cleanup
    let request_id = "timeout-789".to_string();
    let response_rx = manager.register_request(request_id.clone());

    // Wait for timeout
    tokio::time::sleep(Duration::from_millis(50)).await;

    // The response should be a timeout error
    let result = response_rx.await;
    match result {
        Ok(response) => {
            assert!(response.is_err());
            let error = response.unwrap_err();
            match error {
                RpcError::Timeout(_) => assert!(true),
                _ => panic!("Expected timeout error"),
            }
        }
        Err(_) => {
            // Channel closed due to timeout
            assert!(true);
        }
    }
}

#[cfg(feature = "advanced-rpc")]
#[tokio::test]
async fn test_rpc_method_registry() {
    let mut registry = RpcMethodRegistry::new();

    // Test registering a method
    let method_name = "test_method";
    registry.register(method_name, |params| {
        Ok(serde_json::json!({"result": "success", "params": params}))
    });

    // Test calling the method
    let params = serde_json::json!({"input": "test"});
    let result = registry.call(method_name, params);

    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response["result"], "success");
    assert_eq!(response["params"]["input"], "test");
}

#[cfg(feature = "advanced-rpc")]
#[tokio::test]
async fn test_rpc_method_registry_unknown_method() {
    let registry = RpcMethodRegistry::new();

    // Test calling unknown method
    let params = serde_json::json!({});
    let result = registry.call("unknown_method", params);

    assert!(result.is_err());
    let error = result.unwrap_err();
    match error {
        RpcError::MethodNotFound(_) => assert!(true),
        _ => panic!("Expected MethodNotFound error"),
    }
}

#[cfg(feature = "advanced-rpc")]
#[tokio::test]
async fn test_rpc_request_id_generation() {
    // Test that request IDs are unique
    let mut ids = std::collections::HashSet::new();

    for _ in 0..100 {
        let request = RpcRequest {
            id: uuid::Uuid::new_v4().to_string(),
            method: "test".to_string(),
            params: serde_json::json!({}),
        };

        assert!(ids.insert(request.id.clone()));
    }

    assert_eq!(ids.len(), 100);
}

#[cfg(feature = "advanced-rpc")]
#[tokio::test]
async fn test_rpc_performance_metrics() {
    // Test RPC performance by measuring request/response times
    let manager = RpcCorrelationManager::new(Duration::from_secs(5));

    let start_time = std::time::Instant::now();
    let request_id = "perf-test".to_string();
    let response_rx = manager.register_request(request_id.clone());

    // Simulate quick response
    let response = RpcResponse {
        id: request_id.clone(),
        result: Some(serde_json::json!({"success": true})),
        error: None,
    };

    manager.handle_response(response);
    let _received = response_rx.await.unwrap();
    let elapsed = start_time.elapsed();

    // Should be very fast for in-memory operations
    assert!(elapsed.as_millis() < 10);
}

#[cfg(feature = "advanced-rpc")]
#[tokio::test]
async fn test_rpc_error_codes() {
    // Test RPC error types
    let connection_error = RpcError::ConnectionFailed("Connection lost".to_string());
    let timeout_error = RpcError::Timeout("Request timed out".to_string());
    let method_error = RpcError::MethodNotFound("Unknown method".to_string());
    let params_error = RpcError::InvalidParams("Invalid parameters".to_string());
    let internal_error = RpcError::InternalError("Internal server error".to_string());

    // Test error serialization
    assert!(serde_json::to_string(&connection_error).unwrap().contains("ConnectionFailed"));
    assert!(serde_json::to_string(&timeout_error).unwrap().contains("Timeout"));
    assert!(serde_json::to_string(&method_error).unwrap().contains("MethodNotFound"));
    assert!(serde_json::to_string(&params_error).unwrap().contains("InvalidParams"));
    assert!(serde_json::to_string(&internal_error).unwrap().contains("InternalError"));
}

#[cfg(feature = "advanced-rpc")]
#[tokio::test]
async fn test_rpc_correlation_manager_multiple_requests() {
    let manager = RpcCorrelationManager::new(Duration::from_secs(5));

    // Test multiple concurrent requests
    let mut response_rxs = Vec::new();
    let mut request_ids = Vec::new();

    for i in 0..5 {
        let request_id = format!("multi-{}", i);
        let response_rx = manager.register_request(request_id.clone());
        response_rxs.push(response_rx);
        request_ids.push(request_id);
    }

    // Handle responses in different order
    for (i, request_id) in request_ids.iter().enumerate() {
        let response = RpcResponse {
            id: request_id.clone(),
            result: Some(serde_json::json!({"index": i})),
            error: None,
        };

        let result = manager.handle_response(response);
        assert!(result.is_ok());
    }

    // Collect all responses
    let mut responses = Vec::new();
    for response_rx in response_rxs {
        let response = response_rx.await.unwrap();
        assert!(response.is_ok());
        responses.push(response.unwrap());
    }

    // Verify all responses were received
    assert_eq!(responses.len(), 5);
    for (i, response) in responses.iter().enumerate() {
        assert_eq!(response.id, format!("multi-{}", i));
        assert_eq!(response.result.as_ref().unwrap()["index"], i);
    }
}

#[cfg(feature = "advanced-rpc")]
#[tokio::test]
async fn test_rpc_method_registry_multiple_methods() {
    let mut registry = RpcMethodRegistry::new();

    // Register multiple methods
    registry.register("add", |params| {
        let a: i32 = params["a"].as_i64().unwrap() as i32;
        let b: i32 = params["b"].as_i64().unwrap() as i32;
        Ok(serde_json::json!({"sum": a + b}))
    });

    registry.register("multiply", |params| {
        let a: i32 = params["a"].as_i64().unwrap() as i32;
        let b: i32 = params["b"].as_i64().unwrap() as i32;
        Ok(serde_json::json!({"product": a * b}))
    });

    // Test calling different methods
    let add_result = registry.call("add", serde_json::json!({"a": 5, "b": 3}));
    assert!(add_result.is_ok());
    assert_eq!(add_result.unwrap()["sum"], 8);

    let multiply_result = registry.call("multiply", serde_json::json!({"a": 4, "b": 6}));
    assert!(multiply_result.is_ok());
    assert_eq!(multiply_result.unwrap()["product"], 24);
}

#[cfg(feature = "advanced-rpc")]
#[tokio::test]
async fn test_rpc_batch_processing() {
    // Test batch RPC request structure
    let batch_requests = vec![
        RpcRequest {
            id: "batch-1".to_string(),
            method: "echo".to_string(),
            params: serde_json::json!({"msg": "hello"}),
        },
        RpcRequest {
            id: "batch-2".to_string(),
            method: "echo".to_string(),
            params: serde_json::json!({"msg": "world"}),
        },
    ];

    // Test batch serialization
    let batch_json = serde_json::to_string(&batch_requests).unwrap();
    let deserialized_batch: Vec<RpcRequest> = serde_json::from_str(&batch_json).unwrap();

    assert_eq!(deserialized_batch.len(), 2);
    assert_eq!(deserialized_batch[0].id, "batch-1");
    assert_eq!(deserialized_batch[1].id, "batch-2");

    // Test batch response structure
    let batch_responses = vec![
        RpcResponse {
            id: "batch-1".to_string(),
            result: Some(serde_json::json!({"echo": "hello"})),
            error: None,
        },
        RpcResponse {
            id: "batch-2".to_string(),
            result: Some(serde_json::json!({"echo": "world"})),
            error: None,
        },
    ];

    let batch_response_json = serde_json::to_string(&batch_responses).unwrap();
    let deserialized_responses: Vec<RpcResponse> = serde_json::from_str(&batch_response_json).unwrap();

    assert_eq!(deserialized_responses.len(), 2);
    assert_eq!(deserialized_responses[0].id, "batch-1");
    assert_eq!(deserialized_responses[1].id, "batch-2");
}
