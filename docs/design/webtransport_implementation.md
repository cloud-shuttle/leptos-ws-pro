# WebTransport Implementation Design

## 🎯 **Objective**

Implement a complete WebTransport client with HTTP/3 support, bidirectional streams, reliability modes, and congestion control.

## 📊 **Current State**

### **What's Working**

- ✅ Basic WebTransport structure and types
- ✅ Stream configuration definitions
- ✅ Reliability mode enums
- ✅ Congestion control types

### **What's Missing**

- ❌ Real HTTP/3 connection establishment
- ❌ Bidirectional stream creation and management
- ❌ Data sending and receiving
- ❌ Stream reliability handling
- ❌ Congestion control implementation
- ❌ Performance monitoring

## 🏗 **Architecture Design**

### **Core Components**

```
WebTransportConnection
├── Http3Manager (handles HTTP/3 connections)
├── StreamManager (manages bidirectional streams)
├── ReliabilityEngine (handles reliability modes)
├── CongestionController (manages congestion control)
├── PerformanceMonitor (tracks stream performance)
└── ErrorHandler (handles WebTransport errors)
```

### **Stream Flow**

```
HTTP/3 Connection → Stream Creation → Data Transfer → Reliability → Congestion Control
        ↑               ↓                ↓              ↓              ↓
        └───────────────┴────────────────┴──────────────┴──────────────┘
```

## 🛠 **Implementation Plan**

### **Phase 1: HTTP/3 Connection Management**

#### **1.1 Real HTTP/3 Connection**

```rust
impl WebTransportConnection {
    pub async fn connect(&mut self, url: &str) -> Result<(), TransportError> {
        *self.state.lock().unwrap() = ConnectionState::Connecting;

        // Parse URL
        let parsed_url = url::Url::parse(url)
            .map_err(|e| TransportError::ConnectionFailed(format!("Invalid URL: {}", e)))?;

        // Validate HTTPS requirement
        if parsed_url.scheme() != "https" {
            return Err(TransportError::ProtocolError(
                "WebTransport requires HTTPS".to_string()
            ));
        }

        // Create HTTP/3 client
        let client = self.create_http3_client().await?;

        // Establish WebTransport connection
        let connection = self.establish_webtransport_connection(&client, &parsed_url).await?;

        self.http3_client = Some(client);
        self.connection = Some(connection);
        *self.state.lock().unwrap() = ConnectionState::Connected;

        Ok(())
    }

    async fn create_http3_client(&self) -> Result<reqwest::Client, TransportError> {
        // Create HTTP/3 client with proper configuration
        let client = reqwest::Client::builder()
            .http3_prior_knowledge()
            .timeout(self.config.connection_timeout)
            .build()
            .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?;

        Ok(client)
    }

    async fn establish_webtransport_connection(
        &self,
        client: &reqwest::Client,
        url: &url::Url,
    ) -> Result<WebTransportSession, TransportError> {
        // Create WebTransport request
        let mut request = client
            .request(reqwest::Method::CONNECT, url.as_str())
            .header("Sec-WebTransport-HTTP3-Draft", "draft02")
            .header("Connection", "Upgrade")
            .header("Upgrade", "webtransport");

        // Add custom headers
        for (key, value) in &self.config.headers {
            request = request.header(key, value);
        }

        // Send request
        let response = request.send().await
            .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?;

        // Validate response
        if !response.status().is_success() {
            return Err(TransportError::ConnectionFailed(
                format!("WebTransport connection failed: {}", response.status())
            ));
        }

        // Check for WebTransport headers
        let upgrade = response.headers().get("upgrade")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("");

        if !upgrade.contains("webtransport") {
            return Err(TransportError::ProtocolError(
                "Invalid WebTransport response".to_string()
            ));
        }

        // Create WebTransport session
        let session = WebTransportSession {
            url: url.clone(),
            established_at: Instant::now(),
            streams: HashMap::new(),
            next_stream_id: 1,
        };

        Ok(session)
    }
}
```

#### **1.2 WebTransport Session Management**

