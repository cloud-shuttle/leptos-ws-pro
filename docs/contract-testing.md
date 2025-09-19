# Contract Testing Documentation

## Overview

Contract testing ensures that the Leptos WS Pro API implementation matches its specifications and maintains backward compatibility. This document describes our comprehensive contract testing strategy.

## What is Contract Testing?

Contract testing verifies that:

- **API implementations match their specifications** (OpenAPI, JSON Schema)
- **Client and server expectations are aligned**
- **Backward compatibility is maintained** across versions
- **Performance requirements are met**
- **Security requirements are enforced**

## Contract Testing Strategy

### 1. Schema Validation Testing

**Purpose**: Ensure all API messages conform to their JSON schemas.

**Tools**: `jsonschema` crate for Rust

**Test Coverage**:

- Message format validation
- RPC request/response validation
- Transport configuration validation
- Error response validation

**Example**:

```rust
#[test]
fn test_message_schema_validation() {
    let schema = load_schema("api/schemas/message-schema.json")?;
    let valid_message = json!({
        "data": "SGVsbG8gV29ybGQ=",
        "message_type": "text",
        "timestamp": 1640995200000
    });

    let result = validate_against_schema(&schema, &valid_message);
    assert!(result.is_ok());
}
```

### 2. API Contract Testing

**Purpose**: Verify API endpoints behave according to OpenAPI specification.

**Tools**: Custom mock clients and servers

**Test Coverage**:

- WebSocket connection establishment
- RPC method calls
- Message subscriptions
- Health checks
- Error handling

**Example**:

```rust
#[test]
fn test_websocket_connection_contract() {
    let client = MockApiClient::new("wss://api.example.com".to_string());
    let params = json!({
        "url": "wss://api.example.com/ws",
        "protocol": "websocket",
        "codec": "json"
    });

    let connection = client.connect_websocket(&params);
    assert!(connection.is_ok());
}
```

### 3. Backward Compatibility Testing

**Purpose**: Ensure API changes don't break existing clients.

**Test Coverage**:

- Version compatibility
- Deprecated feature handling
- Migration path validation
- Breaking change detection

**Example**:

```rust
#[test]
fn test_backward_compatible_requests() {
    let server = VersionedApiClient::new(ApiVersion::new(1, 1, 0));
    let old_client = ApiVersion::new(1, 0, 0);

    let request = json!({
        "id": "req-123",
        "method": "SendMessage",
        "params": {}
    });

    let response = server.handle_request(&request, &old_client);
    assert!(response.is_ok());
}
```

## Contract Specifications

### API Contracts

**Location**: `api/contracts/`

**Files**:

- `leptos-ws-pro-api-v1.yaml` - OpenAPI 3.0 specification
- `contract-testing-config.yaml` - Contract testing configuration

### JSON Schemas

**Location**: `api/schemas/`

**Files**:

- `message-schema.json` - Core message format
- `rpc-request-schema.json` - RPC request format
- `rpc-response-schema.json` - RPC response format
- `transport-config-schema.json` - Transport configuration

### API Versions

**Location**: `api/versions/`

**Files**:

- `v1-api-contract.md` - Version 1.0 API contract

## Running Contract Tests

### All Contract Tests

```bash
cargo test --test schema_validation_tests
cargo test --test api_contract_tests
cargo test --test backward_compatibility_tests
```

### Specific Test Categories

```bash
# Schema validation only
cargo test --test schema_validation_tests

# API contract validation only
cargo test --test api_contract_tests

# Backward compatibility only
cargo test --test backward_compatibility_tests
```

### With Verbose Output

```bash
cargo test --test schema_validation_tests -- --nocapture
```

## Contract Testing in CI/CD

### GitHub Actions Integration

```yaml
name: Contract Testing
on: [push, pull_request]

jobs:
  contract-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Run Contract Tests
        run: |
          cargo test --test schema_validation_tests
          cargo test --test api_contract_tests
          cargo test --test backward_compatibility_tests
      - name: Validate OpenAPI Spec
        run: |
          npx @apidevtools/swagger-cli validate api/contracts/leptos-ws-pro-api-v1.yaml
```

