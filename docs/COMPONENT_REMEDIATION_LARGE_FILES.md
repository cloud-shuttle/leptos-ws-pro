# Large Files Remediation Plan

## Current Status: ⚠️ **FILES TOO LARGE**

### Problem

Several files exceed the 300-line limit, making them difficult to maintain, test, and understand.

### Files Requiring Split

- `src/rpc/advanced.rs`: **441 lines** ⚠️
- `src/rpc/correlation.rs`: **445 lines** ⚠️
- `src/transport/adaptive.rs`: **386 lines** ⚠️
- `src/transport/websocket.rs`: **378 lines** ⚠️
- `src/transport/mod.rs`: **357 lines** ⚠️
- `src/transport/optimized.rs`: **305 lines** ⚠️
- `src/performance/manager.rs`: **397 lines** ⚠️

## Remediation Plan

### 1. RPC Advanced Module Split

#### Current: `src/rpc/advanced.rs` (441 lines)

**Split into:**

- `src/rpc/advanced/mod.rs` (50 lines) - Main module
- `src/rpc/advanced/client.rs` (150 lines) - BidirectionalRpcClient
- `src/rpc/advanced/server.rs` (150 lines) - RpcServer implementation
- `src/rpc/advanced/middleware.rs` (100 lines) - Middleware integration
- `src/rpc/advanced/streaming.rs` (100 lines) - Streaming RPC support

#### Structure:

```rust
// src/rpc/advanced/mod.rs
pub mod client;
pub mod server;
pub mod middleware;
pub mod streaming;

pub use client::BidirectionalRpcClient;
pub use server::RpcServer;
pub use middleware::RpcMiddleware;
pub use streaming::StreamingRpc;
```

### 2. RPC Correlation Module Split

#### Current: `src/rpc/correlation.rs` (445 lines)

**Split into:**

- `src/rpc/correlation/mod.rs` (50 lines) - Main module
- `src/rpc/correlation/manager.rs` (150 lines) - CorrelationManager
- `src/rpc/correlation/tracker.rs` (150 lines) - RequestTracker
- `src/rpc/correlation/timeout.rs` (100 lines) - Timeout handling
- `src/rpc/correlation/metrics.rs` (100 lines) - Correlation metrics

#### Structure:

```rust
// src/rpc/correlation/mod.rs
pub mod manager;
pub mod tracker;
pub mod timeout;
pub mod metrics;

pub use manager::CorrelationManager;
pub use tracker::RequestTracker;
pub use timeout::TimeoutManager;
pub use metrics::CorrelationMetrics;
```

### 3. Transport Adaptive Module Split

#### Current: `src/transport/adaptive.rs` (386 lines)

**Split into:**

- `src/transport/adaptive/mod.rs` (50 lines) - Main module
- `src/transport/adaptive/selector.rs` (150 lines) - Transport selection logic
- `src/transport/adaptive/fallback.rs` (150 lines) - Fallback mechanisms
- `src/transport/adaptive/capabilities.rs` (100 lines) - Capability detection

#### Structure:

```rust
// src/transport/adaptive/mod.rs
pub mod selector;
pub mod fallback;
pub mod capabilities;

pub use selector::TransportSelector;
pub use fallback::FallbackManager;
pub use capabilities::TransportCapabilities;
```

### 4. Transport WebSocket Module Split

#### Current: `src/transport/websocket.rs` (378 lines)

**Split into:**

- `src/transport/websocket/mod.rs` (50 lines) - Main module
- `src/transport/websocket/client.rs` (150 lines) - WebSocket client
- `src/transport/websocket/server.rs` (150 lines) - WebSocket server
- `src/transport/websocket/frame.rs` (100 lines) - Frame handling

#### Structure:

```rust
// src/transport/websocket/mod.rs
pub mod client;
pub mod server;
pub mod frame;

pub use client::WebSocketClient;
pub use server::WebSocketServer;
pub use frame::WebSocketFrame;
```

### 5. Transport Module Split

#### Current: `src/transport/mod.rs` (357 lines)

**Split into:**

- `src/transport/mod.rs` (50 lines) - Main module exports
- `src/transport/traits.rs` (150 lines) - Transport traits
- `src/transport/config.rs` (100 lines) - Configuration types
- `src/transport/errors.rs` (100 lines) - Transport errors

#### Structure:

```rust
// src/transport/mod.rs
pub mod traits;
pub mod config;
pub mod errors;
pub mod websocket;
pub mod sse;
pub mod webtransport;
pub mod adaptive;
pub mod optimized;

pub use traits::*;
pub use config::*;
pub use errors::*;
```

### 6. Transport Optimized Module Split

#### Current: `src/transport/optimized.rs` (305 lines)

**Split into:**

- `src/transport/optimized/mod.rs` (50 lines) - Main module
- `src/transport/optimized/transport.rs` (150 lines) - OptimizedTransport
- `src/transport/optimized/stream.rs` (100 lines) - OptimizedStream
- `src/transport/optimized/sink.rs` (100 lines) - OptimizedSink

#### Structure:

```rust
// src/transport/optimized/mod.rs
pub mod transport;
pub mod stream;
pub mod sink;

pub use transport::OptimizedTransport;
pub use stream::OptimizedStream;
pub use sink::OptimizedSink;
```

### 7. Performance Manager Module Split

#### Current: `src/performance/manager.rs` (397 lines)

**Split into:**

- `src/performance/manager/mod.rs` (50 lines) - Main module
- `src/performance/manager/core.rs` (150 lines) - PerformanceManager core
- `src/performance/manager/monitoring.rs` (150 lines) - Monitoring logic
- `src/performance/manager/optimization.rs` (100 lines) - Optimization strategies

#### Structure:

```rust
// src/performance/manager/mod.rs
pub mod core;
pub mod monitoring;
pub mod optimization;

pub use core::PerformanceManager;
pub use monitoring::PerformanceMonitor;
pub use optimization::OptimizationStrategy;
```

## Implementation Strategy

### Phase 1: Preparation (Week 1)

1. **Create new module directories**
2. **Move existing code to new files**
3. **Update module declarations**
4. **Fix import statements**

### Phase 2: Refactoring (Week 1-2)

1. **Extract common functionality**
2. **Create shared utilities**
3. **Improve code organization**
4. **Add proper documentation**

### Phase 3: Testing (Week 2)

1. **Update existing tests**
2. **Add new unit tests**
3. **Verify functionality unchanged**
4. **Performance testing**

## Benefits of Splitting

### Maintainability

- Easier to understand individual components
- Simpler to modify specific functionality
- Better separation of concerns

### Testing

- More focused unit tests
- Easier to mock dependencies
- Better test coverage

### LLM Compatibility

- Smaller files easier for AI to process
- Better context understanding
- More precise code generation

## Success Criteria

- [ ] All files under 300 lines
- [ ] No functionality lost
- [ ] All tests passing
- [ ] Clean module structure
- [ ] Proper documentation
- [ ] No compilation warnings

## Estimated Effort

- **Refactoring**: 2 weeks
- **Testing**: 1 week
- **Total**: 3 weeks