```rust
pub struct WebTransportSession {
    pub url: url::Url,
    pub established_at: Instant,
    pub streams: HashMap<u32, WebTransportStream>,
    pub next_stream_id: u32,
}

pub struct WebTransportStream {
    pub id: u32,
    pub stream: BidirectionalStream,
    pub config: StreamConfig,
    pub reliability: ReliabilityMode,
    pub ordering: OrderingMode,
    pub congestion_control: CongestionControl,
    pub created_at: Instant,
    pub last_used: Instant,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub messages_sent: u64,
    pub messages_received: u64,
}

impl WebTransportConnection {
    pub async fn create_stream(
        &self,
        stream_config: StreamConfig,
    ) -> Result<AdvancedWebTransportStream, TransportError> {
        let connection = self.connection.as_ref()
            .ok_or_else(|| TransportError::NotConnected)?;

        let stream_id = connection.next_stream_id;

        // Create bidirectional stream
        let stream = self.create_bidirectional_stream(stream_id).await?;

        // Create WebTransport stream
        let webtransport_stream = WebTransportStream {
            id: stream_id,
            stream,
            config: stream_config.clone(),
            reliability: stream_config.reliability,
            ordering: stream_config.ordering,
            congestion_control: stream_config.congestion_control,
            created_at: Instant::now(),
            last_used: Instant::now(),
            bytes_sent: 0,
            bytes_received: 0,
            messages_sent: 0,
            messages_received: 0,
        };

        // Add to connection
        self.connection.as_ref().unwrap().streams.insert(stream_id, webtransport_stream);

        // Create advanced stream wrapper
        let advanced_stream = AdvancedWebTransportStream {
            stream_id,
            reliability: stream_config.reliability,
            ordering: stream_config.ordering,
            congestion_control: stream_config.congestion_control,
            is_active: true,
            can_send: true,
            can_receive: true,
            send_latency: Duration::from_millis(0),
            delivery_guaranteed: stream_config.reliability == ReliabilityMode::Reliable,
            max_retransmissions: stream_config.max_retransmissions,
            retransmission_count: 0,
            average_send_rate: 0.0,
            last_used: Instant::now(),
        };

        Ok(advanced_stream)
    }

    async fn create_bidirectional_stream(&self, stream_id: u32) -> Result<BidirectionalStream, TransportError> {
        // In a real implementation, this would create an actual HTTP/3 bidirectional stream
        // For now, we'll create a mock stream that simulates the behavior

        let (send_tx, send_rx) = mpsc::unbounded_channel();
        let (recv_tx, recv_rx) = mpsc::unbounded_channel();

        let stream = BidirectionalStream {
            stream_id,
            send_tx,
            send_rx,
            recv_tx,
            recv_rx,
        };

        Ok(stream)
    }
}
```

### **Phase 2: Stream Reliability Implementation**

#### **2.1 Reliability Engine**

