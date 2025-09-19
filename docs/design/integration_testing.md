# 🧪 **Integration Testing Implementation Design**

## 🎯 **OBJECTIVE**

Implement comprehensive integration testing with real WebSocket servers to validate the library's functionality in production-like environments.

## 📊 **CURRENT STATE**

### **What's Working**

- ✅ Unit test framework (41 tests passing)
- ✅ Basic test infrastructure
- ✅ Mock-based testing
- ✅ Test organization structure

### **What's Missing**

- ❌ Real WebSocket server integration tests
- ❌ End-to-end RPC testing
- ❌ Performance benchmarking with real servers
- ❌ Load testing infrastructure

## 🏗️ **ARCHITECTURE DESIGN**

### **Test Infrastructure**

```
Integration Tests
├── WebSocket Server Tests (real servers)
├── RPC End-to-End Tests (request/response cycles)
├── Performance Tests (benchmarks with real servers)
├── Load Tests (high-load scenarios)
└── Security Tests (penetration testing)
```

### **Test Server Architecture**

```
Test Infrastructure
├── Echo Server (message echo)
├── RPC Server (request/response)
├── Performance Server (benchmarking)
├── Load Test Server (high-load scenarios)
└── Security Test Server (attack simulation)
```

## 🔧 **IMPLEMENTATION PLAN**

### **Phase 1: Test Server Infrastructure (Week 1)**

#### **1.1 WebSocket Echo Server**

```rust
// tests/integration/servers/echo_server.rs
pub struct EchoServer {
    listener: TcpListener,
    clients: Arc<Mutex<Vec<WebSocketStream<MaybeTlsStream<TcpStream>>>>>,
}

impl EchoServer {
    pub async fn new(port: u16) -> Result<Self, Box<dyn std::error::Error>> {
        let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).await?;

        Ok(Self {
            listener,
            clients: Arc::new(Mutex::new(Vec::new())),
        })
    }

    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        while let Ok((stream, _)) = self.listener.accept().await {
            let ws_stream = tokio_tungstenite::accept_async(stream).await?;
            let clients = self.clients.clone();

            tokio::spawn(async move {
                Self::handle_client(ws_stream, clients).await;
            });
        }
        Ok(())
    }

    async fn handle_client(
        mut ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
        clients: Arc<Mutex<Vec<WebSocketStream<MaybeTlsStream<TcpStream>>>>>,
    ) {
        while let Some(msg) = ws_stream.next().await {
            match msg {
                Ok(tungstenite::Message::Text(text)) => {
                    // Echo back the message
                    if let Err(e) = ws_stream.send(tungstenite::Message::Text(text)).await {
                        eprintln!("Error sending message: {}", e);
                        break;
                    }
                }
                Ok(tungstenite::Message::Binary(data)) => {
                    // Echo back binary data
                    if let Err(e) = ws_stream.send(tungstenite::Message::Binary(data)).await {
                        eprintln!("Error sending binary data: {}", e);
                        break;
                    }
                }
                Err(e) => {
                    eprintln!("WebSocket error: {}", e);
                    break;
                }
                _ => {}
            }
        }
    }
}
```

#### **1.2 RPC Test Server**

