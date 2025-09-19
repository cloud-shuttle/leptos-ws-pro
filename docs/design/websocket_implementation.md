# WebSocket Implementation Design

## 🎯 **Objective**

Complete the WebSocket implementation to provide a robust, production-ready WebSocket client with proper error handling, reconnection, and message management.

## 📊 **Current State**

### **What's Working**

- ✅ Basic tokio-tungstenite integration
- ✅ Connection state management
- ✅ Message type conversion (internal ↔ WebSocket)
- ✅ Stream/Sink splitting for bidirectional communication
- ✅ Basic error handling structure

### **What's Missing**

- ❌ Real connection establishment and management
- ❌ Automatic reconnection with backoff
- ❌ Heartbeat/ping-pong mechanism
- ❌ Message queuing during disconnection
- ❌ Connection quality monitoring
- ❌ Proper error recovery

## 🏗 **Architecture Design**

### **Core Components**

```
WebSocketConnection
├── ConnectionManager (handles connect/disconnect/reconnect)
├── MessageHandler (processes incoming messages)
├── HeartbeatManager (ping/pong and connection health)
├── ReconnectionManager (automatic reconnection logic)
├── MessageQueue (queues messages during disconnection)
└── MetricsCollector (connection quality and performance)
```

### **State Machine**

```
Disconnected → Connecting → Connected → Reconnecting → Failed
     ↑              ↓           ↓           ↓           ↓
     └──────────────┴───────────┴───────────┴───────────┘
```

## 🛠 **Implementation Plan**

### **Phase 1: Core Connection Management**

#### **1.1 Enhanced Connection Establishment**

```rust
impl WebSocketConnection {
    pub async fn connect(&mut self, url: &str) -> Result<(), TransportError> {
        *self.state.lock().unwrap() = ConnectionState::Connecting;

        // Parse URL and validate
        let parsed_url = url::Url::parse(url)
            .map_err(|e| TransportError::ConnectionFailed(format!("Invalid URL: {}", e)))?;

        // Set up connection with proper headers
        let mut request = parsed_url.into_client_request()
            .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?;

        // Add custom headers from config
        for (key, value) in &self.config.headers {
            request.headers_mut().insert(
                key.parse().unwrap(),
                value.parse().unwrap()
            );
        }

        // Establish connection with timeout
        let connect_future = connect_async(request);
        let timeout = tokio::time::timeout(self.config.connection_timeout, connect_future);

        match timeout.await {
            Ok(Ok((ws_stream, response))) => {
                self.stream = Some(ws_stream);
                *self.state.lock().unwrap() = ConnectionState::Connected;
                self.start_heartbeat().await;
                Ok(())
            }
            Ok(Err(e)) => {
                *self.state.lock().unwrap() = ConnectionState::Failed;
                Err(TransportError::ConnectionFailed(e.to_string()))
            }
            Err(_) => {
                *self.state.lock().unwrap() = ConnectionState::Failed;
                Err(TransportError::ConnectionFailed("Connection timeout".to_string()))
            }
        }
    }
}
```

#### **1.2 Heartbeat Management**

```rust
struct HeartbeatManager {
    interval: Duration,
    timeout: Duration,
    last_pong: Arc<Mutex<Option<Instant>>>,
    is_running: Arc<AtomicBool>,
}

impl HeartbeatManager {
    pub async fn start(&self, stream: &mut WebSocketStream<MaybeTlsStream<TcpStream>>) {
        self.is_running.store(true, Ordering::Relaxed);

        while self.is_running.load(Ordering::Relaxed) {
            tokio::time::sleep(self.interval).await;

            // Send ping
            if let Err(e) = stream.send(Message::Ping(vec![])).await {
                eprintln!("Failed to send ping: {}", e);
                break;
            }

            // Check for pong timeout
            if let Some(last_pong) = *self.last_pong.lock().unwrap() {
                if last_pong.elapsed() > self.timeout {
                    eprintln!("Heartbeat timeout");
                    break;
                }
            }
        }
    }

    pub fn handle_pong(&self) {
        *self.last_pong.lock().unwrap() = Some(Instant::now());
    }
}
```

### **Phase 2: Reconnection System**

#### **2.1 Reconnection Manager**

```rust
struct ReconnectionManager {
    max_attempts: Option<usize>,
    base_delay: Duration,
    max_delay: Duration,
    jitter: bool,
    current_attempt: Arc<AtomicUsize>,
}

impl ReconnectionManager {
    pub async fn attempt_reconnection<F, Fut>(
        &self,
        mut connect_fn: F,
    ) -> Result<(), TransportError>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<(), TransportError>>,
    {
        let mut attempt = 0;

        loop {
            attempt += 1;

            // Check if we've exceeded max attempts
            if let Some(max) = self.max_attempts {
                if attempt > max {
                    return Err(TransportError::ConnectionFailed(
                        "Max reconnection attempts exceeded".to_string()
                    ));
                }
            }

            // Calculate delay with exponential backoff
            let delay = self.calculate_delay(attempt);
            tokio::time::sleep(delay).await;

            // Attempt reconnection
            match connect_fn().await {
                Ok(()) => return Ok(()),
                Err(e) => {
                    eprintln!("Reconnection attempt {} failed: {}", attempt, e);
                    // Continue to next attempt
                }
            }
        }
    }

    fn calculate_delay(&self, attempt: usize) -> Duration {
        let base_delay_ms = self.base_delay.as_millis() as u64;
        let delay_ms = base_delay_ms * 2_u64.pow(attempt as u32 - 1);
        let delay_ms = delay_ms.min(self.max_delay.as_millis() as u64);

        if self.jitter {
            let jitter = fastrand::u64(0..delay_ms / 4);
            Duration::from_millis(delay_ms + jitter)
        } else {
            Duration::from_millis(delay_ms)
        }
    }
}
```

