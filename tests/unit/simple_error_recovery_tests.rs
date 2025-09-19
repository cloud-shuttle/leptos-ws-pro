//! Simple Error Recovery Tests
//!
//! Basic tests for error recovery functionality using the existing API

use leptos_ws_pro::error_handling::{
    ErrorContext, CircuitBreaker, ErrorType
};
use leptos_ws_pro::transport::{TransportError, ConnectionState};
use std::time::Duration;

#[tokio::test]
async fn test_circuit_breaker_basic_functionality() {
    // Given: A circuit breaker
    let mut breaker = CircuitBreaker::new();

    // When: Recording failures
    for _ in 0..5 {
        if breaker.allow_request() {
            breaker.record_failure();
        }
    }

    // Then: Circuit should be open
    assert_eq!(breaker.get_state(), "open");

    // And: Should not allow requests
    assert!(!breaker.allow_request());
}

#[tokio::test]
async fn test_circuit_breaker_recovery() {
    // Given: A circuit breaker with failures
    let mut breaker = CircuitBreaker::new();

    // Record some failures to open the circuit
    for _ in 0..5 {
        if breaker.allow_request() {
            breaker.record_failure();
        }
    }

    assert_eq!(breaker.get_state(), "open");

    // Verify the circuit is open and can't allow requests
    assert!(!breaker.allow_request(), "Circuit should be open and not allow requests");

    // Test that recording success doesn't immediately close an open circuit
    breaker.record_success();
    assert_eq!(breaker.get_state(), "open", "Circuit should remain open until timeout");
}

#[tokio::test]
async fn test_error_context_creation() {
    // Given: Error context parameters
    let operation = "connect";
    let component = "websocket";

    // When: Creating error context
    let context = ErrorContext::new(operation, component)
        .with_connection_state(ConnectionState::Disconnected)
        .with_attempt(1)
        .with_trace_id("trace-123".to_string());

    // Then: Should have correct values
    assert_eq!(context.operation, "connect");
    assert_eq!(context.component, "websocket");
    assert_eq!(context.attempt_number, 1);
    assert_eq!(context.trace_id, Some("trace-123".to_string()));
    assert_eq!(context.connection_state, Some(ConnectionState::Disconnected));
}

#[tokio::test]
async fn test_error_context_with_error_type() {
    // Given: Error context with error type
    let mut context = ErrorContext::new("test", "component");
    context.error_type = Some(ErrorType::Transport);
    context.message = Some("Connection failed".to_string());
    context.service = Some("websocket-service".to_string());

    // Then: Should have correct error information
    assert_eq!(context.error_type, Some(ErrorType::Transport));
    assert_eq!(context.message, Some("Connection failed".to_string()));
    assert_eq!(context.service, Some("websocket-service".to_string()));
}
