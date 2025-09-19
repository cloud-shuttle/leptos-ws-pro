# Server-Sent Events (SSE) Implementation Design

## 🎯 **Objective**

Implement a complete Server-Sent Events client with real HTTP streaming, event parsing, reconnection strategies, and heartbeat mechanisms.

## 📊 **Current State**

### **What's Working**

- ✅ Basic SSE structure and types
- ✅ Event parsing framework
- ✅ Reconnection strategy definitions
- ✅ Heartbeat configuration structure

### **What's Missing**

- ❌ Real HTTP streaming implementation
- ❌ Event stream parsing
- ❌ Automatic reconnection logic
- ❌ Heartbeat mechanism
- ❌ Event type subscription
- ❌ Error handling and recovery

## 🏗 **Architecture Design**

### **Core Components**

```
SseConnection
├── HttpStreamManager (handles HTTP streaming)
├── EventParser (parses SSE events)
├── ReconnectionManager (automatic reconnection)
├── HeartbeatManager (connection health monitoring)
├── EventSubscriptionManager (event type filtering)
└── MetricsCollector (performance tracking)
```

### **Event Flow**

```
HTTP Request → Stream Response → Event Parser → Event Handler → Client
     ↑              ↓               ↓              ↓           ↓
     └──────────────┴───────────────┴──────────────┴───────────┘
```

## 🛠 **Implementation Plan**

### **Phase 1: HTTP Streaming Implementation**

#### **1.1 Real HTTP Stream Connection**

```rust
impl SseConnection {
    pub async fn connect(&mut self, url: &str) -> Result<(), TransportError> {
        *self.state.lock().unwrap() = ConnectionState::Connecting;

        // Parse and validate URL
        let parsed_url = url::Url::parse(url)
            .map_err(|e| TransportError::ConnectionFailed(format!("Invalid URL: {}", e)))?;

        // Create HTTP request with SSE headers
        let mut request = self.client
            .get(parsed_url.clone())
            .header("Accept", "text/event-stream")
            .header("Cache-Control", "no-cache")
            .header("Connection", "keep-alive");

        // Add custom headers from config
        for (key, value) in &self.config.headers {
            request = request.header(key, value);
        }

        // Send request and get stream
        let response = request.send().await
            .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?;

        // Validate response
        if !response.status().is_success() {
            return Err(TransportError::ConnectionFailed(
                format!("HTTP error: {}", response.status())
            ));
        }

        // Check content type
        let content_type = response.headers()
            .get("content-type")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("");

        if !content_type.contains("text/event-stream") {
            return Err(TransportError::ProtocolError(
                "Invalid content type for SSE".to_string()
            ));
        }

        // Start streaming
        self.start_streaming(response).await?;
        *self.state.lock().unwrap() = ConnectionState::Connected;

        Ok(())
    }

    async fn start_streaming(&mut self, response: reqwest::Response) -> Result<(), TransportError> {
        let mut stream = response.bytes_stream();
        let (tx, rx) = mpsc::unbounded_channel();

        self.event_sender = Some(tx);
        self.event_receiver = Some(rx);

        // Spawn streaming task
        let state = self.state.clone();
        let subscribed_types = self.subscribed_event_types.clone();
        let event_handlers = self.event_handlers.clone();
        let heartbeat_config = self.heartbeat_config.clone();
        let last_heartbeat = self.last_heartbeat.clone();

        tokio::spawn(async move {
            let mut buffer = String::new();
            let mut event_buffer = String::new();

            while let Some(chunk) = stream.next().await {
                match chunk {
                    Ok(bytes) => {
                        let chunk_str = String::from_utf8_lossy(&bytes);
                        buffer.push_str(&chunk_str);

                        // Process complete events
                        while let Some(event_end) = buffer.find("\n\n") {
                            let event_data = buffer[..event_end].to_string();
                            buffer = buffer[event_end + 2..].to_string();

                            if let Ok(event) = Self::parse_sse_event(&event_data) {
                                // Check if we're subscribed to this event type
                                let subscribed = subscribed_types.lock().unwrap();
                                if event.event_type.is_empty() ||
                                   subscribed.contains(&event.event_type) ||
                                   subscribed.is_empty() {

                                    // Create message from event
                                    let message = Message {
                                        data: event.data.as_bytes().to_vec(),
                                        message_type: MessageType::Text,
                                    };

                                    // Send to channel
                                    if let Some(sender) = &tx {
                                        let _ = sender.send(message);
                                    }

                                    // Update heartbeat
                                    *last_heartbeat.lock().unwrap() = Some(Instant::now());
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("SSE stream error: {}", e);
                        *state.lock().unwrap() = ConnectionState::Failed;
                        break;
                    }
                }
            }
        });

        Ok(())
    }
}
```

