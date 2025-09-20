# RPC System Remediation Plan

## Current Status: ⚠️ PARTIALLY FUNCTIONAL

### What Works

- ✅ **Request/Response Types**: Basic RPC message structures
- ✅ **Correlation Logic**: Request-response correlation framework
- ✅ **JSON Codec**: Basic serialization support
- ✅ **RPC Client Structure**: Basic client framework exists

### What's Broken/Missing

- ❌ **Transport Integration**: RpcClient doesn't own connections
- ❌ **Connection Management**: Requires external mpsc::Sender
- ❌ **Timeout Handling**: Basic timeout, no advanced retry logic
- ❌ **Type Safety**: Limited compile-time RPC method validation
- ❌ **Error Handling**: Incomplete error recovery strategies
- ❌ **Streaming RPCs**: No support for bidirectional streaming

## Critical Issues

### 1. Incomplete Transport Integration

**Problem**: RpcClient takes `mpsc::Sender<RpcMessage>` but doesn't manage connection
**Impact**: Users must handle connection lifecycle manually
**Solution**: RpcClient should own `Box<dyn Transport>`

### 2. Limited Type Safety

**Problem**: RPC methods are runtime strings, not compile-time validated
**Impact**: No compile-time guarantees for method signatures
**Solution**: Proc macro for type-safe RPC definitions

### 3. Missing Advanced Features

**Problem**: No streaming, no async iterators, no cancellation
**Impact**: Cannot handle complex RPC patterns
**Solution**: Implement streaming RPC support

## Remediation Tasks

### Phase 1: Core RPC Stability (Week 1-2)

- [ ] **Fix Transport Ownership**

  ```rust
  pub struct RpcClient {
      transport: Box<dyn Transport>,
      correlations: CorrelationManager,
      config: RpcConfig,
  }

  impl RpcClient {
      pub fn new(transport: Box<dyn Transport>) -> Self { /* ... */ }
      pub async fn call<T, R>(&mut self, method: &str, params: T) -> Result<R, RpcError>
      where T: Serialize, R: DeserializeOwned { /* ... */ }
  }
  ```

- [ ] **Improve Error Handling**
  - Add comprehensive error types for all failure modes
  - Implement retry logic with exponential backoff
  - Add circuit breaker for failing RPC endpoints

- [ ] **Connection Lifecycle Management**
  - Automatic reconnection on transport failures
  - Connection health monitoring
  - Graceful shutdown and cleanup

### Phase 2: Type Safety Enhancement (Week 3-4)

- [ ] **Proc Macro for Type-Safe RPCs**

  ```rust
  #[rpc_service]
  trait UserService {
      async fn get_user(id: u64) -> Result<User, UserError>;
      async fn create_user(user: CreateUserRequest) -> Result<User, UserError>;
  }

  // Generated client:
  let client = UserServiceClient::new(transport);
  let user = client.get_user(123).await?; // Compile-time validated
  ```

- [ ] **Method Registry**
  - Compile-time method validation
  - Automatic serialization/deserialization
  - Parameter type checking

### Phase 3: Advanced RPC Features (Week 5-6)

- [ ] **Streaming RPC Support**

  ```rust
  #[rpc_service]
  trait StreamingService {
      async fn subscribe_events() -> impl Stream<Item = Event>;
      async fn upload_data(stream: impl Stream<Item = Chunk>) -> Result<(), Error>;
      async fn bidirectional_chat() -> (impl Sink<ChatMessage>, impl Stream<Item = ChatMessage>);
  }
  ```

- [ ] **Request Cancellation**
  - Tokio cancellation token integration
  - Timeout handling per request
  - Graceful cancellation propagation

- [ ] **Batch Operations**
  - Multiple RPC calls in single network round-trip
  - Batch result handling and error management
  - Configurable batch size limits

### Phase 4: Performance & Observability (Week 7-8)

- [ ] **Performance Optimizations**
  - Connection multiplexing
  - Request pipelining where supported
  - Zero-copy serialization integration

- [ ] **Observability Integration**
  - Distributed tracing for RPC calls
  - Metrics collection (latency, success rate, etc.)
  - Request/response logging with filtering

## Implementation Priorities

### P0: Core Functionality (Critical)

```rust
// rpc/client.rs - Fixed client implementation
impl RpcClient {
    pub async fn connect<T: Transport + 'static>(mut transport: T) -> Result<Self, RpcError> {
        transport.connect().await?;
        Ok(Self {
            transport: Box::new(transport),
            correlations: CorrelationManager::new(),
            config: RpcConfig::default(),
        })
    }

    pub async fn call<Req, Resp>(&mut self, method: &str, params: Req) -> Result<Resp, RpcError>
    where
        Req: Serialize + Send,
        Resp: DeserializeOwned + Send,
    {
        let request_id = self.correlations.generate_id();
        let message = RpcMessage::request(request_id, method, params)?;

        // Send via owned transport
        self.transport.send(message.into()).await?;

        // Wait for correlation response
        self.correlations.wait_for_response(request_id).await
    }
}
```

### P1: Type Safety

```rust
// rpc/macros.rs - Type-safe RPC generation
#[proc_macro_attribute]
pub fn rpc_service(args: TokenStream, input: TokenStream) -> TokenStream {
    // Generate type-safe client and server implementations
    // from trait definition
}
```

### P2: Advanced Features

```rust
// rpc/streaming.rs - Streaming RPC support
pub trait StreamingRpc {
    type Request: Serialize;
    type Response: DeserializeOwned;
    type Stream: Stream<Item = Self::Response>;

    async fn call_streaming(&mut self, req: Self::Request) -> Result<Self::Stream, RpcError>;
}
```

## Testing Strategy

### Unit Tests

- [ ] Request/response correlation
- [ ] Error handling scenarios
- [ ] Timeout and cancellation
- [ ] Serialization/deserialization

### Integration Tests

- [ ] Real transport integration
- [ ] Connection failure recovery
- [ ] Concurrent RPC calls
- [ ] Streaming RPC operations

### Performance Tests

- [ ] RPC call latency benchmarks
- [ ] Throughput under load
- [ ] Memory usage profiling
- [ ] Connection scaling tests

## Success Criteria

1. **Self-Contained**: RpcClient manages its own transport connection
2. **Type Safety**: Compile-time validation of RPC method signatures
3. **Reliability**: Automatic retry and error recovery
4. **Performance**: <1ms overhead for local RPC calls
5. **Feature Complete**: Support for streaming and advanced RPC patterns

## Timeline: 8 weeks total

- **Weeks 1-2**: Core functionality fixes and stability
- **Weeks 3-4**: Type safety and proc macro implementation
- **Weeks 5-6**: Advanced features (streaming, cancellation)
- **Weeks 7-8**: Performance optimization and comprehensive testing
