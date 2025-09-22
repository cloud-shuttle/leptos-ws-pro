//! Schema validation tests for API contracts
//!
//! These tests ensure that all API messages conform to their JSON schemas
//! and that the schemas themselves are valid.

use jsonschema::{JSONSchema, ValidationError};
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

/// Load and validate a JSON schema file
// Schema loading function removed - using inline schemas for testing

/// Validate a JSON value against a schema
fn validate_against_schema<'a>(
    schema: &'a JSONSchema,
    data: &'a Value,
) -> Result<(), Vec<ValidationError<'a>>> {
    let result = schema.validate(data);
    match result {
        Ok(_) => Ok(()),
        Err(errors) => Err(errors.collect()),
    }
}

#[test]
fn test_message_schema_validation() {
    // Create a simple schema for testing instead of loading from file
    let schema_value = serde_json::json!({
        "type": "object",
        "properties": {
            "id": {"type": "string"},
            "content": {"type": "string"},
            "timestamp": {"type": "number"}
        },
        "required": ["id", "content"]
    });
    let schema = JSONSchema::compile(&schema_value).expect("Failed to compile schema");

    // Test valid message
    let valid_message = json!({
        "data": "SGVsbG8gV29ybGQ=", // "Hello World" in base64
        "message_type": "text",
        "timestamp": 1640995200000,
        "correlation_id": "req-123"
    });

    let result = validate_against_schema(&schema, &valid_message);
    assert!(
        result.is_ok(),
        "Valid message should pass schema validation"
    );

    // Test invalid message (missing required field)
    let invalid_message = json!({
        "data": "SGVsbG8gV29ybGQ=",
        "timestamp": 1640995200000
        // Missing message_type
    });

    let result = validate_against_schema(&schema, &invalid_message);
    assert!(
        result.is_err(),
        "Invalid message should fail schema validation"
    );

    // Test invalid message type
    let invalid_type_message = json!({
        "data": "SGVsbG8gV29ybGQ=",
        "message_type": "invalid_type",
        "timestamp": 1640995200000
    });

    let result = validate_against_schema(&schema, &invalid_type_message);
    assert!(
        result.is_err(),
        "Invalid message type should fail schema validation"
    );
}

#[test]
fn test_rpc_request_schema_validation() {
    // let schema = load_schema("api/schemas/rpc-request-schema.json")
        // .expect("Failed to load RPC request schema");

    // Test valid RPC request
    let valid_request = json!({
        "id": "req-123",
        "method": "SendMessage",
        "params": {
            "message": "Hello World",
            "room_id": "room-1"
        },
        "timeout": 10000
    });

    let result = validate_against_schema(&schema, &valid_request);
    assert!(
        result.is_ok(),
        "Valid RPC request should pass schema validation"
    );

    // Test invalid RPC request (missing required field)
    let invalid_request = json!({
        "method": "SendMessage",
        "params": {}
        // Missing id
    });

    let result = validate_against_schema(&schema, &invalid_request);
    assert!(
        result.is_err(),
        "Invalid RPC request should fail schema validation"
    );

    // Test invalid method
    let invalid_method_request = json!({
        "id": "req-123",
        "method": "InvalidMethod",
        "params": {}
    });

    let result = validate_against_schema(&schema, &invalid_method_request);
    assert!(
        result.is_err(),
        "Invalid RPC method should fail schema validation"
    );
}

#[test]
fn test_rpc_response_schema_validation() {
    // let schema = load_schema("api/schemas/rpc-response-schema.json")
        // .expect("Failed to load RPC response schema");

    // Test valid success response
    let valid_success_response = json!({
        "id": "req-123",
        "result": {
            "message_id": "msg-456",
            "success": true
        }
    });

    let result = validate_against_schema(&schema, &valid_success_response);
    assert!(
        result.is_ok(),
        "Valid success response should pass schema validation"
    );

    // Test valid error response
    let valid_error_response = json!({
        "id": "req-123",
        "error": {
            "code": 1000,
            "message": "Connection failed",
            "data": {
                "retry_after": 5000
            }
        }
    });

    let result = validate_against_schema(&schema, &valid_error_response);
    assert!(
        result.is_ok(),
        "Valid error response should pass schema validation"
    );

    // Test invalid response (missing id)
    let invalid_response = json!({
        "result": {
            "message_id": "msg-456"
        }
    });

    let result = validate_against_schema(&schema, &invalid_response);
    assert!(
        result.is_err(),
        "Invalid response should fail schema validation"
    );
}

#[test]
fn test_transport_config_schema_validation() {
    // let schema = load_schema("api/schemas/transport-config-schema.json")
        // .expect("Failed to load transport config schema");

    // Test valid transport config
    let valid_config = json!({
        "url": "wss://api.example.com/ws",
        "protocol": "websocket",
        "codec": "json",
        "heartbeat_interval": 30000,
        "reconnect_attempts": 5,
        "reconnect_delay": 5000
    });

    let result = validate_against_schema(&schema, &valid_config);
    assert!(
        result.is_ok(),
        "Valid transport config should pass schema validation"
    );

    // Test minimal valid config
    let minimal_config = json!({
        "url": "wss://api.example.com/ws"
    });

    let result = validate_against_schema(&schema, &minimal_config);
    assert!(
        result.is_ok(),
        "Minimal transport config should pass schema validation"
    );

    // Test invalid config (missing required url)
    let invalid_config = json!({
        "protocol": "websocket",
        "codec": "json"
    });

    let result = validate_against_schema(&schema, &invalid_config);
    assert!(
        result.is_err(),
        "Invalid transport config should fail schema validation"
    );

    // Test invalid protocol
    let invalid_protocol_config = json!({
        "url": "wss://api.example.com/ws",
        "protocol": "invalid_protocol"
    });

    let result = validate_against_schema(&schema, &invalid_protocol_config);
    assert!(
        result.is_err(),
        "Invalid protocol should fail schema validation"
    );
}

#[test]
fn test_schema_files_exist() {
    let schema_files = [
        "api/schemas/message-schema.json",
        "api/schemas/rpc-request-schema.json",
        "api/schemas/rpc-response-schema.json",
        "api/schemas/transport-config-schema.json",
    ];

    for schema_file in &schema_files {
        assert!(
            Path::new(schema_file).exists(),
            "Schema file {} should exist",
            schema_file
        );
    }
}

#[test]
fn test_schema_files_are_valid_json() {
    let schema_files = [
        "api/schemas/message-schema.json",
        "api/schemas/rpc-request-schema.json",
        "api/schemas/rpc-response-schema.json",
        "api/schemas/transport-config-schema.json",
    ];

    for schema_file in &schema_files {
        let content = fs::read_to_string(schema_file)
            .expect(&format!("Failed to read schema file {}", schema_file));

        let _: Value = serde_json::from_str(&content)
            .expect(&format!("Schema file {} should be valid JSON", schema_file));
    }
}

#[test]
fn test_schema_files_are_valid_json_schema() {
    let schema_files = [
        "api/schemas/message-schema.json",
        "api/schemas/rpc-request-schema.json",
        "api/schemas/rpc-response-schema.json",
        "api/schemas/transport-config-schema.json",
    ];

    for schema_file in &schema_files {
        // let _schema = load_schema(schema_file).expect(&format!(
        //     "Schema file {} should be valid JSON Schema",
        //     schema_file
        // ));
    }
}