```rust
pub struct ReliabilityEngine {
    reliable_streams: HashMap<u32, ReliableStreamState>,
    unreliable_streams: HashMap<u32, UnreliableStreamState>,
    retransmission_queue: VecDeque<RetransmissionEntry>,
}

pub struct ReliableStreamState {
    pub stream_id: u32,
    pub sent_messages: HashMap<u64, SentMessage>,
    pub received_messages: HashSet<u64>,
    pub next_message_id: u64,
    pub ack_timeout: Duration,
    pub max_retransmissions: u32,
}

pub struct SentMessage {
    pub message_id: u64,
    pub data: Vec<u8>,
    pub sent_at: Instant,
    pub retransmission_count: u32,
    pub acked: bool,
}

impl ReliabilityEngine {
    pub fn new() -> Self {
        Self {
            reliable_streams: HashMap::new(),
            unreliable_streams: HashMap::new(),
            retransmission_queue: VecDeque::new(),
        }
    }

    pub async fn send_reliable_message(
        &mut self,
        stream_id: u32,
        data: Vec<u8>,
    ) -> Result<(), TransportError> {
        let stream_state = self.reliable_streams.entry(stream_id).or_insert_with(|| {
            ReliableStreamState {
                stream_id,
                sent_messages: HashMap::new(),
                received_messages: HashSet::new(),
                next_message_id: 1,
                ack_timeout: Duration::from_millis(1000),
                max_retransmissions: 3,
            }
        });

        let message_id = stream_state.next_message_id;
        stream_state.next_message_id += 1;

        let sent_message = SentMessage {
            message_id,
            data: data.clone(),
            sent_at: Instant::now(),
            retransmission_count: 0,
            acked: false,
        };

        stream_state.sent_messages.insert(message_id, sent_message);

        // Send message with reliability header
        let reliable_message = ReliableMessage {
            message_id,
            data,
            stream_id,
            is_ack: false,
        };

        self.send_message(reliable_message).await?;

        // Schedule retransmission check
        self.schedule_retransmission_check(stream_id, message_id).await;

        Ok(())
    }

    pub async fn handle_ack(&mut self, stream_id: u32, message_id: u64) -> Result<(), TransportError> {
        if let Some(stream_state) = self.reliable_streams.get_mut(&stream_id) {
            if let Some(sent_message) = stream_state.sent_messages.get_mut(&message_id) {
                sent_message.acked = true;
            }
        }

        Ok(())
    }

    pub async fn handle_received_message(
        &mut self,
        stream_id: u32,
        message: ReliableMessage,
    ) -> Result<Vec<u8>, TransportError> {
        if message.is_ack {
            // Handle acknowledgment
            self.handle_ack(stream_id, message.message_id).await?;
            return Ok(vec![]);
        }

        // Send acknowledgment
        let ack_message = ReliableMessage {
            message_id: message.message_id,
            data: vec![],
            stream_id,
            is_ack: true,
        };

        self.send_message(ack_message).await?;

        // Check if we've already received this message
        if let Some(stream_state) = self.reliable_streams.get_mut(&stream_id) {
            if stream_state.received_messages.contains(&message.message_id) {
                return Ok(vec![]); // Duplicate message
            }

            stream_state.received_messages.insert(message.message_id);
        }

        Ok(message.data)
    }

    async fn schedule_retransmission_check(&self, stream_id: u32, message_id: u64) {
        let retransmission_entry = RetransmissionEntry {
            stream_id,
            message_id,
            scheduled_time: Instant::now() + Duration::from_millis(1000),
        };

        // In a real implementation, this would be added to a retransmission queue
        // and processed by a background task
    }

    async fn send_message(&self, message: ReliableMessage) -> Result<(), TransportError> {
        // In a real implementation, this would send the message over the WebTransport stream
        Ok(())
    }
}
```

### **Phase 3: Congestion Control**

#### **3.1 Congestion Controller**