```rust
// tests/integration/servers/rpc_server.rs
pub struct RpcServer {
    listener: TcpListener,
    methods: Arc<Mutex<HashMap<String, RpcMethodHandler>>>,
}

impl RpcServer {
    pub async fn new(port: u16) -> Result<Self, Box<dyn std::error::Error>> {
        let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).await?;

        let mut server = Self {
            listener,
            methods: Arc::new(Mutex::new(HashMap::new())),
        };

        // Register default RPC methods
        server.register_method("echo", Self::echo_handler).await;
        server.register_method("add", Self::add_handler).await;
        server.register_method("get_time", Self::get_time_handler).await;

        Ok(server)
    }

    async fn register_method<F>(&self, name: &str, handler: F)
    where
        F: Fn(serde_json::Value) -> Result<serde_json::Value, String> + Send + Sync + 'static,
    {
        self.methods.lock().unwrap().insert(name.to_string(), Box::new(handler));
    }

    async fn handle_rpc_request(&self, request: RpcRequest) -> RpcResponse {
        let methods = self.methods.lock().unwrap();

        if let Some(handler) = methods.get(&request.method) {
            match handler(request.params) {
                Ok(result) => RpcResponse {
                    id: request.id,
                    result: Some(result),
                    error: None,
                },
                Err(error) => RpcResponse {
                    id: request.id,
                    result: None,
                    error: Some(RpcError {
                        code: -32603,
                        message: error,
                        data: None,
                    }),
                },
            }
        } else {
            RpcResponse {
                id: request.id,
                result: None,
                error: Some(RpcError {
                    code: -32601,
                    message: "Method not found".to_string(),
                    data: None,
                }),
            }
        }
    }

    fn echo_handler(params: serde_json::Value) -> Result<serde_json::Value, String> {
        Ok(params)
    }

    fn add_handler(params: serde_json::Value) -> Result<serde_json::Value, String> {
        if let (Some(a), Some(b)) = (params.get("a").and_then(|v| v.as_i64()),
                                     params.get("b").and_then(|v| v.as_i64())) {
            Ok(serde_json::json!({ "result": a + b }))
        } else {
            Err("Invalid parameters".to_string())
        }
    }

    fn get_time_handler(_params: serde_json::Value) -> Result<serde_json::Value, String> {
        Ok(serde_json::json!({ "timestamp": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() }))
    }
}
```

### **Phase 2: Integration Test Suite (Week 2)**

#### **2.1 WebSocket Integration Tests**

```rust
// tests/integration/websocket_integration_tests.rs
#[tokio::test]
async fn test_websocket_echo_integration() {
    // Start echo server
    let server = EchoServer::new(8080).await.unwrap();
    let server_handle = tokio::spawn(async move {
        server.start().await.unwrap();
    });

    // Wait for server to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Create WebSocket client
    let config = TransportConfig {
        url: "ws://127.0.0.1:8080".to_string(),
        ..Default::default()
    };

    let mut client = WebSocketConnection::new(config).await.unwrap();
    client.connect("ws://127.0.0.1:8080").await.unwrap();

    // Test message echo
    let test_message = Message {
        data: b"Hello, WebSocket!".to_vec(),
        message_type: MessageType::Text,
    };

    client.send_message(&test_message).await.unwrap();

    // Receive echo response
    let response = client.receive_message().await.unwrap();
    assert_eq!(response.data, test_message.data);

    // Cleanup
    client.disconnect().await.unwrap();
    server_handle.abort();
}

#[tokio::test]
async fn test_websocket_binary_integration() {
    // Similar test for binary messages
    let server = EchoServer::new(8081).await.unwrap();
    let server_handle = tokio::spawn(async move {
        server.start().await.unwrap();
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let config = TransportConfig {
        url: "ws://127.0.0.1:8081".to_string(),
        ..Default::default()
    };

    let mut client = WebSocketConnection::new(config).await.unwrap();
    client.connect("ws://127.0.0.1:8081").await.unwrap();

    let binary_data = vec![0x00, 0x01, 0x02, 0x03, 0x04];
    let test_message = Message {
        data: binary_data.clone(),
        message_type: MessageType::Binary,
    };

    client.send_message(&test_message).await.unwrap();
    let response = client.receive_message().await.unwrap();
    assert_eq!(response.data, binary_data);

    client.disconnect().await.unwrap();
    server_handle.abort();
}
```

#### **2.2 RPC Integration Tests**

