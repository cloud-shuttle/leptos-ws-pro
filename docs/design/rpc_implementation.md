# 🚀 **RPC System Real Implementation Design**

## 🎯 **OBJECTIVE**

Replace mock RPC responses with real WebSocket communication, implementing actual request/response correlation and timeout handling.

## 📊 **CURRENT STATE**

### **What's Working**

- ✅ RPC request/response type definitions
- ✅ Request ID generation and correlation structure
- ✅ Error handling framework
- ✅ Basic client structure

### **What's Missing**

- ❌ Real WebSocket message sending
- ❌ Actual request/response correlation
- ❌ Real timeout handling
- ❌ Integration with transport layer

## 🏗️ **ARCHITECTURE DESIGN**

### **Core Components**

```
RpcClient
├── MessageSender (tokio::sync::mpsc::UnboundedSender<Message>)
├── ResponseReceiver (tokio::sync::mpsc::UnboundedReceiver<RpcResponse>)
├── CorrelationManager (manages pending requests)
├── TimeoutManager (handles request timeouts)
└── ErrorHandler (processes RPC errors)
```

### **Message Flow**

```
Client Request → JSON Serialize → WebSocket Send → Server
Server Response → WebSocket Receive → JSON Deserialize → Client
```

## 🔧 **IMPLEMENTATION PLAN**

### **Phase 1: Real Message Sending (Week 1)**

#### **1.1 Update RpcClient Constructor**

```rust
impl<T> RpcClient<T> {
    pub fn new(
        message_sender: mpsc::UnboundedSender<Message>,
        response_receiver: mpsc::UnboundedReceiver<RpcResponse<T>>,
    ) -> Self {
        Self {
            correlation_manager: Arc::new(RpcCorrelationManager::new()),
            message_sender,
            response_receiver: Arc::new(Mutex::new(Some(response_receiver))),
            // ... other fields
        }
    }
}
```

#### **1.2 Implement Real RPC Call**

```rust
pub async fn call<U>(&self, method_name: &str, params: U, method_type: RpcMethod) -> Result<RpcResponse<T>, RpcError> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let request = RpcRequest {
        id: request_id.clone(),
        method: method_name.to_string(),
        params,
        method_type,
    };

    // Serialize request to JSON
    let request_json = serde_json::to_string(&request)?;

    // Create WebSocket message
    let message = Message {
        data: request_json.into_bytes(),
        message_type: MessageType::Text,
    };

    // Send via WebSocket
    self.message_sender.send(message)?;

    // Wait for response with timeout
    self.wait_for_response(&request_id, Duration::from_secs(30)).await
}
```

### **Phase 2: Response Correlation (Week 2)**

#### **2.1 Response Handler**

```rust
pub async fn handle_response(&self, response_data: &[u8]) -> Result<(), RpcError> {
    let response: RpcResponse<T> = serde_json::from_slice(response_data)?;

    // Send to waiting request
    if let Some(receiver) = self.response_receiver.lock().await.as_mut() {
        receiver.send(response).map_err(|_| RpcError::InternalError)?;
    }

    Ok(())
}
```

#### **2.2 Timeout Handling**

```rust
async fn wait_for_response(&self, request_id: &str, timeout: Duration) -> Result<RpcResponse<T>, RpcError> {
    let start = std::time::Instant::now();

    loop {
        if start.elapsed() > timeout {
            return Err(RpcError::Timeout);
        }

        if let Some(receiver) = self.response_receiver.lock().await.as_mut() {
            if let Ok(response) = receiver.try_recv() {
                if response.id == request_id {
                    return Ok(response);
                }
            }
        }

        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}
```

## 🧪 **TESTING STRATEGY**

### **Unit Tests**

1. **Request Serialization** - Verify JSON serialization
2. **Response Deserialization** - Verify JSON deserialization
3. **Timeout Handling** - Test timeout scenarios
4. **Error Handling** - Test error propagation

### **Integration Tests**

1. **Echo Server Test** - Send request, receive echo response
2. **Error Response Test** - Test server error responses
3. **Timeout Test** - Test request timeout scenarios
4. **Concurrent Requests** - Test multiple simultaneous requests

### **Test Server Setup**

```rust
// Simple echo server for testing
async fn start_test_server() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;

    while let Ok((stream, _)) = listener.accept().await {
        let ws_stream = tokio_tungstenite::accept_async(stream).await?;
        // Handle WebSocket messages and echo back
    }
}
```

## 📊 **SUCCESS CRITERIA**

### **Functional Requirements**

- ✅ Real WebSocket message sending
- ✅ Actual request/response correlation
- ✅ Timeout handling (30 seconds default)
- ✅ Error handling and propagation
- ✅ Concurrent request support

### **Performance Requirements**

- ✅ < 10ms latency for local connections
- ✅ 100+ concurrent requests
- ✅ < 1MB memory overhead per client
- ✅ 95%+ test coverage

### **Quality Requirements**

- ✅ All integration tests pass with real servers
- ✅ No memory leaks in long-running tests
- ✅ Proper error handling for all failure modes
- ✅ Thread-safe concurrent access

## 🔄 **MIGRATION STRATEGY**

### **Backward Compatibility**

- Maintain existing API surface
- Add new constructor with message channels
- Keep mock responses as fallback option
- Gradual migration of existing code

### **Rollout Plan**

1. **Week 1**: Implement real message sending
2. **Week 2**: Add response correlation and timeout handling
3. **Week 3**: Integration testing with real servers
4. **Week 4**: Performance optimization and final testing

## 🚨 **RISKS & MITIGATION**

### **High Risk Items**

1. **Message Channel Deadlocks** - Use unbounded channels and proper error handling
2. **Timeout Race Conditions** - Use atomic operations and proper synchronization
3. **Memory Leaks** - Implement proper cleanup and resource management
4. **Performance Degradation** - Benchmark and optimize critical paths

### **Mitigation Strategies**

1. **Comprehensive Testing** - Unit, integration, and performance tests
2. **Gradual Rollout** - Implement incrementally with validation
3. **Fallback Options** - Maintain mock responses as backup
4. **Monitoring** - Add metrics and logging for production use

---

**This design provides a clear path to implementing real RPC communication while maintaining the existing API and ensuring production-ready quality.**