#### **1.2 SSE Event Parsing**

```rust
impl SseConnection {
    fn parse_sse_event(data: &str) -> Result<SseEvent, TransportError> {
        let mut event = SseEvent {
            event_type: String::new(),
            data: String::new(),
            id: None,
            retry: None,
        };

        for line in data.lines() {
            if line.is_empty() {
                continue;
            }

            if let Some(colon_pos) = line.find(':') {
                let field = &line[..colon_pos];
                let value = if colon_pos + 1 < line.len() {
                    &line[colon_pos + 1..]
                } else {
                    ""
                };

                match field {
                    "event" => event.event_type = value.to_string(),
                    "data" => {
                        if !event.data.is_empty() {
                            event.data.push('\n');
                        }
                        event.data.push_str(value);
                    }
                    "id" => event.id = Some(value.to_string()),
                    "retry" => {
                        if let Ok(retry_ms) = value.parse::<u64>() {
                            event.retry = Some(retry_ms);
                        }
                    }
                    _ => {
                        // Unknown field, ignore
                    }
                }
            }
        }

        Ok(event)
    }
}
```

### **Phase 2: Reconnection System**

#### **2.1 Automatic Reconnection**

```rust
impl SseConnection {
    pub async fn start_reconnection(&mut self, url: &str) -> Result<(), TransportError> {
        let strategy = self.reconnection_strategy.lock().unwrap().clone();

        match strategy {
            ReconnectionStrategy::None => {
                Err(TransportError::ConnectionFailed("Reconnection disabled".to_string()))
            }
            ReconnectionStrategy::Immediate => {
                self.connect(url).await
            }
            ReconnectionStrategy::ExponentialBackoff { base_delay, max_delay, max_attempts } => {
                self.reconnect_with_exponential_backoff(url, base_delay, max_delay, max_attempts).await
            }
            ReconnectionStrategy::LinearBackoff { delay, max_attempts } => {
                self.reconnect_with_linear_backoff(url, delay, max_attempts).await
            }
        }
    }

    async fn reconnect_with_exponential_backoff(
        &mut self,
        url: &str,
        base_delay: Duration,
        max_delay: Duration,
        max_attempts: u32,
    ) -> Result<(), TransportError> {
        let mut attempt = 0;

        while attempt < max_attempts {
            attempt += 1;

            // Calculate delay with exponential backoff
            let delay_ms = base_delay.as_millis() as u64 * 2_u64.pow(attempt - 1);
            let delay = Duration::from_millis(delay_ms.min(max_delay.as_millis() as u64));

            tokio::time::sleep(delay).await;

            match self.connect(url).await {
                Ok(()) => return Ok(()),
                Err(e) => {
                    eprintln!("SSE reconnection attempt {} failed: {}", attempt, e);
                    if attempt >= max_attempts {
                        return Err(e);
                    }
                }
            }
        }

        Err(TransportError::ConnectionFailed("Max reconnection attempts exceeded".to_string()))
    }

    async fn reconnect_with_linear_backoff(
        &mut self,
        url: &str,
        delay: Duration,
        max_attempts: u32,
    ) -> Result<(), TransportError> {
        for attempt in 1..=max_attempts {
            tokio::time::sleep(delay).await;

            match self.connect(url).await {
                Ok(()) => return Ok(()),
                Err(e) => {
                    eprintln!("SSE reconnection attempt {} failed: {}", attempt, e);
                    if attempt >= max_attempts {
                        return Err(e);
                    }
                }
            }
        }

        Err(TransportError::ConnectionFailed("Max reconnection attempts exceeded".to_string()))
    }
}
```