```rust
// tests/integration/rpc_integration_tests.rs
#[tokio::test]
async fn test_rpc_echo_integration() {
    // Start RPC server
    let server = RpcServer::new(8082).await.unwrap();
    let server_handle = tokio::spawn(async move {
        server.start().await.unwrap();
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Create RPC client
    let (message_sender, _message_receiver) = mpsc::unbounded_channel();
    let (_, response_receiver) = mpsc::unbounded_channel();
    let codec = JsonCodec::new();

    let client = RpcClient::new(message_sender, response_receiver, codec);

    // Test echo RPC call
    let params = serde_json::json!({"message": "Hello, RPC!"});
    let response = client.call("echo", params, RpcMethod::Call).await.unwrap();

    assert!(response.result.is_some());
    assert!(response.error.is_none());

    server_handle.abort();
}

#[tokio::test]
async fn test_rpc_add_integration() {
    let server = RpcServer::new(8083).await.unwrap();
    let server_handle = tokio::spawn(async move {
        server.start().await.unwrap();
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let (message_sender, _message_receiver) = mpsc::unbounded_channel();
    let (_, response_receiver) = mpsc::unbounded_channel();
    let codec = JsonCodec::new();

    let client = RpcClient::new(message_sender, response_receiver, codec);

    // Test add RPC call
    let params = serde_json::json!({"a": 5, "b": 3});
    let response = client.call("add", params, RpcMethod::Call).await.unwrap();

    assert!(response.result.is_some());
    if let Some(result) = response.result {
        assert_eq!(result["result"], 8);
    }

    server_handle.abort();
}
```

### **Phase 3: Performance Testing (Week 3)**

#### **3.1 Performance Benchmark Server**

```rust
// tests/integration/servers/performance_server.rs
pub struct PerformanceServer {
    listener: TcpListener,
    metrics: Arc<Mutex<PerformanceMetrics>>,
}

impl PerformanceServer {
    pub async fn new(port: u16) -> Result<Self, Box<dyn std::error::Error>> {
        let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).await?;

        Ok(Self {
            listener,
            metrics: Arc::new(Mutex::new(PerformanceMetrics::new())),
        })
    }

    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        while let Ok((stream, _)) = self.listener.accept().await {
            let ws_stream = tokio_tungstenite::accept_async(stream).await?;
            let metrics = self.metrics.clone();

            tokio::spawn(async move {
                Self::handle_performance_client(ws_stream, metrics).await;
            });
        }
        Ok(())
    }

    async fn handle_performance_client(
        mut ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
        metrics: Arc<Mutex<PerformanceMetrics>>,
    ) {
        let mut message_count = 0;
        let start_time = std::time::Instant::now();

        while let Some(msg) = ws_stream.next().await {
            match msg {
                Ok(tungstenite::Message::Text(_)) => {
                    message_count += 1;

                    // Update metrics
                    let mut m = metrics.lock().unwrap();
                    m.total_messages += 1;
                    m.messages_per_second = message_count as f64 / start_time.elapsed().as_secs_f64();
                }
                Ok(tungstenite::Message::Binary(_)) => {
                    message_count += 1;
                }
                Err(_) => break,
                _ => {}
            }
        }
    }
}
```

#### **3.2 Performance Benchmark Tests**

