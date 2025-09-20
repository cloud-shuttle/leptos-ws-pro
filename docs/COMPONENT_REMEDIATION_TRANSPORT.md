# Transport Layer Remediation Plan

## Current Status: ⚠️ PARTIALLY IMPLEMENTED

### What Works

- ✅ **WebSocket Client**: Basic tokio-tungstenite implementation
- ✅ **Transport Trait**: Abstract interface defined
- ✅ **Configuration**: Basic transport configuration structure

### What's Broken/Missing

- ❌ **SSE Implementation**: Empty stub, will not compile
- ❌ **WebTransport**: Incomplete, missing core functionality
- ❌ **Adaptive Transport**: Logic exists but delegates to broken implementations
- ❌ **WASM Support**: No wasm32-unknown-unknown WebSocket path
- ❌ **Connection Pooling**: Stub implementation only
- ❌ **Error Recovery**: Basic retry logic, missing circuit breaker

## Critical Issues

### 1. Missing Transport Implementations

**Problem**: `src/transport/sse.rs` and `src/transport/webtransport.rs` are missing
**Impact**: Build fails when features enabled
**Solution**: Implement minimal working versions

### 2. Platform Support Gap

**Problem**: No WASM WebSocket implementation despite web-sys dependency
**Impact**: Cannot run in browser environments
**Solution**: Add conditional compilation for wasm32 target

### 3. Connection Management

**Problem**: Each transport handles connections independently
**Impact**: No unified connection pooling or lifecycle management
**Solution**: Centralized connection manager

## Remediation Tasks

### Phase 1: Core Transport Stability (Week 1-2)

- [ ] **Fix SSE Implementation**
  - Create `transport/sse/client.rs` with eventsource-stream
  - Implement basic event parsing and reconnection
  - Add error handling and connection state management

- [ ] **Complete WebTransport Implementation**
  - Fix `transport/webtransport/connection.rs` compilation errors
  - Implement client-side WebTransport connection
  - Add server-side support using h3/quinn

- [ ] **Add WASM WebSocket Support**
  - Conditional compilation for `wasm32-unknown-unknown`
  - Use `web-sys` WebSocket API for browser environments
  - Shared trait implementation for native and WASM

### Phase 2: Transport Unification (Week 3-4)

- [ ] **Adaptive Transport Logic**
  - Implement protocol detection and fallback
  - Add transport capability negotiation
  - Create unified transport selection algorithm

- [ ] **Connection Pool Manager**
  - Centralized connection lifecycle management
  - Per-transport connection pooling
  - Connection health monitoring and cleanup

### Phase 3: Advanced Features (Week 5-8)

- [ ] **Circuit Breaker Pattern**
  - Per-transport circuit breaker implementation
  - Failure detection and automatic recovery
  - Configurable failure thresholds

- [ ] **Performance Optimizations**
  - Zero-copy message passing where possible
  - Connection multiplexing for WebTransport
  - Batched message sending

## Implementation Priorities

### P0: Build Fixes (Critical)

```rust
// transport/sse/mod.rs - Minimal working implementation
pub struct SseConnection {
    url: String,
    state: ConnectionState,
}

impl Transport for SseConnection {
    async fn connect(&mut self) -> Result<(), TransportError> {
        // Basic SSE connection using eventsource-stream
        todo!("Implement basic SSE connection")
    }

    async fn send(&mut self, message: Message) -> Result<(), TransportError> {
        Err(TransportError::NotSupported("SSE is read-only".into()))
    }
}
```

### P1: Core Functionality

```rust
// transport/manager.rs - Connection management
pub struct TransportManager {
    connections: HashMap<TransportId, Box<dyn Transport>>,
    pool: ConnectionPool,
}

impl TransportManager {
    pub async fn get_or_create(&mut self, config: TransportConfig) -> Result<TransportId, Error> {
        // Implement connection pooling and lifecycle management
    }
}
```

### P2: Advanced Features

```rust
// transport/adaptive.rs - Smart transport selection
pub struct AdaptiveTransport {
    transports: Vec<Box<dyn Transport>>,
    selector: TransportSelector,
}

impl AdaptiveTransport {
    pub async fn auto_select(&self, requirements: &Requirements) -> Result<TransportId, Error> {
        // Implement intelligent transport selection
    }
}
```

## Testing Strategy

### Unit Tests

- [ ] Each transport implementation
- [ ] Error handling and edge cases
- [ ] Configuration validation
- [ ] State management

### Integration Tests

- [ ] Real network connections
- [ ] Transport fallback scenarios
- [ ] Connection pooling behavior
- [ ] Performance benchmarks

### Contract Tests

- [ ] Transport protocol compliance
- [ ] Message format validation
- [ ] API contract adherence

## Success Criteria

1. **Build Success**: `cargo check --all-features` passes
2. **Test Coverage**: >80% coverage on transport layer
3. **Real Connections**: Can establish actual WebSocket, SSE, and WebTransport connections
4. **Platform Support**: Works on both native and WASM targets
5. **Performance**: Minimal overhead for connection management

## Timeline: 8 weeks total

- **Weeks 1-2**: Core stability and build fixes
- **Weeks 3-4**: Transport unification and management
- **Weeks 5-6**: Advanced features and optimizations
- **Weeks 7-8**: Testing, documentation, and performance validation