```rust
pub struct CongestionController {
    streams: HashMap<u32, StreamCongestionState>,
    global_state: GlobalCongestionState,
    algorithm: CongestionControlAlgorithm,
}

pub struct StreamCongestionState {
    pub stream_id: u32,
    pub congestion_window: u32,
    pub slow_start_threshold: u32,
    pub rtt: Duration,
    pub rtt_variance: Duration,
    pub bytes_in_flight: u32,
    pub last_ack_time: Instant,
    pub packets_lost: u32,
    pub packets_sent: u32,
}

pub struct GlobalCongestionState {
    pub total_bytes_in_flight: u32,
    pub global_congestion_window: u32,
    pub estimated_bandwidth: f64,
    pub estimated_rtt: Duration,
}

impl CongestionController {
    pub fn new(algorithm: CongestionControlAlgorithm) -> Self {
        Self {
            streams: HashMap::new(),
            global_state: GlobalCongestionState {
                total_bytes_in_flight: 0,
                global_congestion_window: 1000,
                estimated_bandwidth: 1.0,
                estimated_rtt: Duration::from_millis(100),
            },
            algorithm,
        }
    }

    pub fn can_send(&mut self, stream_id: u32, bytes: u32) -> bool {
        let stream_state = self.streams.entry(stream_id).or_insert_with(|| {
            StreamCongestionState {
                stream_id,
                congestion_window: 1000,
                slow_start_threshold: 10000,
                rtt: Duration::from_millis(100),
                rtt_variance: Duration::from_millis(10),
                bytes_in_flight: 0,
                last_ack_time: Instant::now(),
                packets_lost: 0,
                packets_sent: 0,
            }
        });

        stream_state.bytes_in_flight + bytes <= stream_state.congestion_window
    }

    pub fn on_packet_sent(&mut self, stream_id: u32, bytes: u32) {
        if let Some(stream_state) = self.streams.get_mut(&stream_id) {
            stream_state.bytes_in_flight += bytes;
            stream_state.packets_sent += 1;
        }

        self.global_state.total_bytes_in_flight += bytes;
    }

    pub fn on_packet_acked(&mut self, stream_id: u32, bytes: u32, rtt: Duration) {
        if let Some(stream_state) = self.streams.get_mut(&stream_id) {
            stream_state.bytes_in_flight = stream_state.bytes_in_flight.saturating_sub(bytes);
            stream_state.last_ack_time = Instant::now();

            // Update RTT
            self.update_rtt(stream_state, rtt);

            // Update congestion window
            self.update_congestion_window(stream_state);
        }

        self.global_state.total_bytes_in_flight =
            self.global_state.total_bytes_in_flight.saturating_sub(bytes);
    }

    pub fn on_packet_lost(&mut self, stream_id: u32, bytes: u32) {
        if let Some(stream_state) = self.streams.get_mut(&stream_id) {
            stream_state.bytes_in_flight = stream_state.bytes_in_flight.saturating_sub(bytes);
            stream_state.packets_lost += 1;

            // Reduce congestion window
            self.reduce_congestion_window(stream_state);
        }

        self.global_state.total_bytes_in_flight =
            self.global_state.total_bytes_in_flight.saturating_sub(bytes);
    }

    fn update_rtt(&self, stream_state: &mut StreamCongestionState, rtt: Duration) {
        // Update RTT using exponential moving average
        let alpha = 0.125;
        let new_rtt = stream_state.rtt.as_millis() as f64 * (1.0 - alpha) + rtt.as_millis() as f64 * alpha;
        stream_state.rtt = Duration::from_millis(new_rtt as u64);

        // Update RTT variance
        let rtt_diff = (rtt.as_millis() as f64 - stream_state.rtt.as_millis() as f64).abs();
        let new_variance = stream_state.rtt_variance.as_millis() as f64 * (1.0 - alpha) + rtt_diff * alpha;
        stream_state.rtt_variance = Duration::from_millis(new_variance as u64);
    }

    fn update_congestion_window(&self, stream_state: &mut StreamCongestionState) {
        match self.algorithm {
            CongestionControlAlgorithm::Cubic => {
                self.update_cubic_window(stream_state);
            }
            CongestionControlAlgorithm::BBR => {
                self.update_bbr_window(stream_state);
            }
            CongestionControlAlgorithm::Reno => {
                self.update_reno_window(stream_state);
            }
        }
    }

    fn update_cubic_window(&self, stream_state: &mut StreamCongestionState) {
        if stream_state.congestion_window < stream_state.slow_start_threshold {
            // Slow start
            stream_state.congestion_window += 1;
        } else {
            // Congestion avoidance
            stream_state.congestion_window += 1 / stream_state.congestion_window as f64;
        }
    }

    fn update_bbr_window(&self, stream_state: &mut StreamCongestionState) {
        // BBR (Bottleneck Bandwidth and RTT) algorithm
        let bdp = self.global_state.estimated_bandwidth * stream_state.rtt.as_secs_f64();
        stream_state.congestion_window = (bdp * 2.0) as u32;
    }

    fn update_reno_window(&self, stream_state: &mut StreamCongestionState) {
        if stream_state.congestion_window < stream_state.slow_start_threshold {
            // Slow start
            stream_state.congestion_window += 1;
        } else {
            // Congestion avoidance
            stream_state.congestion_window += 1;
        }
    }

    fn reduce_congestion_window(&self, stream_state: &mut StreamCongestionState) {
        stream_state.slow_start_threshold = stream_state.congestion_window / 2;
        stream_state.congestion_window = stream_state.slow_start_threshold;
    }
}
```

### **Phase 4: Performance Monitoring**

#### **4.1 Stream Performance Monitor**