```rust
// tests/integration/performance_tests.rs
#[tokio::test]
async fn test_message_throughput_benchmark() {
    let server = PerformanceServer::new(8084).await.unwrap();
    let server_handle = tokio::spawn(async move {
        server.start().await.unwrap();
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let config = TransportConfig {
        url: "ws://127.0.0.1:8084".to_string(),
        ..Default::default()
    };

    let mut client = WebSocketConnection::new(config).await.unwrap();
    client.connect("ws://127.0.0.1:8084").await.unwrap();

    let start_time = std::time::Instant::now();
    let message_count = 1000;

    // Send messages as fast as possible
    for i in 0..message_count {
        let message = Message {
            data: format!("message_{}", i).into_bytes(),
            message_type: MessageType::Text,
        };
        client.send_message(&message).await.unwrap();
    }

    let duration = start_time.elapsed();
    let throughput = message_count as f64 / duration.as_secs_f64();

    // Assert minimum throughput (messages per second)
    assert!(throughput > 100.0, "Throughput should be at least 100 messages/second");

    client.disconnect().await.unwrap();
    server_handle.abort();
}

#[tokio::test]
async fn test_latency_benchmark() {
    let server = EchoServer::new(8085).await.unwrap();
    let server_handle = tokio::spawn(async move {
        server.start().await.unwrap();
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let config = TransportConfig {
        url: "ws://127.0.0.1:8085".to_string(),
        ..Default::default()
    };

    let mut client = WebSocketConnection::new(config).await.unwrap();
    client.connect("ws://127.0.0.1:8085").await.unwrap();

    let mut latencies = Vec::new();
    let test_count = 100;

    for _ in 0..test_count {
        let start_time = std::time::Instant::now();

        let message = Message {
            data: b"latency_test".to_vec(),
            message_type: MessageType::Text,
        };

        client.send_message(&message).await.unwrap();
        let _response = client.receive_message().await.unwrap();

        let latency = start_time.elapsed();
        latencies.push(latency);
    }

    let avg_latency = latencies.iter().sum::<Duration>() / latencies.len() as u32;

    // Assert maximum latency (milliseconds)
    assert!(avg_latency.as_millis() < 10, "Average latency should be less than 10ms");

    client.disconnect().await.unwrap();
    server_handle.abort();
}
```

## 🧪 **TESTING STRATEGY**

### **Test Categories**

1. **Unit Tests** - Individual component testing
2. **Integration Tests** - Cross-component testing with real servers
3. **Performance Tests** - Benchmarking with real servers
4. **Load Tests** - High-load scenario testing
5. **Security Tests** - Penetration testing and validation

### **Test Infrastructure**

1. **Test Servers** - Real WebSocket servers for testing
2. **Test Clients** - Library clients for testing
3. **Test Data** - Realistic test data and scenarios
4. **Test Metrics** - Performance and quality metrics

## 📊 **SUCCESS CRITERIA**

### **Functional Requirements**

- ✅ All integration tests pass with real servers
- ✅ RPC request/response cycles work end-to-end
- ✅ WebSocket connections work with real servers
- ✅ Error handling works with real network conditions

### **Performance Requirements**

- ✅ Message throughput > 100 messages/second
- ✅ Average latency < 10ms for local connections
- ✅ Connection establishment < 100ms
- ✅ Memory usage < 50MB under normal load

### **Quality Requirements**

- ✅ 95%+ test coverage for integration scenarios
- ✅ All tests pass consistently
- ✅ No memory leaks in long-running tests
- ✅ Proper error handling for all failure modes

## 🔄 **MIGRATION STRATEGY**

### **Test Infrastructure**

- Set up test servers in CI/CD pipeline
- Automate test server startup and teardown
- Add test data and scenario management
- Implement test result reporting and analysis

### **Rollout Plan**

1. **Week 1**: Implement test server infrastructure
2. **Week 2**: Implement integration test suite
3. **Week 3**: Implement performance testing
4. **Week 4**: Add load testing and optimization

## 🚨 **RISKS & MITIGATION**

### **High Risk Items**

1. **Test Server Dependencies** - Tests depend on external servers
2. **Test Flakiness** - Network conditions might cause test failures
3. **Performance Variability** - Performance tests might be inconsistent
4. **Resource Usage** - Test servers might consume significant resources

### **Mitigation Strategies**

1. **Test Isolation** - Use separate ports and processes for each test
2. **Retry Logic** - Add retry logic for flaky tests
3. **Performance Baselines** - Set realistic performance baselines
4. **Resource Management** - Implement proper resource cleanup

---

**This design provides a comprehensive integration testing framework that validates the library's functionality with real WebSocket servers and ensures production-ready quality.**
