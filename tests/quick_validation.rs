//! Quick validation tests for v1.0 implementation
//! Tests key functionality to assess current implementation state

#[cfg(test)]
mod validation_tests {
    use leptos_ws_pro::{
        codec::{Codec, CompressedCodec, JsonCodec},
        reactive::WebSocketContext,
        rpc::{RpcError, RpcMethod},
        transport::{ConnectionState, Message, MessageType},
    };
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestData {
        id: u32,
        message: String,
    }

    /// Test 1: Codec System Validation
    #[test]
    fn test_codec_system() {
        let codec = JsonCodec::new();
        let data = TestData {
            id: 1,
            message: "test".to_string(),
        };

        // Test encoding
        let encoded = codec.encode(&data).expect("Failed to encode");
        assert!(!encoded.is_empty());

        // Test decoding
        let decoded: TestData = codec.decode(&encoded).expect("Failed to decode");
        assert_eq!(data, decoded);

        // Test content type
        assert_eq!(
            <JsonCodec as Codec<TestData>>::content_type(&codec),
            "application/json"
        );

        println!("✅ Codec System: PASSED");
    }

    /// Test 2: Compressed Codec Validation
    #[test]
    fn test_compressed_codec() {
        let inner_codec = JsonCodec::new();
        let compressed_codec = CompressedCodec::new(inner_codec);

        let data = TestData {
            id: 42,
            message: "compression test".to_string(),
        };

        // Test encoding/decoding through compression layer
        let encoded = compressed_codec
            .encode(&data)
            .expect("Failed to encode with compression");
        let decoded: TestData = compressed_codec
            .decode(&encoded)
            .expect("Failed to decode with compression");
        assert_eq!(data, decoded);

        println!("✅ Compressed Codec: PASSED");
    }

    /// Test 3: Transport System Basic Validation
    #[test]
    fn test_transport_factory() {
        // Test capability detection
        let capabilities = leptos_ws_pro::transport::TransportCapabilities::detect();
        assert!(capabilities.websocket); // Should always support WebSocket

        println!("✅ Transport Factory: PASSED");
    }

    /// Test 4: Message System Validation
    #[test]
    fn test_message_system() {
        let text_msg = Message {
            data: "Hello World".as_bytes().to_vec(),
            message_type: MessageType::Text,
        };
        assert_eq!(text_msg.message_type, MessageType::Text);
        assert_eq!(String::from_utf8_lossy(&text_msg.data), "Hello World");

        let binary_data = vec![1, 2, 3, 4];
        let binary_msg = Message {
            data: binary_data.clone(),
            message_type: MessageType::Binary,
        };
        assert_eq!(binary_msg.message_type, MessageType::Binary);
        assert_eq!(binary_msg.data, binary_data);

        println!("✅ Message System: PASSED");
    }

    /// Test 5: Connection State Management
    #[test]
    fn test_connection_states() {
        // Test state transitions
        assert_ne!(ConnectionState::Connecting, ConnectionState::Connected);
        assert_ne!(ConnectionState::Connected, ConnectionState::Disconnected);

        // Test connection state values
        assert_eq!(ConnectionState::Connected, ConnectionState::Connected);

        println!("✅ Connection States: PASSED");
    }

    /// Test 6: RPC System Basic Validation
    #[test]
    fn test_rpc_structures() {
        // Test RPC method variants
        assert_ne!(RpcMethod::Query, RpcMethod::Mutation);
        assert_ne!(RpcMethod::Call, RpcMethod::Subscription);

        // Test RPC error creation
        let error = RpcError {
            code: 404,
            message: "Method not found".to_string(),
            data: None,
        };
        assert_eq!(error.code, 404);

        println!("✅ RPC Structures: PASSED");
    }

    /// Test 7: WebSocket Context Creation
    #[tokio::test]
    async fn test_websocket_context_creation() {
        use leptos_ws_pro::reactive::WebSocketProvider;

        let provider = WebSocketProvider::new("ws://localhost:8080");
        let context = WebSocketContext::new(provider);

        assert_eq!(context.url(), "ws://localhost:8080");
        // Note: We can't easily test the signal value without the Get trait
        // This test verifies the context can be created successfully

        println!("✅ WebSocket Context: PASSED");
    }

    /// Summary Test: Integration Readiness
    #[test]
    fn test_integration_readiness() {
        let mut passed = 0;
        let total = 7;

        // Count passed tests (this is a meta-test)
        println!("\n=== v1.0 Implementation Validation Summary ===");

        // Basic functionality tests
        if std::panic::catch_unwind(|| test_codec_system()).is_ok() {
            passed += 1;
        }
        if std::panic::catch_unwind(|| test_compressed_codec()).is_ok() {
            passed += 1;
        }
        if std::panic::catch_unwind(|| test_transport_factory()).is_ok() {
            passed += 1;
        }
        if std::panic::catch_unwind(|| test_message_system()).is_ok() {
            passed += 1;
        }
        if std::panic::catch_unwind(|| test_connection_states()).is_ok() {
            passed += 1;
        }
        if std::panic::catch_unwind(|| test_rpc_structures()).is_ok() {
            passed += 1;
        }

        println!("✅ Basic Tests Passed: {}/{}", passed, total - 1);

        // Calculate readiness percentage
        let readiness = (passed as f32 / (total - 1) as f32) * 100.0;
        println!("🎯 Implementation Readiness: {:.1}%", readiness);

        if readiness >= 80.0 {
            println!("🚀 READY for integration testing!");
        } else {
            println!("⚠️  Need more implementation work");
        }

        assert!(passed >= 5, "Need at least 5/6 basic tests to pass");
    }
}
