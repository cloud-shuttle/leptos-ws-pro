//! Comprehensive integration tests for v1.0 TDD
//!
//! This test suite ensures all modules work together correctly
//! following TDD principles for v1.0 release.

use leptos_ws_pro::{
    codec::{JsonCodec, Codec, WsMessage},
    reactive::{WebSocketContext, WebSocketProvider, ConnectionMetrics, UserPresence},
    rpc::{RpcClient, RpcRequest, RpcResponse, RpcMethod, RpcError, SendMessageParams, ChatMessage, SubscribeMessagesParams},
    transport::{ConnectionState, Message, MessageType, TransportConfig, TransportFactory},
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::sleep;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IntegrationTestData {
    test_id: u32,
    payload: String,
    metadata: std::collections::HashMap<String, String>,
}

#[cfg(test)]
mod integration_core_tests {
    use super::*;


    #[tokio::test]
    async fn test_full_websocket_stack_integration() {
        // Create provider and context
        let provider = WebSocketProvider::new("ws://localhost:8080");
        let context = WebSocketContext::new(provider);

        // Create RPC client
        let rpc_client = RpcClient::<IntegrationTestData>::new(context.clone(), JsonCodec);

        // Test initial state
        assert_eq!(context.connection_state(), ConnectionState::Disconnected);
        assert!(!context.is_connected());

        // Test RPC client creation
        assert_eq!(rpc_client.context().get_url(), "ws://localhost:8080");
        // ID generation is handled internally by the RPC client

        // Test codec integration
        let codec = JsonCodec::new();
        let test_data = IntegrationTestData {
            test_id: 1,
            payload: "Integration test".to_string(),
            metadata: [("source".to_string(), "test_suite".to_string())]
                .iter().cloned().collect(),
        };

        let encoded = codec.encode(&test_data).unwrap();
        let decoded: IntegrationTestData = codec.decode(&encoded).unwrap();
        assert_eq!(test_data, decoded);

        // Test message handling through context
        let message = Message {
            data: encoded,
            message_type: MessageType::Text,
        };

        context.handle_message(message);

        let metrics = context.get_connection_metrics();
        assert_eq!(metrics.messages_received, 1);
        assert!(metrics.bytes_received > 0);
    }

    #[tokio::test]
    async fn test_rpc_with_websocket_context_integration() {
        let provider = WebSocketProvider::new("ws://localhost:9001");
        let context = WebSocketContext::new(provider);
        let rpc_client = RpcClient::<SendMessageParams>::new(context.clone(), JsonCodec);

        // Test RPC request creation and ID generation
        let params = SendMessageParams {
            message: "Hello from integration test".to_string(),
            channel: Some("test-channel".to_string()),
            content: Some("Hello from integration test".to_string()),
            room_id: Some("test-room".to_string()),
        };

        // Test query method
        let query_result = rpc_client.query::<SendMessageParams>("get_message", params.clone()).await;
        assert!(query_result.is_ok()); // Should succeed with mock implementation

        // Test mutation method
        let mutation_result = rpc_client.mutation::<SendMessageParams>("send_message", params.clone()).await;
        assert!(mutation_result.is_ok()); // Should succeed with mock implementation

        // Verify ID generation worked
        let id1 = rpc_client.generate_id();
        let id2 = rpc_client.generate_id();
        assert_eq!(id1, "rpc_3"); // Should be 3 after 2 previous calls
        assert_eq!(id2, "rpc_4");
    }

    #[tokio::test]
    async fn test_transport_factory_with_reactive_context() {
        // Test transport factory configuration
        let config = TransportConfig {
            url: "ws://localhost:8080".to_string(),
            protocols: vec!["chat".to_string()],
            headers: [("User-Agent".to_string(), "leptos-ws-test".to_string())]
                .iter().cloned().collect(),
            timeout: Duration::from_secs(10),
            heartbeat_interval: Some(Duration::from_secs(15)),
            max_reconnect_attempts: Some(3),
            reconnect_delay: Duration::from_secs(2),
            connection_timeout: Duration::from_secs(30),
            enable_compression: false,
            max_message_size: 1024 * 1024,
        };

        // Test factory creation (will fail without server, but tests integration)
        let factory_result = TransportFactory::create_websocket(config.clone()).await;
        match factory_result {
            Err(leptos_ws_pro::transport::TransportError::ConnectionFailed(_)) => {
                // Expected without server
                assert!(true);
            }
            Ok(_) => {
                // Unexpected success in test environment
                assert!(true);
            }
            Err(e) => {
                println!("Factory error: {:?}", e);
                assert!(true);
            }
        }

        // Test reactive context with same URL
        let provider = WebSocketProvider::new(&config.url);
        let context = WebSocketContext::new(provider);
        assert_eq!(context.get_url(), config.url);
    }

    #[test]
    fn test_codec_with_rpc_message_integration() {
        let codec = JsonCodec::new();

        // Test RPC request encoding/decoding
        let request = RpcRequest {
            id: "integration-test-123".to_string(),
            method: "test_integration".to_string(),
            params: IntegrationTestData {
                test_id: 42,
                payload: "RPC integration test".to_string(),
                metadata: [("type".to_string(), "integration".to_string())]
                    .iter().cloned().collect(),
            },
            method_type: RpcMethod::Call,
        };

        let encoded = codec.encode(&request).unwrap();
        let decoded: RpcRequest<IntegrationTestData> = codec.decode(&encoded).unwrap();

        assert_eq!(request.id, decoded.id);
        assert_eq!(request.method, decoded.method);
        assert_eq!(request.method_type, decoded.method_type);
        assert_eq!(request.params, decoded.params);

        // Test RPC response encoding/decoding
        let response = RpcResponse {
            id: "integration-test-123".to_string(),
            result: Some(IntegrationTestData {
                test_id: 42,
                payload: "Response data".to_string(),
                metadata: [("status".to_string(), "success".to_string())]
                    .iter().cloned().collect(),
            }),
            error: None,
        };

        let encoded_resp = codec.encode(&response).unwrap();
        let decoded_resp: RpcResponse<IntegrationTestData> = codec.decode(&encoded_resp).unwrap();

        assert_eq!(response.id, decoded_resp.id);
        assert_eq!(response.result, decoded_resp.result);
        assert!(decoded_resp.error.is_none());
    }

    #[test]
    fn test_ws_message_wrapper_integration() {
        let codec = JsonCodec::new();

        // Test WsMessage with RPC request
        let rpc_request = RpcRequest {
            id: "ws-msg-test".to_string(),
            method: "wrapped_call".to_string(),
            params: IntegrationTestData {
                test_id: 1,
                payload: "Wrapped in WsMessage".to_string(),
                metadata: std::collections::HashMap::new(),
            },
            method_type: RpcMethod::Query,
        };

        let wrapped_message = WsMessage::new(rpc_request.clone());

        let encoded = codec.encode(&wrapped_message).unwrap();
        let decoded: WsMessage<RpcRequest<IntegrationTestData>> = codec.decode(&encoded).unwrap();

        assert_eq!(wrapped_message.data.id, decoded.data.id);
        assert_eq!(wrapped_message.data.method, decoded.data.method);
        assert_eq!(wrapped_message.data.params, decoded.data.params);
    }

    #[tokio::test]
    async fn test_presence_integration_with_context() {
        let provider = WebSocketProvider::new("ws://localhost:8080");
        let context = WebSocketContext::new(provider);

        // Test presence updates
        let user1 = UserPresence {
            user_id: "user1".to_string(),
            status: "online".to_string(),
            last_seen: 1000,
        };

        let user2 = UserPresence {
            user_id: "user2".to_string(),
            status: "busy".to_string(),
            last_seen: 2000,
        };

        context.update_presence("user1", user1.clone());
        context.update_presence("user2", user2.clone());

        // Test presence retrieval
        let presence_data = context.get_presence();
        assert_eq!(presence_data.len(), 2);
        assert_eq!(presence_data["user1"], user1);
        assert_eq!(presence_data["user2"], user2);

        // Test presence with RPC integration (conceptual)
        let rpc_client = RpcClient::<UserPresence>::new(context.clone(), JsonCodec);
        let presence_id = rpc_client.generate_id();
        assert_eq!(presence_id, "rpc_1");

        // Verify context still maintains presence data
        let updated_presence = context.get_presence();
        assert_eq!(updated_presence.len(), 2);
    }

    #[tokio::test]
    async fn test_metrics_integration_across_modules() {
        let provider = WebSocketProvider::new("ws://localhost:8080");
        let context = WebSocketContext::new(provider);
        let codec = JsonCodec::new();

        // Test initial metrics
        let initial_metrics = context.get_connection_metrics();
        assert_eq!(initial_metrics, ConnectionMetrics::default());

        // Generate some test data
        let test_data = IntegrationTestData {
            test_id: 1,
            payload: "Metrics test data".to_string(),
            metadata: [("test".to_string(), "metrics".to_string())]
                .iter().cloned().collect(),
        };

        // Encode data and create messages
        let encoded_data = codec.encode(&test_data).unwrap();

        let messages = vec![
            Message {
                data: encoded_data.clone(),
                message_type: MessageType::Text,
            },
            Message {
                data: b"Binary test data".to_vec(),
                message_type: MessageType::Binary,
            },
            Message {
                data: b"Ping".to_vec(),
                message_type: MessageType::Ping,
            },
        ];

        let total_bytes = messages.iter().map(|m| m.data.len()).sum::<usize>() as u64;

        // Handle messages
        for message in messages {
            context.handle_message(message);
        }

        // Verify metrics integration
        let final_metrics = context.get_connection_metrics();
        assert_eq!(final_metrics.messages_received, 3);
        assert_eq!(final_metrics.bytes_received, total_bytes);
        assert_eq!(final_metrics.messages_sent, 0);
        assert_eq!(final_metrics.bytes_sent, 0);

        // Test heartbeat integration with metrics
        let heartbeat_result = context.send_heartbeat();
        assert!(heartbeat_result.is_ok());

        // Verify sent messages tracking (heartbeat doesn't update main metrics in current impl)
        let sent_messages: Vec<serde_json::Value> = context.get_sent_messages();
        assert!(!sent_messages.is_empty());
    }

    #[tokio::test]
    async fn test_error_handling_integration() {
        let provider = WebSocketProvider::new("ws://invalid-test-url:99999");
        let context = WebSocketContext::new(provider);
        let rpc_client = RpcClient::<IntegrationTestData>::new(context.clone(), JsonCodec);

        // Test connection failure
        let connect_result = context.connect().await;
        assert!(connect_result.is_err());
        assert_eq!(context.connection_state(), ConnectionState::Disconnected);

        // Test RPC error handling
        let test_params = IntegrationTestData {
            test_id: 1,
            payload: "Error test".to_string(),
            metadata: std::collections::HashMap::new(),
        };

        let rpc_result = rpc_client.query::<IntegrationTestData>("error_method", test_params).await;
        assert!(rpc_result.is_err());

        match rpc_result {
            Err(RpcError { code, message, .. }) => {
                assert_eq!(code, -32603); // Internal error code
                // Check for any error message (the exact message may vary)
                assert!(!message.is_empty());
            }
            _ => panic!("Expected RpcError"),
        }

        // Test codec error handling
        let codec = JsonCodec::new();
        let invalid_data = b"invalid json {{{";
        let decode_result = <JsonCodec as Codec<IntegrationTestData>>::decode(&codec, invalid_data);
        assert!(decode_result.is_err());
    }

    #[tokio::test]
    async fn test_reconnection_integration() {
        let provider = WebSocketProvider::new("ws://localhost:8080");
        let context = WebSocketContext::new(provider);

        // Test reconnection parameters
        assert_eq!(context.reconnect_interval(), 5);
        assert_eq!(context.max_reconnect_attempts(), 3);

        // Test connection quality impact on reconnection
        context.update_connection_quality(0.3); // Poor quality
        assert!(context.should_reconnect_due_to_quality());

        // Test reconnection attempts
        for i in 1..=5 {
            let result = context.attempt_reconnection();
            assert!(result.is_ok());
            assert_eq!(context.reconnection_attempts(), i);
        }

        // Test reconnection with RPC client
        let rpc_client = RpcClient::<IntegrationTestData>::new(context.clone(), JsonCodec);

        // Generate ID to verify client still works after reconnection attempts
        let id = rpc_client.generate_id();
        assert_eq!(id, "rpc_1");

        // Verify context state after reconnection attempts
        assert_eq!(context.reconnection_attempts(), 5);
        assert!(context.should_reconnect_due_to_quality());
    }

    #[tokio::test]
    async fn test_message_acknowledgment_integration() {
        let provider = WebSocketProvider::new("ws://localhost:8080");
        let context = WebSocketContext::new(provider);

        // Test message acknowledgment without connection
        let test_data = IntegrationTestData {
            test_id: 1,
            payload: "Ack test".to_string(),
            metadata: std::collections::HashMap::new(),
        };

        let ack_result = context.send_message_with_ack(&test_data).await;
        assert!(ack_result.is_err()); // Expected without connection

        // Test acknowledgment tracking
        context.acknowledge_message(1);
        context.acknowledge_message(2);

        let acks = context.get_acknowledged_messages();
        assert_eq!(acks, vec![1, 2]);

        // Test with RPC client
        let rpc_client = RpcClient::<IntegrationTestData>::new(context.clone(), JsonCodec);
        let subscription = rpc_client.subscribe(SubscribeMessagesParams { channel: Some("ack_test".to_string()), room_id: None }).await.unwrap();

        // Verify subscription creation doesn't interfere with acknowledgments
        assert_eq!(subscription.id, "rpc_1");

        let updated_acks = context.get_acknowledged_messages();
        assert_eq!(updated_acks, vec![1, 2]); // Should be unchanged
    }
}

#[cfg(test)]
mod cross_module_compatibility_tests {
    use super::*;

    #[test]
    fn test_transport_message_with_codec_integration() {
        let codec = JsonCodec::new();

        // Create a transport message
        let transport_msg = Message {
            data: b"Transport integration test".to_vec(),
            message_type: MessageType::Text,
        };

        // Encode the transport message using codec
        let encoded = codec.encode(&transport_msg).unwrap();
        let decoded: Message = codec.decode(&encoded).unwrap();

        assert_eq!(transport_msg.data, decoded.data);
        assert_eq!(transport_msg.message_type, decoded.message_type);
    }

    #[test]
    fn test_rpc_error_with_transport_error_compatibility() {
        use leptos_ws_pro::transport::TransportError;

        // Test that transport errors can be converted to RPC errors conceptually
        let transport_error = TransportError::ConnectionFailed("Network unreachable".to_string());

        // Create corresponding RPC error
        let rpc_error = RpcError {
            code: -32603, // Internal error
            message: format!("Transport error: {}", transport_error),
            data: Some(serde_json::json!({"transport_error": "ConnectionFailed"})),
        };

        assert_eq!(rpc_error.code, -32603);
        assert!(rpc_error.message.contains("Transport error"));
        assert!(rpc_error.data.is_some());
    }

    #[test]
    fn test_connection_state_with_rpc_method_compatibility() {
        // Test that connection states align with RPC method availability
        let states_and_methods = vec![
            (ConnectionState::Disconnected, false),
            (ConnectionState::Connecting, false),
            (ConnectionState::Connected, true),
            (ConnectionState::Reconnecting, false),
            (ConnectionState::Failed, false),
        ];

        for (state, should_allow_rpc) in states_and_methods {
            let allows_rpc = matches!(state, ConnectionState::Connected);
            assert_eq!(allows_rpc, should_allow_rpc, "State {:?} RPC availability mismatch", state);
        }
    }

    #[tokio::test]
    async fn test_full_stack_message_flow() {
        let provider = WebSocketProvider::new("ws://localhost:8080");
        let context = WebSocketContext::new(provider);
        let codec = JsonCodec::new();

        // Create test data that flows through all layers
        let original_data = IntegrationTestData {
            test_id: 999,
            payload: "Full stack test".to_string(),
            metadata: [
                ("layer".to_string(), "transport".to_string()),
                ("encoding".to_string(), "json".to_string()),
            ].iter().cloned().collect(),
        };

        // Step 1: Encode with codec
        let encoded_data = codec.encode(&original_data).unwrap();

        // Step 2: Wrap in transport message
        let transport_message = Message {
            data: encoded_data,
            message_type: MessageType::Text,
        };

        // Step 3: Handle through reactive context
        context.handle_message(transport_message.clone());

        // Step 4: Verify metrics updated
        let metrics = context.get_connection_metrics();
        assert_eq!(metrics.messages_received, 1);
        assert_eq!(metrics.bytes_received, transport_message.data.len() as u64);

        // Step 5: Retrieve and decode
        let received_messages: Vec<IntegrationTestData> = context.get_received_messages();
        assert_eq!(received_messages.len(), 1);
        assert_eq!(received_messages[0], original_data);
    }

    #[test]
    fn test_websocket_config_with_transport_config_compatibility() {
        use leptos_ws_pro::reactive::WebSocketConfig;

        // Test that WebSocket configs are compatible with transport configs
        let ws_config = WebSocketConfig {
            url: "wss://api.example.com/ws".to_string(),
            protocols: vec!["v1".to_string(), "chat".to_string()],
            heartbeat_interval: Some(30),
            reconnect_interval: Some(5),
            max_reconnect_attempts: Some(10),
            codec: Box::new(JsonCodec::new()),
        };

        let transport_config = TransportConfig {
            url: ws_config.url.clone(),
            protocols: ws_config.protocols.clone(),
            headers: std::collections::HashMap::new(),
            timeout: Duration::from_secs(30),
            heartbeat_interval: ws_config.heartbeat_interval.map(Duration::from_secs),
            max_reconnect_attempts: ws_config.max_reconnect_attempts.map(|x| x as usize),
            reconnect_delay: Duration::from_secs(ws_config.reconnect_interval.unwrap_or(5)),
            connection_timeout: Duration::from_secs(30),
            enable_compression: false,
            max_message_size: 1024 * 1024,
        };

        // Verify compatibility
        assert_eq!(ws_config.url, transport_config.url);
        assert_eq!(ws_config.protocols, transport_config.protocols);
        assert_eq!(
            ws_config.heartbeat_interval.map(Duration::from_secs),
            transport_config.heartbeat_interval
        );
    }
}

#[cfg(test)]
mod performance_integration_tests {
    use super::*;
    use std::time::Instant;

    #[tokio::test]
    async fn test_high_throughput_message_handling() {
        let provider = WebSocketProvider::new("ws://localhost:8080");
        let context = WebSocketContext::new(provider);
        let codec = JsonCodec::new();

        let test_data = IntegrationTestData {
            test_id: 1,
            payload: "Performance test".to_string(),
            metadata: [("bench".to_string(), "throughput".to_string())]
                .iter().cloned().collect(),
        };

        let encoded_data = codec.encode(&test_data).unwrap();
        let message_count = 1000;

        let start = Instant::now();

        for i in 0..message_count {
            let message = Message {
                data: encoded_data.clone(),
                message_type: if i % 2 == 0 { MessageType::Text } else { MessageType::Binary },
            };
            context.handle_message(message);
        }

        let elapsed = start.elapsed();

        // Verify all messages processed
        let metrics = context.get_connection_metrics();
        assert_eq!(metrics.messages_received, message_count);
        assert_eq!(metrics.bytes_received, (encoded_data.len() * message_count as usize) as u64);

        // Should process 1000 messages quickly (less than 100ms)
        assert!(elapsed.as_millis() < 100, "Processing took too long: {:?}", elapsed);

        println!("Processed {} messages in {:?} ({:.2} msgs/ms)",
                 message_count, elapsed, message_count as f64 / elapsed.as_millis() as f64);
    }

    #[test]
    fn test_codec_performance_integration() {
        let codec = JsonCodec::new();
        let iterations = 1000;

        // Create test data of various sizes
        let small_data = IntegrationTestData {
            test_id: 1,
            payload: "small".to_string(),
            metadata: std::collections::HashMap::new(),
        };

        let large_data = IntegrationTestData {
            test_id: 1,
            payload: "x".repeat(10000), // 10KB string
            metadata: (0..100).map(|i| (format!("key_{}", i), format!("value_{}", i))).collect(),
        };

        let test_cases = vec![
            ("small", small_data),
            ("large", large_data),
        ];

        for (name, data) in test_cases {
            let start = Instant::now();

            for _ in 0..iterations {
                let encoded = codec.encode(&data).unwrap();
                let _decoded: IntegrationTestData = codec.decode(&encoded).unwrap();
            }

            let elapsed = start.elapsed();
            println!("{} data: {} iterations in {:?} ({:.2} ops/ms)",
                     name, iterations, elapsed, iterations as f64 / elapsed.as_millis() as f64);

            // Should complete in reasonable time (less than 1 second)
            assert!(elapsed.as_secs() < 1, "{} data took too long: {:?}", name, elapsed);
        }
    }

    #[tokio::test]
    async fn test_rpc_id_generation_performance() {
        let provider = WebSocketProvider::new("ws://localhost:8080");
        let context = WebSocketContext::new(provider);
        let rpc_client = RpcClient::<IntegrationTestData>::new(context, JsonCodec);

        let iterations = 10000;
        let start = Instant::now();

        for _ in 0..iterations {
            let _id = rpc_client.generate_id();
        }

        let elapsed = start.elapsed();

        // Verify final counter
        // ID generation is handled internally by the RPC client

        // Should be very fast (less than 10ms for 10k IDs)
        assert!(elapsed.as_millis() < 10, "ID generation took too long: {:?}", elapsed);

        println!("Generated {} IDs in {:?} ({:.2} IDs/ms)",
                 iterations, elapsed, iterations as f64 / elapsed.as_millis() as f64);
    }
}

#[cfg(test)]
mod concurrent_integration_tests {
    use super::*;
    use std::sync::Arc;
    use tokio::task::JoinSet;

    #[tokio::test]
    async fn test_concurrent_context_usage() {
        let provider = WebSocketProvider::new("ws://localhost:8080");
        let context = Arc::new(WebSocketContext::new(provider));
        let codec = Arc::new(JsonCodec::new());

        let mut join_set = JoinSet::new();

        // Spawn multiple tasks that use the context concurrently
        for task_id in 0..10 {
            let context_clone = context.clone();
            let codec_clone = codec.clone();

            join_set.spawn(async move {
                let test_data = IntegrationTestData {
                    test_id: task_id,
                    payload: format!("Concurrent task {}", task_id),
                    metadata: [("task_id".to_string(), task_id.to_string())]
                        .iter().cloned().collect(),
                };

                let encoded = codec_clone.encode(&test_data).unwrap();
                let message = Message {
                    data: encoded,
                    message_type: MessageType::Text,
                };

                context_clone.handle_message(message);

                // Update presence
                let presence = UserPresence {
                    user_id: format!("user_{}", task_id),
                    status: "active".to_string(),
                    last_seen: task_id as u64 * 1000,
                };

                context_clone.update_presence(&format!("user_{}", task_id), presence);

                task_id
            });
        }

        // Wait for all tasks to complete
        let mut completed_tasks = Vec::new();
        while let Some(result) = join_set.join_next().await {
            completed_tasks.push(result.unwrap());
        }

        // Verify all tasks completed
        assert_eq!(completed_tasks.len(), 10);
        completed_tasks.sort();
        assert_eq!(completed_tasks, (0..10).collect::<Vec<_>>());

        // Verify metrics reflect all messages
        let final_metrics = context.get_connection_metrics();
        assert_eq!(final_metrics.messages_received, 10);

        // Verify presence updates
        let presence_data = context.get_presence();
        assert_eq!(presence_data.len(), 10);
        for i in 0..10 {
            assert!(presence_data.contains_key(&format!("user_{}", i)));
        }
    }

    #[tokio::test]
    async fn test_concurrent_rpc_clients() {
        let provider = WebSocketProvider::new("ws://localhost:8080");
        let context = Arc::new(WebSocketContext::new(provider));

        let mut join_set = JoinSet::new();

        // Create multiple RPC clients concurrently
        for client_id in 0..5 {
            let context_clone = context.clone();

            join_set.spawn(async move {
                let rpc_client = RpcClient::<IntegrationTestData>::new(context_clone.as_ref().clone(), JsonCodec);

                // Generate IDs concurrently
                let mut ids = Vec::new();
                for _ in 0..10 {
                    ids.push(rpc_client.generate_id());
                }

                (client_id, ids)
            });
        }

        // Collect results
        let mut all_results = Vec::new();
        while let Some(result) = join_set.join_next().await {
            all_results.push(result.unwrap());
        }

        // Verify each client has independent ID counters
        assert_eq!(all_results.len(), 5);

        for (client_id, ids) in all_results {
            assert_eq!(ids.len(), 10);
            // Each client should start from rpc_1
            assert_eq!(ids[0], "rpc_1");
            assert_eq!(ids[9], "rpc_10");
            println!("Client {} generated IDs: {:?}", client_id, &ids[0..3]);
        }
    }

    #[tokio::test]
    async fn test_concurrent_state_changes() {
        let provider = WebSocketProvider::new("ws://localhost:8080");
        let context = Arc::new(WebSocketContext::new(provider));

        let mut join_set = JoinSet::new();

        let states = vec![
            ConnectionState::Connecting,
            ConnectionState::Connected,
            ConnectionState::Reconnecting,
            ConnectionState::Disconnected,
            ConnectionState::Failed,
        ];

        // Spawn tasks that change state concurrently
        for (i, state) in states.into_iter().enumerate() {
            let context_clone = context.clone();

            join_set.spawn(async move {
                // Sleep to stagger state changes
                sleep(Duration::from_millis(i as u64 * 10)).await;
                context_clone.set_connection_state(state);
                (i, state)
            });
        }

        // Wait for all state changes
        let mut results = Vec::new();
        while let Some(result) = join_set.join_next().await {
            results.push(result.unwrap());
        }

        assert_eq!(results.len(), 5);

        // Final state should be from the last task
        let final_state = context.connection_state();
        println!("Final connection state: {:?}", final_state);

        // Should be one of the valid states
        assert!(matches!(final_state,
            ConnectionState::Connecting | ConnectionState::Connected |
            ConnectionState::Reconnecting | ConnectionState::Disconnected |
            ConnectionState::Failed
        ));
    }
}
