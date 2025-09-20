//! Additional tests to improve code coverage
//!
//! These tests target uncovered code paths identified by tarpaulin

use leptos_ws_pro::error_handling::circuit_breaker::CircuitBreakerState;
use leptos_ws_pro::{
    codec::{Codec, HybridCodec, JsonCodec, RkyvCodec, WsMessage},
    error_handling::{CircuitBreaker, ErrorContext, ErrorRecoveryHandler},
    performance::{ConnectionPool, MessageBatcher, MessageCache, PerformanceManager},
    rpc::correlation::RpcCorrelationManager,
    security::{CsrfProtector, InputValidator, RateLimiter, ThreatDetector, TokenBucket},
    transport::{Message, MessageType, TransportCapabilities, TransportConfig},
    zero_copy::{MessageBatch, ZeroCopyBuffer, ZeroCopyCodec},
};

use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct TestData {
    id: u32,
    name: String,
    value: f64,
}

#[cfg(test)]
mod coverage_tests {
    use super::*;

    #[test]
    fn test_error_context_creation() {
        let context = ErrorContext::new("test_error", "test_operation");
        assert_eq!(context.error_type, "test_error");
        assert_eq!(context.operation, "test_operation");

        let context_with_state = context.with_connection_state("Connected".to_string());
        assert_eq!(
            context_with_state.connection_state,
            Some("Connected".to_string())
        );

        let context_with_attempt = context_with_state.with_attempt(3);
        assert_eq!(context_with_attempt.attempt_count, Some(3));

        let context_with_trace = context_with_attempt.with_trace_id("trace_123".to_string());
        assert_eq!(context_with_trace.trace_id, Some("trace_123".to_string()));

        let context_with_session = context_with_trace.with_session_id("session_456".to_string());
        assert_eq!(
            context_with_session.session_id,
            Some("session_456".to_string())
        );
    }

    #[test]
    fn test_circuit_breaker_states() {
        let mut breaker = CircuitBreaker::new(5, Duration::from_secs(10));

        // Test initial state
        assert_eq!(breaker.get_state(), CircuitBreakerState::Closed);

        // Test failure recording
        for _ in 0..5 {
            breaker.record_failure();
        }
        assert_eq!(breaker.get_state(), CircuitBreakerState::Open);

        // Test success recording (should not change from Open to Closed directly)
        breaker.record_success();
        assert_eq!(breaker.get_state(), CircuitBreakerState::Open);
    }

