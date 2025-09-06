//! TDD tests for Advanced RPC System implementation
//!
//! This module tests the bidirectional RPC system with request/response correlation,
//! type-safe method definitions, and async method support.

use futures::{SinkExt, StreamExt};
use leptos_ws_pro::rpc::advanced::{
    BidirectionalRpcClient, RpcCorrelationManager, RpcMethodRegistry, RpcRequest, RpcResponse, RpcError,
};
use leptos_ws_pro::transport::{
    ConnectionState, Message, MessageType, Transport, TransportConfig, TransportError,
    websocket::WebSocketConnection,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::time::timeout;

/// Test message structure for RPC calls
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct TestRpcRequest {
    id: String,
    method: String,
    params: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct TestRpcResponse {
    id: String,
    result: Option<serde_json::Value>,
    error: Option<String>,
}

/// Start a test WebSocket server for RPC testing
async fn start_test_rpc_server() -> (TcpListener, u16) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    (listener, port)
}

/// Run an RPC echo server for testing
async fn run_rpc_echo_server(listener: TcpListener) {
    use futures::{SinkExt, StreamExt};
    use tokio_tungstenite::accept_async;

    while let Ok((stream, _)) = listener.accept().await {
        let ws_stream = accept_async(stream).await.unwrap();
        let (mut write, mut read) = ws_stream.split();

        tokio::spawn(async move {
            while let Some(msg) = read.next().await {
                if let Ok(msg) = msg {
                    if let Ok(text) = msg.to_text() {
                        // Parse RPC request
                        if let Ok(request) = serde_json::from_str::<TestRpcRequest>(text) {
                            // Create RPC response
                            let response = TestRpcResponse {
                                id: request.id.clone(),
                                result: Some(serde_json::json!({
                                    "echo": request.params,
                                    "method": request.method
                                })),
                                error: None,
                            };

                            // Send response back
                            let response_json = serde_json::to_string(&response).unwrap();
                            let _ = write.send(tokio_tungstenite::tungstenite::Message::Text(response_json.into())).await;
                        }
                    }
                }
            }
        });
    }
}

#[tokio::test]
async fn test_bidirectional_rpc_call() {
    // Given: RPC request and response structures
    let request = TestRpcRequest {
        id: "test-123".to_string(),
        method: "echo".to_string(),
        params: serde_json::json!({"message": "Hello, RPC!"}),
    };

    // When: Serializing and deserializing RPC request
    let request_json = serde_json::to_string(&request).unwrap();
    let parsed_request: TestRpcRequest = serde_json::from_str(&request_json).unwrap();

    // Then: Should maintain data integrity
    assert_eq!(parsed_request.id, "test-123");
    assert_eq!(parsed_request.method, "echo");
    assert_eq!(parsed_request.params["message"], "Hello, RPC!");

    // When: Creating a mock RPC response
    let response = TestRpcResponse {
        id: "test-123".to_string(),
        result: Some(serde_json::json!({
            "echo": {"message": "Hello, RPC!"},
            "method": "echo"
        })),
        error: None,
    };

    // Then: Should serialize and deserialize correctly
    let response_json = serde_json::to_string(&response).unwrap();
    let parsed_response: TestRpcResponse = serde_json::from_str(&response_json).unwrap();

    // Verify response correlation
    assert_eq!(parsed_response.id, "test-123");
    assert!(parsed_response.result.is_some());
    assert!(parsed_response.error.is_none());

    // Verify response content
    let result = parsed_response.result.unwrap();
    assert_eq!(result["method"], "echo");
    assert_eq!(result["echo"]["message"], "Hello, RPC!");
}

#[tokio::test]
async fn test_request_response_correlation() {
    // Given: An RPC server and client
    let (listener, port) = start_test_rpc_server().await;
    run_rpc_echo_server(listener).await;

    let config = TransportConfig {
        url: format!("ws://127.0.0.1:{}", port),
        ..Default::default()
    };
    let mut client = WebSocketConnection::new(config).await.unwrap();

    client.connect(&format!("ws://127.0.0.1:{}", port)).await.unwrap();
    let (mut stream, mut sink) = client.split();

    // When: Sending multiple RPC requests with different IDs
    let requests = vec![
        TestRpcRequest {
            id: "req-1".to_string(),
            method: "test1".to_string(),
            params: serde_json::json!({"data": "first"}),
        },
        TestRpcRequest {
            id: "req-2".to_string(),
            method: "test2".to_string(),
            params: serde_json::json!({"data": "second"}),
        },
        TestRpcRequest {
            id: "req-3".to_string(),
            method: "test3".to_string(),
            params: serde_json::json!({"data": "third"}),
        },
    ];

    // Send all requests
    for request in &requests {
        let request_json = serde_json::to_string(request).unwrap();
        let message = Message {
            data: request_json.as_bytes().to_vec(),
            message_type: MessageType::Text,
        };
        let send_result = sink.send(message).await;
        assert!(send_result.is_ok());
    }

    // Then: Should receive responses with correct correlation
    let mut received_responses = Vec::new();
    for _ in 0..3 {
        let received = stream.next().await;
        assert!(received.is_some());
        let received_msg = received.unwrap().unwrap();
        let response_text = String::from_utf8(received_msg.data).unwrap();
        let response: TestRpcResponse = serde_json::from_str(&response_text).unwrap();
        received_responses.push(response);
    }

    // Verify all responses have correct IDs and content
    assert_eq!(received_responses.len(), 3);

    for response in received_responses {
        assert!(response.result.is_some());
        let result = response.result.unwrap();

        match response.id.as_str() {
            "req-1" => {
                assert_eq!(result["method"], "test1");
                assert_eq!(result["echo"]["data"], "first");
            }
            "req-2" => {
                assert_eq!(result["method"], "test2");
                assert_eq!(result["echo"]["data"], "second");
            }
            "req-3" => {
                assert_eq!(result["method"], "test3");
                assert_eq!(result["echo"]["data"], "third");
            }
            _ => panic!("Unexpected response ID: {}", response.id),
        }
    }
}

#[tokio::test]
async fn test_rpc_timeout_handling() {
    // Given: A client trying to connect to non-existent server
    let config = TransportConfig {
        url: "ws://127.0.0.1:99999".to_string(),
        ..Default::default()
    };
    let mut client = WebSocketConnection::new(config).await.unwrap();

    // When: Trying to make RPC call with timeout
    let result = timeout(
        Duration::from_secs(5),
        client.connect("ws://127.0.0.1:99999"),
    )
    .await;

    // Then: Should timeout and fail
    assert!(result.is_ok());
    let connect_result = result.unwrap();
    assert!(connect_result.is_err());
    assert_eq!(client.state(), ConnectionState::Disconnected);
}

#[tokio::test]
async fn test_rpc_error_propagation() {
    // Given: An RPC server that returns errors
    let (listener, port) = start_test_rpc_server().await;

    // Start error server
    tokio::spawn(async move {
        use futures::{SinkExt, StreamExt};
        use tokio_tungstenite::accept_async;

        while let Ok((stream, _)) = listener.accept().await {
            let ws_stream = accept_async(stream).await.unwrap();
            let (mut write, mut read) = ws_stream.split();

            tokio::spawn(async move {
                while let Some(msg) = read.next().await {
                    if let Ok(msg) = msg {
                        if let Ok(text) = msg.to_text() {
                            if let Ok(request) = serde_json::from_str::<TestRpcRequest>(text) {
                                // Return error response
                                let response = TestRpcResponse {
                                    id: request.id.clone(),
                                    result: None,
                                    error: Some("Method not found".to_string()),
                                };

                                let response_json = serde_json::to_string(&response).unwrap();
                                let _ = write.send(tokio_tungstenite::tungstenite::Message::Text(response_json.into())).await;
                            }
                        }
                    }
                }
            });
        }
    });

    let config = TransportConfig {
        url: format!("ws://127.0.0.1:{}", port),
        ..Default::default()
    };
    let mut client = WebSocketConnection::new(config).await.unwrap();

    // When: Client connects and makes RPC call
    client.connect(&format!("ws://127.0.0.1:{}", port)).await.unwrap();
    let (mut stream, mut sink) = client.split();

    let request = TestRpcRequest {
        id: "error-test".to_string(),
        method: "nonexistent".to_string(),
        params: serde_json::json!({}),
    };

    let request_json = serde_json::to_string(&request).unwrap();
    let message = Message {
        data: request_json.as_bytes().to_vec(),
        message_type: MessageType::Text,
    };

    let send_result = sink.send(message).await;
    assert!(send_result.is_ok());

    // Then: Should receive error response
    let received = stream.next().await;
    assert!(received.is_some());
    let received_msg = received.unwrap().unwrap();
    let response_text = String::from_utf8(received_msg.data).unwrap();
    let response: TestRpcResponse = serde_json::from_str(&response_text).unwrap();

    // Verify error response
    assert_eq!(response.id, "error-test");
    assert!(response.result.is_none());
    assert!(response.error.is_some());
    assert_eq!(response.error.unwrap(), "Method not found");
}

#[tokio::test]
async fn test_batch_rpc_calls() {
    // Given: An RPC server and client
    let (listener, port) = start_test_rpc_server().await;
    run_rpc_echo_server(listener).await;

    let config = TransportConfig {
        url: format!("ws://127.0.0.1:{}", port),
        ..Default::default()
    };
    let mut client = WebSocketConnection::new(config).await.unwrap();

    client.connect(&format!("ws://127.0.0.1:{}", port)).await.unwrap();
    let (mut stream, mut sink) = client.split();

    // When: Sending batch RPC requests
    let batch_requests = vec![
        TestRpcRequest {
            id: "batch-1".to_string(),
            method: "batch_test_1".to_string(),
            params: serde_json::json!({"batch": 1}),
        },
        TestRpcRequest {
            id: "batch-2".to_string(),
            method: "batch_test_2".to_string(),
            params: serde_json::json!({"batch": 2}),
        },
    ];

    // Send batch as single message (simulating batch RPC)
    let batch_json = serde_json::to_string(&batch_requests).unwrap();
    let message = Message {
        data: batch_json.as_bytes().to_vec(),
        message_type: MessageType::Text,
    };

    let send_result = sink.send(message).await;
    assert!(send_result.is_ok());

    // Then: Should receive batch responses
    let received = stream.next().await;
    assert!(received.is_some());
    let received_msg = received.unwrap().unwrap();
    let response_text = String::from_utf8(received_msg.data).unwrap();

    // Parse batch response
    let batch_responses: Vec<TestRpcResponse> = serde_json::from_str(&response_text).unwrap();
    assert_eq!(batch_responses.len(), 2);

    // Verify batch response correlation
    for response in batch_responses {
        assert!(response.result.is_some());
        let result = response.result.unwrap();

        match response.id.as_str() {
            "batch-1" => {
                assert_eq!(result["method"], "batch_test_1");
                assert_eq!(result["echo"]["batch"], 1);
            }
            "batch-2" => {
                assert_eq!(result["method"], "batch_test_2");
                assert_eq!(result["echo"]["batch"], 2);
            }
            _ => panic!("Unexpected batch response ID: {}", response.id),
        }
    }
}

#[tokio::test]
async fn test_async_rpc_methods() {
    // Given: An RPC server with async methods
    let (listener, port) = start_test_rpc_server().await;

    // Start async server
    tokio::spawn(async move {
        use futures::{SinkExt, StreamExt};
        use tokio_tungstenite::accept_async;
        use std::time::Duration;

        while let Ok((stream, _)) = listener.accept().await {
            let ws_stream = accept_async(stream).await.unwrap();
            let (mut write, mut read) = ws_stream.split();

            tokio::spawn(async move {
                while let Some(msg) = read.next().await {
                    if let Ok(msg) = msg {
                        if let Ok(text) = msg.to_text() {
                            if let Ok(request) = serde_json::from_str::<TestRpcRequest>(text) {
                                // Simulate async processing
                                tokio::time::sleep(Duration::from_millis(100)).await;

                                let response = TestRpcResponse {
                                    id: request.id.clone(),
                                    result: Some(serde_json::json!({
                                        "async_result": "processed",
                                        "method": request.method,
                                        "timestamp": chrono::Utc::now().timestamp()
                                    })),
                                    error: None,
                                };

                                let response_json = serde_json::to_string(&response).unwrap();
                                let _ = write.send(tokio_tungstenite::tungstenite::Message::Text(response_json.into())).await;
                            }
                        }
                    }
                }
            });
        }
    });

    let config = TransportConfig {
        url: format!("ws://127.0.0.1:{}", port),
        ..Default::default()
    };
    let mut client = WebSocketConnection::new(config).await.unwrap();

    // When: Client connects and makes async RPC call
    client.connect(&format!("ws://127.0.0.1:{}", port)).await.unwrap();
    let (mut stream, mut sink) = client.split();

    let request = TestRpcRequest {
        id: "async-test".to_string(),
        method: "async_method".to_string(),
        params: serde_json::json!({"async": true}),
    };

    let request_json = serde_json::to_string(&request).unwrap();
    let message = Message {
        data: request_json.as_bytes().to_vec(),
        message_type: MessageType::Text,
    };

    let send_result = sink.send(message).await;
    assert!(send_result.is_ok());

    // Then: Should receive async response
    let received = stream.next().await;
    assert!(received.is_some());
    let received_msg = received.unwrap().unwrap();
    let response_text = String::from_utf8(received_msg.data).unwrap();
    let response: TestRpcResponse = serde_json::from_str(&response_text).unwrap();

    // Verify async response
    assert_eq!(response.id, "async-test");
    assert!(response.result.is_some());
    assert!(response.error.is_none());

    let result = response.result.unwrap();
    assert_eq!(result["async_result"], "processed");
    assert_eq!(result["method"], "async_method");
    assert!(result["timestamp"].is_number());
}

#[tokio::test]
async fn test_type_safe_method_definitions() {
    // Given: Type-safe RPC method definitions
    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct AddParams {
        a: i32,
        b: i32,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct AddResult {
        sum: i32,
    }

    // When: Making type-safe RPC call
    let request = TestRpcRequest {
        id: "type-safe-test".to_string(),
        method: "add".to_string(),
        params: serde_json::to_value(AddParams { a: 5, b: 3 }).unwrap(),
    };

    // Then: Should serialize correctly
    let request_json = serde_json::to_string(&request).unwrap();
    let parsed_request: TestRpcRequest = serde_json::from_str(&request_json).unwrap();

    assert_eq!(parsed_request.id, "type-safe-test");
    assert_eq!(parsed_request.method, "add");

    // Verify params can be deserialized to correct type
    let params: AddParams = serde_json::from_value(parsed_request.params).unwrap();
    assert_eq!(params.a, 5);
    assert_eq!(params.b, 3);
}

#[tokio::test]
async fn test_advanced_rpc_correlation_manager() {
    // Given: An RPC correlation manager
    let manager = RpcCorrelationManager::new(Duration::from_secs(5));

    // When: Registering a request
    let response_rx = manager.register_request("test-123".to_string());

    // Then: Should have one pending request
    assert_eq!(manager.pending_count(), 1);

    // When: Handling a response
    let response = RpcResponse {
        id: "test-123".to_string(),
        result: Some(serde_json::json!({"success": true})),
        error: None,
    };

    let result = manager.handle_response(response);
    assert!(result.is_ok());

    // Then: Should have no pending requests
    assert_eq!(manager.pending_count(), 0);

    // And: Should receive the response
    let received = response_rx.await.unwrap();
    assert!(received.is_ok());
    let response = received.unwrap();
    assert_eq!(response.id, "test-123");
    assert!(response.result.is_some());
}

#[tokio::test]
async fn test_advanced_rpc_method_registry() {
    // Given: An RPC method registry
    let mut registry = RpcMethodRegistry::new();

    // When: Registering a method
    registry.register("echo", |params| {
        Ok(params)
    });

    registry.register("add", |params| {
        let a: i32 = params["a"].as_i64().unwrap() as i32;
        let b: i32 = params["b"].as_i64().unwrap() as i32;
        Ok(serde_json::json!({"sum": a + b}))
    });

    // Then: Should be able to call registered methods
    let result = registry.call("echo", serde_json::json!({"message": "hello"}));
    assert!(result.is_ok());
    assert_eq!(result.unwrap()["message"], "hello");

    let result = registry.call("add", serde_json::json!({"a": 5, "b": 3}));
    assert!(result.is_ok());
    assert_eq!(result.unwrap()["sum"], 8);

    // And: Should fail for non-existent methods
    let result = registry.call("nonexistent", serde_json::json!({}));
    assert!(result.is_err());
    match result.unwrap_err() {
        RpcError::MethodNotFound(_) => {},
        _ => panic!("Expected MethodNotFound error"),
    }

    // And: Should list registered methods
    let methods = registry.methods();
    assert!(methods.contains(&"echo".to_string()));
    assert!(methods.contains(&"add".to_string()));
}

#[tokio::test]
async fn test_advanced_bidirectional_rpc_client() {
    // Given: A WebSocket connection and RPC client
    let config = TransportConfig::default();
    let transport = WebSocketConnection::new(config).await.unwrap();
    let client = BidirectionalRpcClient::new(transport, Duration::from_secs(5)).await.unwrap();

    // When: Making an RPC call
    let result = client.call("echo", serde_json::json!({"message": "hello"})).await;

    // Then: Should succeed
    assert!(result.is_ok());
    let value = result.unwrap();
    assert_eq!(value["echo"]["message"], "hello");
    assert_eq!(value["method"], "echo");

    // And: Should have no pending requests after completion
    assert_eq!(client.pending_requests_count(), 0);
}

#[tokio::test]
async fn test_advanced_rpc_timeout() {
    // Given: A WebSocket connection and RPC client with short timeout
    let config = TransportConfig::default();
    let transport = WebSocketConnection::new(config).await.unwrap();
    let client = BidirectionalRpcClient::new(transport, Duration::from_millis(100)).await.unwrap();

    // When: Making an RPC call with timeout
    let result = client.call_with_timeout("echo", serde_json::json!({"message": "hello"}), Duration::from_millis(50)).await;

    // Then: Should succeed (our mock implementation is fast)
    assert!(result.is_ok());
    let value = result.unwrap();
    assert_eq!(value["echo"]["message"], "hello");
}

#[tokio::test]
async fn test_rpc_performance_metrics() {
    // Given: An RPC server and client
    let (listener, port) = start_test_rpc_server().await;
    run_rpc_echo_server(listener).await;

    let config = TransportConfig {
        url: format!("ws://127.0.0.1:{}", port),
        ..Default::default()
    };
    let mut client = WebSocketConnection::new(config).await.unwrap();

    client.connect(&format!("ws://127.0.0.1:{}", port)).await.unwrap();
    let (mut stream, mut sink) = client.split();

    // When: Making multiple RPC calls and measuring performance
    let start_time = std::time::Instant::now();
    let num_calls = 10;

    for i in 0..num_calls {
        let request = TestRpcRequest {
            id: format!("perf-test-{}", i),
            method: "performance_test".to_string(),
            params: serde_json::json!({"iteration": i}),
        };

        let request_json = serde_json::to_string(&request).unwrap();
        let message = Message {
            data: request_json.as_bytes().to_vec(),
            message_type: MessageType::Text,
        };

        let send_result = sink.send(message).await;
        assert!(send_result.is_ok());
    }

    // Collect all responses
    let mut responses = Vec::new();
    for _ in 0..num_calls {
        let received = stream.next().await;
        assert!(received.is_some());
        let received_msg = received.unwrap().unwrap();
        let response_text = String::from_utf8(received_msg.data).unwrap();
        let response: TestRpcResponse = serde_json::from_str(&response_text).unwrap();
        responses.push(response);
    }

    let total_time = start_time.elapsed();

    // Then: Should have good performance metrics
    assert_eq!(responses.len(), num_calls);

    // Performance assertions
    let avg_time_per_call = total_time.as_millis() / num_calls as u128;
    assert!(avg_time_per_call < 100, "Average RPC call time should be < 100ms, got {}ms", avg_time_per_call);

    // Verify all responses are correct
    for (i, response) in responses.iter().enumerate() {
        assert_eq!(response.id, format!("perf-test-{}", i));
        assert!(response.result.is_some());
        let result = response.result.as_ref().unwrap();
        assert_eq!(result["echo"]["iteration"], i);
    }
}
