# File Breakdown Strategy - Large File Remediation

## Files Requiring Breakdown (>300 lines)

### Priority 1: Critical Files (>500 lines)

#### 1. `src/reactive/mod.rs` (587 lines) - CRITICAL

**Target Structure:**

- `reactive/signals.rs` (~150 lines) - Core signal implementations
- `reactive/websocket.rs` (~150 lines) - WebSocket reactive integration
- `reactive/transport.rs` (~150 lines) - Transport-agnostic reactive layer
- `reactive/hooks.rs` (~150 lines) - Leptos hooks and utilities
- `reactive/mod.rs` (~50 lines) - Module organization and re-exports

#### 2. `src/transport/webtransport/connection.rs` (507 lines)

**Target Structure:**

- `connection/client.rs` (~150 lines) - Client connection logic
- `connection/server.rs` (~150 lines) - Server connection handling
- `connection/stream.rs` (~150 lines) - Stream management
- `connection/config.rs` (~100 lines) - Configuration and setup

#### 3. `src/transport/sse/connection.rs` (503 lines)

**Target Structure:**

- `sse/client.rs` (~150 lines) - SSE client implementation
- `sse/server.rs` (~150 lines) - SSE server implementation
- `sse/events.rs` (~100 lines) - Event parsing and handling
- `sse/reconnect.rs` (~100 lines) - Reconnection logic

### Priority 2: High Priority Files (400-499 lines)

#### 4. `src/rpc/correlation.rs` (422 lines)

**Target Structure:**

- `rpc/correlation/manager.rs` (~150 lines) - Request correlation management
- `rpc/correlation/types.rs` (~100 lines) - Correlation data types
- `rpc/correlation/timeout.rs` (~100 lines) - Timeout handling
- `rpc/correlation/cleanup.rs` (~100 lines) - Resource cleanup

#### 5. `src/middleware/mod.rs` (412 lines)

**Target Structure:**

- `middleware/auth.rs` (~150 lines) - Authentication middleware
- `middleware/rate_limit.rs` (~100 lines) - Rate limiting middleware
- `middleware/validation.rs` (~100 lines) - Input validation
- `middleware/chain.rs` (~100 lines) - Middleware chaining

#### 6. `src/metrics/mod.rs` (407 lines)

**Target Structure:**

- `metrics/collector.rs` (~150 lines) - Metrics collection
- `metrics/exporter.rs` (~100 lines) - Metrics export
- `metrics/types.rs` (~100 lines) - Metric data types
- `metrics/config.rs` (~75 lines) - Configuration

### Priority 3: Medium Priority Files (350-399 lines)

#### 7-10. Additional Files

Similar breakdown strategy for:

- `src/rpc/advanced.rs` (394 lines) → `rpc/advanced/{client,server,types,config}.rs`
- `src/resilience/mod.rs` (391 lines) → `resilience/{circuit_breaker,retry,timeout,backoff}.rs`
- `src/transport/websocket.rs` (373 lines) → `transport/websocket/{client,server,config,errors}.rs`
- `src/performance/manager.rs` (372 lines) → `performance/manager/{pool,cache,metrics,config}.rs`

## Implementation Strategy

### Step 1: Create Target Structure

```bash
# Example for reactive module
mkdir -p src/reactive/{signals,websocket,transport,hooks}
```

### Step 2: Extract and Move Code

- Move related functions/structs to appropriate files
- Maintain single responsibility principle
- Ensure each file has <300 lines
- Preserve public API compatibility

### Step 3: Update Module Declarations

```rust
// reactive/mod.rs
pub mod signals;
pub mod websocket;
pub mod transport;
pub mod hooks;

// Re-export public APIs
pub use signals::*;
pub use websocket::*;
// etc.
```

### Step 4: Validation

- [ ] `cargo check` passes
- [ ] All tests compile and pass
- [ ] No breaking API changes
- [ ] Documentation builds
- [ ] Each file <300 lines

## Benefits of This Approach

1. **Better Testing**: Smaller files = more focused tests
2. **LLM Friendly**: Each file fits in context window
3. **Maintainability**: Single responsibility principle
4. **Code Review**: Easier to review smaller changes
5. **Parallel Development**: Teams can work on different files

## Timeline

- **Week 1**: Priority 1 files (3 largest files)
- **Week 2**: Priority 2 files (4 medium files)
- **Week 3**: Priority 3 files (remaining 10 files)
- **Week 4**: Validation and documentation
