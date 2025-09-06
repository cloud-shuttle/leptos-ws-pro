# TDD Phase 1 Implementation Plan: Real Network Implementation

## 🎯 **Current State Assessment**

### ✅ **What's Working:**

- **28 unit tests passing** - Core library functionality works
- **Compilation successful** - No blocking errors
- **Latest dependencies** - Rust 2024, tokio-tungstenite 0.27.0, etc.
- **Architecture in place** - All modules and interfaces defined

### ❌ **What's Broken:**

- **7 integration tests failing** - All WebSocket connection tests fail
- **Simulated connections** - No real network implementation
- **Missing real WebSocket handshake** - Protocol handling not implemented
- **No network error handling** - Connection management incomplete

---

## 🚀 **TDD Implementation Strategy**

### **Phase 1A: Real WebSocket Connection (Week 1)**

#### **Test 1: Basic WebSocket Connection**

```rust
#[tokio::test]
async fn test_real_websocket_connection() {
    // Given: A WebSocket server running on localhost:8080
    let server = start_test_server().await;

    // When: Client connects to the server
    let mut client = WebSocketConnection::new(TransportConfig::default()).await?;
    let result = client.connect("ws://localhost:8080").await;

    // Then: Connection should succeed
    assert!(result.is_ok());
    assert_eq!(client.state(), ConnectionState::Connected);
}
```

#### **Test 2: WebSocket Handshake**

```rust
#[tokio::test]
async fn test_websocket_handshake() {
    // Given: A WebSocket server with specific headers
    let server = start_test_server_with_headers().await;

    // When: Client connects with custom headers
    let mut client = WebSocketConnection::new(config).await?;
    let result = client.connect_with_headers("ws://localhost:8080", headers).await;

    // Then: Handshake should include custom headers
    assert!(result.is_ok());
    assert!(server.received_headers().contains("Authorization"));
}
```

#### **Test 3: Message Sending/Receiving**

```rust
#[tokio::test]
async fn test_websocket_message_flow() {
    // Given: Connected WebSocket client and server
    let (client, server) = setup_connected_websocket().await;

    // When: Client sends a message
    let message = Message::text("Hello, WebSocket!");
    let send_result = client.send(message.clone()).await;

    // Then: Server should receive the message
    assert!(send_result.is_ok());
    let received = server.receive_message().await;
    assert_eq!(received, message);
}
```

### **Phase 1B: Network Error Handling (Week 1-2)**

#### **Test 4: Connection Timeout**

```rust
#[tokio::test]
async fn test_connection_timeout() {
    // Given: A non-existent server address
    let mut client = WebSocketConnection::new(config).await?;

    // When: Client tries to connect with timeout
    let result = client.connect_with_timeout("ws://127.0.0.1:99999", Duration::from_millis(100)).await;

    // Then: Should timeout and return error
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), TransportError::Timeout));
}
```

#### **Test 5: Network Interruption Recovery**

```rust
#[tokio::test]
async fn test_network_interruption_recovery() {
    // Given: Connected WebSocket client
    let (mut client, server) = setup_connected_websocket().await;

    // When: Server disconnects unexpectedly
    server.disconnect().await;

    // Then: Client should detect disconnection and attempt reconnection
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(client.state(), ConnectionState::Disconnected);

    // When: Server comes back online
    server.restart().await;

    // Then: Client should automatically reconnect
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert_eq!(client.state(), ConnectionState::Connected);
}
```

### **Phase 1C: Transport Layer Completion (Week 2-3)**

#### **Test 6: WebTransport Implementation**

```rust
#[tokio::test]
async fn test_webtransport_connection() {
    // Given: A WebTransport server
    let server = start_webtransport_server().await;

    // When: Client connects via WebTransport
    let mut client = WebTransportConnection::new(config).await?;
    let result = client.connect("https://localhost:8080").await;

    // Then: Connection should succeed
    assert!(result.is_ok());
    assert_eq!(client.state(), ConnectionState::Connected);
}
```

#### **Test 7: Server-Sent Events**

```rust
#[tokio::test]
async fn test_sse_connection() {
    // Given: An SSE server
    let server = start_sse_server().await;

    // When: Client connects via SSE
    let mut client = SseConnection::new(config).await?;
    let result = client.connect("http://localhost:8080/events").await;

    // Then: Connection should succeed and receive events
    assert!(result.is_ok());
    let event = client.receive_event().await;
    assert!(event.is_some());
}
```

#### **Test 8: Adaptive Transport Selection**

```rust
#[tokio::test]
async fn test_adaptive_transport_selection() {
    // Given: Multiple transport options available
    let mut transport = AdaptiveTransport::new(config).await?;

    // When: Client connects to a URL
    let result = transport.connect("ws://localhost:8080").await;

    // Then: Should automatically select best transport
    assert!(result.is_ok());
    assert!(matches!(transport.selected_transport(), TransportType::WebSocket));
}
```

---

## 🛠️ **Implementation Steps**