### **Phase 3: Message Management**

#### **3.1 Message Queue for Disconnection**

```rust
struct MessageQueue {
    queue: Arc<Mutex<VecDeque<Message>>>,
    max_size: usize,
}

impl MessageQueue {
    pub fn new(max_size: usize) -> Self {
        Self {
            queue: Arc::new(Mutex::new(VecDeque::new())),
            max_size,
        }
    }

    pub fn enqueue(&self, message: Message) -> Result<(), TransportError> {
        let mut queue = self.queue.lock().unwrap();

        if queue.len() >= self.max_size {
            return Err(TransportError::SendFailed(
                "Message queue full".to_string()
            ));
        }

        queue.push_back(message);
        Ok(())
    }

    pub fn dequeue_all(&self) -> Vec<Message> {
        let mut queue = self.queue.lock().unwrap();
        queue.drain(..).collect()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.lock().unwrap().is_empty()
    }
}
```

#### **3.2 Enhanced Message Sending**

```rust
impl WebSocketConnection {
    pub async fn send_message(&self, message: &Message) -> Result<(), TransportError> {
        let state = *self.state.lock().unwrap();

        match state {
            ConnectionState::Connected => {
                // Send immediately
                self.send_immediate(message).await
            }
            ConnectionState::Connecting | ConnectionState::Reconnecting => {
                // Queue message for later
                self.message_queue.enqueue(message.clone())
            }
            ConnectionState::Disconnected | ConnectionState::Failed => {
                Err(TransportError::NotConnected)
            }
        }
    }

    async fn send_immediate(&self, message: &Message) -> Result<(), TransportError> {
        if let Some(stream) = &self.stream {
            let ws_message = self.convert_to_ws_message(message)?;
            stream.send(ws_message).await
                .map_err(|e| TransportError::SendFailed(e.to_string()))?;
            Ok(())
        } else {
            Err(TransportError::NotConnected)
        }
    }
}
```

### **Phase 4: Connection Quality Monitoring**

#### **4.1 Metrics Collection**

```rust
#[derive(Debug, Clone)]
pub struct ConnectionMetrics {
    pub messages_sent: u64,
    pub messages_received: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub connection_uptime: Duration,
    pub last_message_time: Option<Instant>,
    pub average_latency: Option<Duration>,
    pub reconnection_count: u32,
    pub error_count: u32,
}

impl ConnectionMetrics {
    pub fn new() -> Self {
        Self {
            messages_sent: 0,
            messages_received: 0,
            bytes_sent: 0,
            bytes_received: 0,
            connection_uptime: Duration::from_secs(0),
            last_message_time: None,
            average_latency: None,
            reconnection_count: 0,
            error_count: 0,
        }
    }

    pub fn record_message_sent(&mut self, size: usize) {
        self.messages_sent += 1;
        self.bytes_sent += size as u64;
        self.last_message_time = Some(Instant::now());
    }

    pub fn record_message_received(&mut self, size: usize) {
        self.messages_received += 1;
        self.bytes_received += size as u64;
        self.last_message_time = Some(Instant::now());
    }

    pub fn record_reconnection(&mut self) {
        self.reconnection_count += 1;
    }

    pub fn record_error(&mut self) {
        self.error_count += 1;
    }
}
```

## 🧪 **Testing Strategy**

### **Unit Tests**

- Connection establishment and failure
- Message sending and receiving
- Reconnection logic
- Heartbeat mechanism
- Message queuing

### **Integration Tests**

- Real WebSocket server communication
- Network interruption handling
- Long-running connection stability
- High-frequency message sending

### **Performance Tests**

- Connection establishment time
- Message throughput
- Memory usage under load
- Reconnection speed

## ✅ **Success Criteria**

### **Functionality**

- ✅ Reliable connection establishment
- ✅ Automatic reconnection with backoff
- ✅ Heartbeat/ping-pong working
- ✅ Message queuing during disconnection
- ✅ Proper error handling and recovery

### **Performance**

- ✅ < 100ms connection establishment
- ✅ > 1000 messages/second throughput
- ✅ < 1MB memory usage per connection
- ✅ < 1 second reconnection time

### **Reliability**

- ✅ Handles network interruptions gracefully
- ✅ Recovers from server restarts
- ✅ Maintains message ordering
- ✅ No memory leaks during long runs

## 🚀 **Implementation Timeline**

- **Day 1-2**: Core connection management
- **Day 3-4**: Heartbeat and reconnection system
- **Day 5-6**: Message queuing and management
- **Day 7**: Metrics and monitoring
- **Day 8**: Testing and validation

---

**Priority: HIGH - This is the foundation for all other transport implementations.**