### Pre-commit Hooks

```bash
#!/bin/bash
# .git/hooks/pre-commit

echo "Running contract tests..."
cargo test --test schema_validation_tests
if [ $? -ne 0 ]; then
    echo "Schema validation tests failed!"
    exit 1
fi

cargo test --test api_contract_tests
if [ $? -ne 0 ]; then
    echo "API contract tests failed!"
    exit 1
fi

echo "Contract tests passed!"
```

## Contract Evolution

### Adding New Features

1. **Update OpenAPI specification**
2. **Add JSON schemas for new types**
3. **Write contract tests**
4. **Ensure backward compatibility**
5. **Update documentation**

### Deprecating Features

1. **Mark feature as deprecated in OpenAPI spec**
2. **Add deprecation notice to contract**
3. **Provide migration guide**
4. **Set removal timeline**
5. **Update contract tests**

### Breaking Changes

1. **Increment major version**
2. **Provide migration guide**
3. **Maintain old version for transition period**
4. **Update all contract tests**
5. **Notify all consumers**

## Performance Contract Testing

### Latency Requirements

- **Connection Establishment**: < 100ms
- **Message Round-trip**: < 50ms
- **RPC Response**: < 100ms

### Throughput Requirements

- **Messages per Second**: > 1000
- **Concurrent Connections**: > 1000
- **Message Size**: < 1MB

### Reliability Requirements

- **Uptime**: > 99.9%
- **Error Rate**: < 0.1%
- **Reconnection Success**: > 95%

## Security Contract Testing

### Authentication

- **Bearer Token**: JWT validation
- **API Key**: Header validation
- **Rate Limiting**: Request throttling

### Data Protection

- **Encryption**: TLS 1.3+ required
- **Input Validation**: All inputs validated
- **CORS**: Proper cross-origin policies

## Monitoring and Alerting

### Contract Violations

- **Schema validation failures**
- **API contract mismatches**
- **Performance requirement violations**
- **Security requirement violations**

### Metrics

- **Contract test pass rate**
- **API response time**
- **Error rate by endpoint**
- **Client version distribution**

## Best Practices

### 1. Schema Design

- Use descriptive field names
- Provide clear documentation
- Include examples
- Validate all constraints

### 2. API Design

- Follow RESTful principles
- Use consistent naming
- Provide clear error messages
- Include versioning strategy

### 3. Testing

- Test all edge cases
- Validate error scenarios
- Test performance requirements
- Ensure backward compatibility

### 4. Documentation

- Keep schemas up-to-date
- Provide migration guides
- Document breaking changes
- Include examples

## Troubleshooting

### Common Issues

**Schema Validation Failures**:

- Check field names and types
- Verify required fields
- Validate enum values
- Check format constraints

**API Contract Mismatches**:

- Verify endpoint URLs
- Check request/response formats
- Validate HTTP status codes
- Ensure error handling

**Backward Compatibility Issues**:

- Check version compatibility
- Verify deprecated features
- Test migration paths
- Validate breaking changes

### Debugging Tips

1. **Enable verbose logging**
2. **Use contract testing tools**
3. **Validate schemas manually**
4. **Test with real clients**
5. **Monitor production metrics**

## Future Enhancements

### Planned Features

- **Consumer-driven contract testing** (Pact)
- **Automated contract generation**
- **Real-time contract validation**
- **Contract testing dashboard**
- **Performance benchmarking**

### Integration Opportunities

- **API Gateway validation**
- **Load balancer health checks**
- **Monitoring system integration**
- **Alert system integration**

## Conclusion

Contract testing is essential for maintaining API quality and reliability. Our comprehensive approach ensures that:

- **APIs match their specifications**
- **Backward compatibility is maintained**
- **Performance requirements are met**
- **Security standards are enforced**
- **Clients can rely on stable interfaces**

By following this contract testing strategy, we ensure that Leptos WS Pro provides a reliable, performant, and secure API that meets the needs of all consumers.