### **Step 1: Replace Simulated WebSocket Connection**

**File:** `src/transport/websocket.rs`

**Current State:** Stub implementation with `unimplemented!()`

**Target Implementation:**

```rust
use tokio_tungstenite::{connect_async, WebSocketStream, MaybeTlsStream};
use tokio::net::TcpStream;
use futures::{SinkExt, StreamExt};

pub struct WebSocketConnection {
    config: TransportConfig,
    state: Arc<RwLock<ConnectionState>>,
    stream: Option<WebSocketStream<MaybeTlsStream<TcpStream>>>,
}

impl WebSocketConnection {
    pub async fn connect(&mut self, url: &str) -> Result<(), TransportError> {
        // Real tokio-tungstenite implementation
        let (ws_stream, _) = connect_async(url)
            .await
            .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?;

        self.stream = Some(ws_stream);
        *self.state.write().await = ConnectionState::Connected;
        Ok(())
    }

    pub async fn send(&mut self, message: Message) -> Result<(), TransportError> {
        if let Some(stream) = &mut self.stream {
            let ws_message = match message {
                Message::Text(text) => tokio_tungstenite::tungstenite::Message::Text(text.into()),
                Message::Binary(data) => tokio_tungstenite::tungstenite::Message::Binary(data),
            };

            stream.send(ws_message).await
                .map_err(|e| TransportError::SendFailed(e.to_string()))?;
            Ok(())
        } else {
            Err(TransportError::NotConnected)
        }
    }
}
```

### **Step 2: Implement Network Error Handling**

**File:** `src/transport/error.rs`

**Add new error types:**

```rust
#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Connection timeout")]
    Timeout,

    #[error("Network interruption: {0}")]
    NetworkInterruption(String),

    #[error("Send failed: {0}")]
    SendFailed(String),

    #[error("Receive failed: {0}")]
    ReceiveFailed(String),

    #[error("Not connected")]
    NotConnected,
}
```

### **Step 3: Implement WebTransport Support**

**File:** `src/transport/webtransport.rs`

**Target Implementation:**

```rust
use hyper::client::conn::http2::SendRequest;
use hyper_util::rt::TokioExecutor;

pub struct WebTransportConnection {
    config: TransportConfig,
    state: Arc<RwLock<ConnectionState>>,
    connection: Option<SendRequest<hyper::body::Incoming>>,
}

impl WebTransportConnection {
    pub async fn connect(&mut self, url: &str) -> Result<(), TransportError> {
        // Real WebTransport implementation using hyper
        let uri = url.parse::<hyper::Uri>()
            .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?;

        let (connection, sender) = hyper::client::conn::http2::Builder::new(TokioExecutor::new())
            .handshake(uri)
            .await
            .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?;

        self.connection = Some(sender);
        *self.state.write().await = ConnectionState::Connected;
        Ok(())
    }
}
```

### **Step 4: Implement Server-Sent Events**

**File:** `src/transport/sse.rs`

**Target Implementation:**

```rust
use reqwest::Client;
use futures::stream::StreamExt;

pub struct SseConnection {
    config: TransportConfig,
    state: Arc<RwLock<ConnectionState>>,
    client: Client,
    stream: Option<reqwest::EventStream>,
}

impl SseConnection {
    pub async fn connect(&mut self, url: &str) -> Result<(), TransportError> {
        let response = self.client
            .get(url)
            .header("Accept", "text/event-stream")
            .send()
            .await
            .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?;

        let stream = response.bytes_stream();
        self.stream = Some(stream);
        *self.state.write().await = ConnectionState::Connected;
        Ok(())
    }
}
```

---

## 📊 **Success Metrics**

### **Week 1 Targets:**

- [ ] All 7 failing integration tests pass
- [ ] Real WebSocket connections work
- [ ] Basic message sending/receiving works
- [ ] Connection timeout handling works

### **Week 2 Targets:**

- [ ] Network interruption recovery works
- [ ] WebTransport implementation complete
- [ ] SSE implementation complete
- [ ] Error handling comprehensive

### **Week 3 Targets:**

- [ ] Adaptive transport selection works
- [ ] All transport types tested
- [ ] Performance benchmarks met
- [ ] Documentation updated

---

## 🧪 **Testing Strategy**

### **Unit Tests:**

- Test individual transport implementations
- Test error handling scenarios
- Test configuration options

### **Integration Tests:**

- Test real network connections
- Test multiple transport types
- Test failure scenarios

### **End-to-End Tests:**

- Test with real WebSocket servers
- Test with real WebTransport servers
- Test with real SSE servers

---

## 🚀 **Next Steps**

1. **Start with WebSocket implementation** - Most critical for Phase 1
2. **Write failing tests first** - Follow TDD methodology
3. **Implement minimal working solution** - Make tests pass
4. **Refactor and optimize** - Improve implementation
5. **Move to next transport type** - Repeat process

This plan ensures we build a solid foundation for real network communication while maintaining the existing architecture and test coverage.