### **Phase 3: Heartbeat Mechanism**

#### **3.1 Heartbeat Monitoring**

```rust
impl SseConnection {
    pub async fn start_heartbeat_monitoring(&mut self) {
        let heartbeat_config = self.heartbeat_config.clone();
        let last_heartbeat = self.last_heartbeat.clone();
        let state = self.state.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(heartbeat_config.lock().unwrap().interval);

            loop {
                interval.tick().await;

                let config = heartbeat_config.lock().unwrap();
                if !config.enabled {
                    continue;
                }

                let last_heartbeat_time = *last_heartbeat.lock().unwrap();
                if let Some(last_time) = last_heartbeat_time {
                    if last_time.elapsed() > config.timeout {
                        eprintln!("SSE heartbeat timeout");
                        *state.lock().unwrap() = ConnectionState::Failed;
                        break;
                    }
                }
            }
        });
    }

    pub async fn receive_heartbeat(&mut self) -> Result<HeartbeatEvent, TransportError> {
        // Wait for heartbeat event or timeout
        let timeout = Duration::from_secs(30);
        let start_time = Instant::now();

        while start_time.elapsed() < timeout {
            if let Some(receiver) = &mut self.event_receiver {
                if let Ok(message) = receiver.try_recv() {
                    // Check if this is a heartbeat event
                    if let Ok(event) = Self::parse_sse_event(&String::from_utf8_lossy(&message.data)) {
                        if event.event_type == "heartbeat" {
                            *self.last_heartbeat.lock().unwrap() = Some(Instant::now());
                            return Ok(HeartbeatEvent {
                                timestamp: Instant::now(),
                                event_type: event.event_type,
                                data: event.data,
                            });
                        }
                    }
                }
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        Err(TransportError::ConnectionFailed("Heartbeat timeout".to_string()))
    }
}
```

### **Phase 4: Event Subscription Management**

#### **4.1 Event Type Filtering**

```rust
impl SseConnection {
    pub fn subscribe_to_event_type(&mut self, event_type: String) {
        self.subscribed_event_types.lock().unwrap().insert(event_type);
    }

    pub fn unsubscribe_from_event_type(&mut self, event_type: &str) {
        self.subscribed_event_types.lock().unwrap().remove(event_type);
    }

    pub fn get_subscribed_event_types(&self) -> Vec<String> {
        self.subscribed_event_types.lock().unwrap().iter().cloned().collect()
    }

    pub fn set_event_handler<F>(&mut self, event_type: String, handler: F)
    where
        F: Fn(Message) + Send + Sync + 'static,
    {
        self.event_handlers.lock().unwrap().insert(
            event_type,
            Box::new(handler)
        );
    }
}
```

## 🧪 **Testing Strategy**

### **Unit Tests**

- SSE event parsing
- Reconnection strategies
- Heartbeat mechanism
- Event type subscription
- Error handling

### **Integration Tests**

- Real SSE server communication
- Network interruption handling
- Long-running connection stability
- Event filtering and handling

### **Performance Tests**

- Connection establishment time
- Event processing throughput
- Memory usage under load
- Reconnection speed

## ✅ **Success Criteria**

### **Functionality**

- ✅ Real HTTP streaming with SSE
- ✅ Automatic reconnection with configurable strategies
- ✅ Heartbeat monitoring and timeout detection
- ✅ Event type subscription and filtering
- ✅ Proper error handling and recovery

### **Performance**

- ✅ < 200ms connection establishment
- ✅ > 100 events/second processing
- ✅ < 500KB memory usage per connection
- ✅ < 2 seconds reconnection time

### **Reliability**

- ✅ Handles network interruptions gracefully
- ✅ Recovers from server restarts
- ✅ Maintains event ordering
- ✅ No memory leaks during long runs

## 🚀 **Implementation Timeline**

- **Day 1-2**: HTTP streaming and event parsing
- **Day 3-4**: Reconnection system
- **Day 5-6**: Heartbeat mechanism
- **Day 7**: Event subscription management
- **Day 8**: Testing and validation

---

**Priority: HIGH - This is a critical fallback transport for WebSocket connections.**