```rust
pub struct PerformanceMonitor {
    stream_metrics: HashMap<u32, StreamMetrics>,
    global_metrics: GlobalMetrics,
    monitoring_interval: Duration,
}

pub struct StreamMetrics {
    pub stream_id: u32,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub messages_sent: u64,
    pub messages_received: u64,
    pub average_send_rate: f64,
    pub average_receive_rate: f64,
    pub retransmission_count: u32,
    pub rtt: Duration,
    pub jitter: Duration,
    pub packet_loss_rate: f64,
    pub throughput: f64,
}

impl PerformanceMonitor {
    pub fn new(monitoring_interval: Duration) -> Self {
        Self {
            stream_metrics: HashMap::new(),
            global_metrics: GlobalMetrics::new(),
            monitoring_interval,
        }
    }

    pub fn record_message_sent(&mut self, stream_id: u32, bytes: usize) {
        let metrics = self.stream_metrics.entry(stream_id).or_insert_with(|| {
            StreamMetrics::new(stream_id)
        });

        metrics.bytes_sent += bytes as u64;
        metrics.messages_sent += 1;

        self.global_metrics.total_bytes_sent += bytes as u64;
        self.global_metrics.total_messages_sent += 1;
    }

    pub fn record_message_received(&mut self, stream_id: u32, bytes: usize) {
        let metrics = self.stream_metrics.entry(stream_id).or_insert_with(|| {
            StreamMetrics::new(stream_id)
        });

        metrics.bytes_received += bytes as u64;
        metrics.messages_received += 1;

        self.global_metrics.total_bytes_received += bytes as u64;
        self.global_metrics.total_messages_received += 1;
    }

    pub fn update_rtt(&mut self, stream_id: u32, rtt: Duration) {
        if let Some(metrics) = self.stream_metrics.get_mut(&stream_id) {
            metrics.rtt = rtt;
        }
    }

    pub fn record_retransmission(&mut self, stream_id: u32) {
        if let Some(metrics) = self.stream_metrics.get_mut(&stream_id) {
            metrics.retransmission_count += 1;
        }

        self.global_metrics.total_retransmissions += 1;
    }

    pub fn calculate_throughput(&mut self, stream_id: u32) -> f64 {
        if let Some(metrics) = self.stream_metrics.get_mut(&stream_id) {
            let time_elapsed = metrics.created_at.elapsed().as_secs_f64();
            if time_elapsed > 0.0 {
                metrics.throughput = metrics.bytes_sent as f64 / time_elapsed;
            }
            metrics.throughput
        } else {
            0.0
        }
    }

    pub fn get_stream_metrics(&self, stream_id: u32) -> Option<&StreamMetrics> {
        self.stream_metrics.get(&stream_id)
    }

    pub fn get_global_metrics(&self) -> &GlobalMetrics {
        &self.global_metrics
    }
}
```

## 🧪 **Testing Strategy**

### **Unit Tests**

- HTTP/3 connection establishment
- Stream creation and management
- Reliability mechanisms
- Congestion control algorithms
- Performance monitoring

### **Integration Tests**

- Real WebTransport server communication
- Stream reliability under network conditions
- Congestion control under load
- Performance under various scenarios

### **Performance Tests**

- Stream creation time
- Data transfer throughput
- Reliability overhead
- Congestion control effectiveness

## ✅ **Success Criteria**

### **Functionality**

- ✅ Real HTTP/3 WebTransport connections
- ✅ Bidirectional stream creation and management
- ✅ Reliable and unreliable message delivery
- ✅ Congestion control with multiple algorithms
- ✅ Performance monitoring and metrics

### **Performance**

- ✅ < 50ms stream creation time
- ✅ > 1000 messages/second per stream
- ✅ < 5% reliability overhead
- ✅ < 10% congestion control overhead
- ✅ 99.9% message delivery reliability

### **Reliability**

- ✅ Handles network interruptions gracefully
- ✅ Recovers from stream failures
- ✅ Maintains message ordering
- ✅ Prevents congestion collapse
- ✅ Monitors and reports performance

## 🚀 **Implementation Timeline**

- **Day 1-2**: HTTP/3 connection management
- **Day 3-4**: Stream creation and management
- **Day 5-6**: Reliability implementation
- **Day 7**: Congestion control
- **Day 8**: Performance monitoring and testing

---

**Priority: MEDIUM - WebTransport is advanced but not critical for basic functionality.**