    #[tokio::test]
    async fn test_performance_manager() {
        let manager =
            PerformanceManager::new(leptos_ws_pro::performance::PerformanceConfig::default());

        // Test connection pool
        let pool = ConnectionPool::new(leptos_ws_pro::performance::ConnectionPoolConfig::default())
            .await
            .unwrap();
        let connection = pool.get_connection();
        assert!(connection.is_ok());

        // Test message batcher
        let mut batcher = MessageBatcher::new(100, Duration::from_millis(100));
        let message_data = "test".as_bytes().to_vec();
        batcher.add_message(message_data).await.unwrap();
        assert_eq!(batcher.pending_count(), 1);

        // Test message cache
        let mut cache = MessageCache::new(1000, Duration::from_secs(60));
        let data = b"test_data".to_vec();
        cache.set("key1".to_string(), data.clone()).await;
        let retrieved = cache.get("key1").await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap(), data);
    }

    #[test]
    fn test_security_components() {
        // Test rate limiter
        let mut limiter = RateLimiter::new(10, 1);
        for _ in 0..10 {
            assert!(limiter.check_rate_limit("user1".to_string()));
        }
        assert!(!limiter.check_rate_limit("user1".to_string()));

        // Test input validator
        let validator = InputValidator::new(100);
        assert!(validator.validate_input("valid_string".to_string()).is_ok());
        assert!(validator.validate_input("x".repeat(200)).is_err());

        // Test threat detector
        let detector = ThreatDetector::new();
        assert!(!detector.is_threat("normal_message".to_string()));
        assert!(detector.is_threat("<script>alert('xss')</script>".to_string()));

        // Test CSRF protector
        let protector = CsrfProtector::new();
        let token = protector.generate_token();
        assert!(protector.validate_token(&token).is_ok());
        assert!(protector.validate_token("invalid_token").is_err());

        // Test token bucket
        let mut bucket = TokenBucket::new(10, 1);
        for _ in 0..10 {
            assert!(bucket.try_consume(1));
        }
        assert!(!bucket.try_consume(1));
    }

    #[test]
    fn test_transport_config() {
        let config = TransportConfig {
            url: "ws://localhost:8080".to_string(),
            protocols: vec!["chat".to_string()],
            headers: std::collections::HashMap::new(),
            timeout: Duration::from_secs(30),
            heartbeat_interval: Some(Duration::from_secs(15)),
            max_reconnect_attempts: Some(5),
            reconnect_delay: Duration::from_secs(2),
            connection_timeout: Duration::from_secs(10),
            enable_compression: true,
            max_message_size: 1024 * 1024,
        };

        assert_eq!(config.url, "ws://localhost:8080");
        assert_eq!(config.protocols.len(), 1);
        assert_eq!(config.timeout, Duration::from_secs(30));
    }

    #[test]
    fn test_transport_capabilities() {
        let capabilities = TransportCapabilities::detect();
        assert!(capabilities.websocket);
        assert!(capabilities.sse);
        assert!(capabilities.webtransport);
    }

    #[test]
    fn test_message_creation() {
        let text_msg = Message {
            message_type: MessageType::Text,
            data: "Hello, World!".as_bytes().to_vec(),
        };
        assert_eq!(text_msg.message_type, MessageType::Text);
        assert_eq!(text_msg.data, "Hello, World!".as_bytes());

        let binary_msg = Message {
            message_type: MessageType::Binary,
            data: vec![0x01, 0x02, 0x03],
        };
        assert_eq!(binary_msg.message_type, MessageType::Binary);
        assert_eq!(binary_msg.data, vec![0x01, 0x02, 0x03]);

        let ping_msg = Message {
            message_type: MessageType::Ping,
            data: vec![],
        };
        assert_eq!(ping_msg.message_type, MessageType::Ping);

        let pong_msg = Message {
            message_type: MessageType::Pong,
            data: vec![],
        };
        assert_eq!(pong_msg.message_type, MessageType::Pong);

        let close_msg = Message {
            message_type: MessageType::Close,
            data: vec![],
        };
        assert_eq!(close_msg.message_type, MessageType::Close);
    }

    #[test]
    fn test_zero_copy_components() {
        // Test zero copy buffer
        let mut buffer = ZeroCopyBuffer::new();
        let data = b"test data";
        buffer.append(data);
        assert_eq!(buffer.len(), data.len());

        let read_data = buffer.read(data.len());
        assert_eq!(read_data, data);

        // Test zero copy codec
        let codec = ZeroCopyCodec::new();
        let test_data = TestData {
            id: 42,
            name: "test".to_string(),
            value: 3.14,
        };
        let encoded = codec.encode(&test_data);
        assert!(encoded.is_ok());

        // Test message batch
        let mut batch = MessageBatch::new();
        let message = Message {
            message_type: MessageType::Text,
            data: "batch_test".as_bytes().to_vec(),
        };
        batch.add_message(message);
        assert_eq!(batch.len(), 1);
    }

    #[test]
    fn test_rpc_correlation_manager() {
        let mut manager = RpcCorrelationManager::new();

        // Test request registration
        let (request_id, receiver) =
            manager.register_request("test_method".to_string(), "test_id".to_string());
        assert!(!request_id.is_empty());
        assert_eq!(manager.pending_count(), 1);

        // Test response handling
        let response = leptos_ws_pro::rpc::RpcResponse {
            id: "test_id".to_string(),
            result: Some(serde_json::Value::String("response".to_string())),
            error: None,
        };
        let result = manager.handle_response(response);
        assert!(result.is_ok());
    }

    #[test]
    fn test_codec_edge_cases() {
        let json_codec = JsonCodec::new();
        let rkyv_codec = RkyvCodec::new();
        let hybrid_codec = HybridCodec::new();

        // Test content types
        assert_eq!(json_codec.content_type(), "application/json");
        assert_eq!(rkyv_codec.content_type(), "application/rkyv");
        assert_eq!(hybrid_codec.content_type(), "application/hybrid");

        // Test with empty data
        let empty_data = TestData {
            id: 0,
            name: String::new(),
            value: 0.0,
        };
        let encoded = json_codec.encode(&empty_data);
        assert!(encoded.is_ok());

        // Test with large data
        let large_data = TestData {
            id: u32::MAX,
            name: "x".repeat(10000),
            value: f64::MAX,
        };
        let encoded = json_codec.encode(&large_data);
        assert!(encoded.is_ok());
    }

    #[test]
    fn test_error_recovery_handler() {
        let mut handler = ErrorRecoveryHandler::new();

        // Test error handling
        let error = leptos_ws_pro::error_handling::LeptosWsError::Transport(
            leptos_ws_pro::transport::TransportError::ConnectionFailed("test".to_string()),
        );
        let result = handler.handle_error(&error, || Ok::<(), _>(()));
        assert!(result.is_ok());
    }

    #[test]
    fn test_ws_message_wrapper() {
        let data = TestData {
            id: 123,
            name: "wrapper_test".to_string(),
            value: 1.23,
        };
        let message = WsMessage::new(data.clone());

        assert_eq!(message.data.id, 123);
        assert_eq!(message.data.name, "wrapper_test");
        assert_eq!(message.data.value, 1.23);
    }

    #[test]
    fn test_performance_profiler() {
        let mut profiler = leptos_ws_pro::performance::PerformanceProfiler::new();

        // Test span creation
        let span = profiler.start_span("test_span");
        assert_eq!(span, ());

        // Test span ending
        profiler.end_span("test_span");

        // Test stats retrieval
        let stats = profiler.get_stats("test_span");
        assert!(stats.is_some());
    }

    #[test]
    fn test_connection_metrics() {
        let metrics = leptos_ws_pro::reactive::ConnectionMetrics::default();

        assert_eq!(metrics.bytes_sent, 0);
        assert_eq!(metrics.bytes_received, 0);
        assert_eq!(metrics.messages_sent, 0);
        assert_eq!(metrics.messages_received, 0);
        assert_eq!(metrics.connection_uptime, Duration::from_secs(0));
    }

    #[test]
    fn test_presence_map() {
        let mut presence = leptos_ws_pro::reactive::PresenceMap::default();

        let user_presence = leptos_ws_pro::reactive::UserPresence {
            user_id: "user1".to_string(),
            status: "online".to_string(),
            last_seen: 1234567890, // Unix timestamp
        };

        presence.update_presence(user_presence.clone());
        let retrieved = presence.get_presence("user1");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().user_id, "user1");
    }
}
