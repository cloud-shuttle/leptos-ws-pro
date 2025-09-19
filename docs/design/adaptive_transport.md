# Adaptive Transport Implementation Design

## 🎯 **Objective**

Implement intelligent transport protocol negotiation and fallback system that automatically selects the best available transport (WebSocket → WebTransport → SSE) based on capabilities and performance.

## 📊 **Current State**

### **What's Working**

- ✅ Basic adaptive transport structure
- ✅ Transport capability detection
- ✅ Fallback chain definition
- ✅ Performance metrics framework

### **What's Missing**

- ❌ Real protocol negotiation logic
- ❌ Performance-based transport selection
- ❌ Dynamic fallback mechanisms
- ❌ Transport health monitoring
- ❌ Automatic transport switching

## 🏗 **Architecture Design**

### **Core Components**

```
AdaptiveTransport
├── ProtocolNegotiator (negotiates best transport)
├── PerformanceMonitor (tracks transport performance)
├── HealthChecker (monitors transport health)
├── FallbackManager (manages fallback strategies)
├── CapabilityDetector (detects available transports)
└── TransportSwitcher (switches between transports)
```

### **Selection Flow**

```
Capability Detection → Performance Analysis → Protocol Negotiation → Transport Selection → Health Monitoring
        ↑                    ↓                      ↓                    ↓                ↓
        └────────────────────┴──────────────────────┴────────────────────┴────────────────┘
```

## 🛠 **Implementation Plan**

### **Phase 1: Protocol Negotiation**

#### **1.1 Intelligent Protocol Negotiator**

```rust
pub struct ProtocolNegotiator {
    supported_protocols: Vec<TransportProtocol>,
    performance_history: HashMap<TransportProtocol, PerformanceHistory>,
    negotiation_strategies: Vec<NegotiationStrategy>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TransportProtocol {
    WebSocket,
    WebTransport,
    SSE,
}

#[derive(Debug, Clone)]
pub struct PerformanceHistory {
    pub success_rate: f64,
    pub average_latency: Duration,
    pub average_throughput: f64,
    pub connection_time: Duration,
    pub last_used: Instant,
    pub failure_count: u32,
    pub success_count: u32,
}

pub enum NegotiationStrategy {
    PerformanceBased,
    ReliabilityBased,
    LatencyBased,
    ThroughputBased,
    Hybrid,
}

impl ProtocolNegotiator {
    pub fn new() -> Self {
        Self {
            supported_protocols: vec![
                TransportProtocol::WebSocket,
                TransportProtocol::WebTransport,
                TransportProtocol::SSE,
            ],
            performance_history: HashMap::new(),
            negotiation_strategies: vec![NegotiationStrategy::Hybrid],
        }
    }

    pub async fn negotiate_protocol(
        &mut self,
        url: &str,
        requirements: &TransportRequirements,
    ) -> Result<TransportProtocol, NegotiationError> {
        // Detect available protocols
        let available_protocols = self.detect_available_protocols(url).await?;

        // Filter by requirements
        let compatible_protocols = self.filter_by_requirements(available_protocols, requirements);

        if compatible_protocols.is_empty() {
            return Err(NegotiationError::NoCompatibleProtocols);
        }

        // Select best protocol based on strategy
        let selected_protocol = self.select_best_protocol(compatible_protocols, requirements).await?;

        // Update performance history
        self.update_performance_history(selected_protocol.clone()).await;

        Ok(selected_protocol)
    }

    async fn detect_available_protocols(&self, url: &str) -> Result<Vec<TransportProtocol>, NegotiationError> {
        let mut available = Vec::new();

        // Check WebSocket support
        if self.is_websocket_supported(url).await {
            available.push(TransportProtocol::WebSocket);
        }

        // Check WebTransport support
        if self.is_webtransport_supported(url).await {
            available.push(TransportProtocol::WebTransport);
        }

        // Check SSE support
        if self.is_sse_supported(url).await {
            available.push(TransportProtocol::SSE);
        }

        if available.is_empty() {
            Err(NegotiationError::NoAvailableProtocols)
        } else {
            Ok(available)
        }
    }

    async fn is_websocket_supported(&self, url: &str) -> bool {
        // Check if URL supports WebSocket
        if let Ok(parsed_url) = url::Url::parse(url) {
            match parsed_url.scheme() {
                "ws" | "wss" => true,
                _ => false,
            }
        } else {
            false
        }
    }

    async fn is_webtransport_supported(&self, url: &str) -> bool {
        // Check if URL supports WebTransport (HTTPS required)
        if let Ok(parsed_url) = url::Url::parse(url) {
            if parsed_url.scheme() == "https" {
                // In a real implementation, this would check for WebTransport support
                // For now, we'll assume it's supported on HTTPS
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    async fn is_sse_supported(&self, url: &str) -> bool {
        // SSE is supported on HTTP/HTTPS
        if let Ok(parsed_url) = url::Url::parse(url) {
            match parsed_url.scheme() {
                "http" | "https" => true,
                _ => false,
            }
        } else {
            false
        }
    }

    fn filter_by_requirements(
        &self,
        protocols: Vec<TransportProtocol>,
        requirements: &TransportRequirements,
    ) -> Vec<TransportProtocol> {
        protocols.into_iter().filter(|protocol| {
            match protocol {
                TransportProtocol::WebSocket => {
                    requirements.bidirectional && requirements.realtime
                }
                TransportProtocol::WebTransport => {
                    requirements.bidirectional && requirements.realtime && requirements.multiplexing
                }
                TransportProtocol::SSE => {
                    !requirements.bidirectional && requirements.realtime
                }
            }
        }).collect()
    }

    async fn select_best_protocol(
        &self,
        protocols: Vec<TransportProtocol>,
        requirements: &TransportRequirements,
    ) -> Result<TransportProtocol, NegotiationError> {
        let mut scored_protocols: Vec<(TransportProtocol, f64)> = Vec::new();

        for protocol in protocols {
            let score = self.calculate_protocol_score(&protocol, requirements).await;
            scored_protocols.push((protocol, score));
        }

        // Sort by score (highest first)
        scored_protocols.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        if let Some((best_protocol, _)) = scored_protocols.first() {
            Ok(best_protocol.clone())
        } else {
            Err(NegotiationError::NoSuitableProtocol)
        }
    }

    async fn calculate_protocol_score(
        &self,
        protocol: &TransportProtocol,
        requirements: &TransportRequirements,
    ) -> f64 {
        let mut score = 0.0;

        // Base score for protocol type
        match protocol {
            TransportProtocol::WebSocket => score += 0.8,
            TransportProtocol::WebTransport => score += 0.9,
            TransportProtocol::SSE => score += 0.6,
        }

        // Performance-based scoring
        if let Some(history) = self.performance_history.get(protocol) {
            score += history.success_rate * 0.3;
            score += (1.0 - history.average_latency.as_millis() as f64 / 1000.0) * 0.2;
            score += (history.average_throughput / 1000.0) * 0.1;
        }

        // Requirements-based scoring
        if requirements.bidirectional {
            match protocol {
                TransportProtocol::WebSocket | TransportProtocol::WebTransport => score += 0.2,
                _ => score -= 0.3,
            }
        }

        if requirements.realtime {
            match protocol {
                TransportProtocol::WebSocket | TransportProtocol::WebTransport => score += 0.2,
                TransportProtocol::SSE => score += 0.1,
            }
        }

        if requirements.multiplexing {
            match protocol {
                TransportProtocol::WebTransport => score += 0.2,
                _ => score -= 0.1,
            }
        }

        score
    }

    async fn update_performance_history(&mut self, protocol: TransportProtocol) {
        // This would be called after successful connection
        let history = self.performance_history.entry(protocol).or_insert_with(|| {
            PerformanceHistory {
                success_rate: 1.0,
                average_latency: Duration::from_millis(100),
                average_throughput: 1000.0,
                connection_time: Duration::from_millis(50),
                last_used: Instant::now(),
                failure_count: 0,
                success_count: 0,
            }
        });

        history.last_used = Instant::now();
    }
}
```

### **Phase 2: Performance Monitoring**

#### **2.1 Transport Performance Monitor**

```rust
pub struct PerformanceMonitor {
    transport_metrics: HashMap<TransportProtocol, TransportMetrics>,
    monitoring_interval: Duration,
    performance_thresholds: PerformanceThresholds,
}

#[derive(Debug, Clone)]
pub struct TransportMetrics {
    pub protocol: TransportProtocol,
    pub connection_count: u64,
    pub successful_connections: u64,
    pub failed_connections: u64,
    pub average_connection_time: Duration,
    pub average_latency: Duration,
    pub average_throughput: f64,
    pub error_rate: f64,
    pub last_connection_time: Option<Instant>,
    pub last_error_time: Option<Instant>,
}

#[derive(Debug, Clone)]
pub struct PerformanceThresholds {
    pub max_latency: Duration,
    pub min_throughput: f64,
    pub max_error_rate: f64,
    pub max_connection_time: Duration,
}

impl PerformanceMonitor {
    pub fn new(monitoring_interval: Duration) -> Self {
        Self {
            transport_metrics: HashMap::new(),
            monitoring_interval,
            performance_thresholds: PerformanceThresholds {
                max_latency: Duration::from_millis(500),
                min_throughput: 100.0,
                max_error_rate: 0.1,
                max_connection_time: Duration::from_secs(5),
            },
        }
    }

    pub fn record_connection_attempt(&mut self, protocol: TransportProtocol) {
        let metrics = self.transport_metrics.entry(protocol.clone()).or_insert_with(|| {
            TransportMetrics::new(protocol)
        });

        metrics.connection_count += 1;
        metrics.last_connection_time = Some(Instant::now());
    }

    pub fn record_connection_success(
        &mut self,
        protocol: TransportProtocol,
        connection_time: Duration,
        latency: Duration,
        throughput: f64,
    ) {
        if let Some(metrics) = self.transport_metrics.get_mut(&protocol) {
            metrics.successful_connections += 1;
            metrics.average_connection_time = self.update_average(
                metrics.average_connection_time,
                connection_time,
                metrics.successful_connections,
            );
            metrics.average_latency = self.update_average(
                metrics.average_latency,
                latency,
                metrics.successful_connections,
            );
            metrics.average_throughput = self.update_average_f64(
                metrics.average_throughput,
                throughput,
                metrics.successful_connections,
            );
            metrics.error_rate = metrics.failed_connections as f64 / metrics.connection_count as f64;
        }
    }

    pub fn record_connection_failure(&mut self, protocol: TransportProtocol) {
        if let Some(metrics) = self.transport_metrics.get_mut(&protocol) {
            metrics.failed_connections += 1;
            metrics.last_error_time = Some(Instant::now());
            metrics.error_rate = metrics.failed_connections as f64 / metrics.connection_count as f64;
        }
    }

    pub fn is_transport_healthy(&self, protocol: &TransportProtocol) -> bool {
        if let Some(metrics) = self.transport_metrics.get(protocol) {
            metrics.average_latency <= self.performance_thresholds.max_latency &&
            metrics.average_throughput >= self.performance_thresholds.min_throughput &&
            metrics.error_rate <= self.performance_thresholds.max_error_rate &&
            metrics.average_connection_time <= self.performance_thresholds.max_connection_time
        } else {
            true // Assume healthy if no data
        }
    }

    pub fn get_best_transport(&self) -> Option<TransportProtocol> {
        let mut best_transport = None;
        let mut best_score = 0.0;

        for (protocol, metrics) in &self.transport_metrics {
            if self.is_transport_healthy(protocol) {
                let score = self.calculate_transport_score(metrics);
                if score > best_score {
                    best_score = score;
                    best_transport = Some(protocol.clone());
                }
            }
        }

        best_transport
    }

    fn calculate_transport_score(&self, metrics: &TransportMetrics) -> f64 {
        let mut score = 0.0;

        // Success rate (40% weight)
        score += (1.0 - metrics.error_rate) * 0.4;

        // Latency (30% weight)
        let latency_score = 1.0 - (metrics.average_latency.as_millis() as f64 / 1000.0);
        score += latency_score.max(0.0) * 0.3;

        // Throughput (20% weight)
        let throughput_score = (metrics.average_throughput / 1000.0).min(1.0);
        score += throughput_score * 0.2;

        // Connection time (10% weight)
        let connection_score = 1.0 - (metrics.average_connection_time.as_millis() as f64 / 5000.0);
        score += connection_score.max(0.0) * 0.1;

        score
    }

    fn update_average(&self, current: Duration, new: Duration, count: u64) -> Duration {
        let current_ms = current.as_millis() as u64;
        let new_ms = new.as_millis() as u64;
        let avg_ms = (current_ms * (count - 1) + new_ms) / count;
        Duration::from_millis(avg_ms)
    }

    fn update_average_f64(&self, current: f64, new: f64, count: u64) -> f64 {
        (current * (count - 1) as f64 + new) / count as f64
    }
}
```

### **Phase 3: Dynamic Fallback**

#### **3.1 Intelligent Fallback Manager**

```rust
pub struct FallbackManager {
    fallback_chain: Vec<TransportProtocol>,
    current_transport: Option<TransportProtocol>,
    fallback_history: VecDeque<FallbackEvent>,
    fallback_strategies: HashMap<FallbackReason, FallbackStrategy>,
}

#[derive(Debug, Clone)]
pub struct FallbackEvent {
    pub timestamp: Instant,
    pub from_transport: TransportProtocol,
    pub to_transport: TransportProtocol,
    pub reason: FallbackReason,
    pub success: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FallbackReason {
    ConnectionFailed,
    PerformanceDegraded,
    HealthCheckFailed,
    UserRequested,
    Maintenance,
}

#[derive(Debug, Clone)]
pub enum FallbackStrategy {
    Immediate,
    Delayed { delay: Duration },
    Gradual { steps: Vec<TransportProtocol> },
    Conditional { condition: Box<dyn Fn() -> bool + Send + Sync> },
}

impl FallbackManager {
    pub fn new() -> Self {
        let mut fallback_strategies = HashMap::new();

        // Define fallback strategies for different reasons
        fallback_strategies.insert(
            FallbackReason::ConnectionFailed,
            FallbackStrategy::Immediate,
        );

        fallback_strategies.insert(
            FallbackReason::PerformanceDegraded,
            FallbackStrategy::Delayed { delay: Duration::from_secs(5) },
        );

        fallback_strategies.insert(
            FallbackReason::HealthCheckFailed,
            FallbackStrategy::Immediate,
        );

        Self {
            fallback_chain: vec![
                TransportProtocol::WebSocket,
                TransportProtocol::WebTransport,
                TransportProtocol::SSE,
            ],
            current_transport: None,
            fallback_history: VecDeque::new(),
            fallback_strategies,
        }
    }

    pub async fn initiate_fallback(
        &mut self,
        reason: FallbackReason,
        performance_monitor: &PerformanceMonitor,
    ) -> Result<TransportProtocol, FallbackError> {
        let current_transport = self.current_transport.clone()
            .ok_or(FallbackError::NoCurrentTransport)?;

        // Get fallback strategy
        let strategy = self.fallback_strategies.get(&reason)
            .ok_or(FallbackError::NoStrategyForReason)?;

        // Find next transport in chain
        let next_transport = self.find_next_transport(&current_transport, performance_monitor)?;

        // Execute fallback strategy
        match strategy {
            FallbackStrategy::Immediate => {
                self.execute_fallback(current_transport, next_transport, reason).await
            }
            FallbackStrategy::Delayed { delay } => {
                tokio::time::sleep(*delay).await;
                self.execute_fallback(current_transport, next_transport, reason).await
            }
            FallbackStrategy::Gradual { steps } => {
                self.execute_gradual_fallback(current_transport, steps.clone(), reason).await
            }
            FallbackStrategy::Conditional { condition } => {
                if condition() {
                    self.execute_fallback(current_transport, next_transport, reason).await
                } else {
                    Err(FallbackError::ConditionNotMet)
                }
            }
        }
    }

    fn find_next_transport(
        &self,
        current: &TransportProtocol,
        performance_monitor: &PerformanceMonitor,
    ) -> Result<TransportProtocol, FallbackError> {
        // Find current transport in chain
        let current_index = self.fallback_chain.iter()
            .position(|p| p == current)
            .ok_or(FallbackError::TransportNotInChain)?;

        // Find next healthy transport
        for i in (current_index + 1)..self.fallback_chain.len() {
            let transport = &self.fallback_chain[i];
            if performance_monitor.is_transport_healthy(transport) {
                return Ok(transport.clone());
            }
        }

        Err(FallbackError::NoHealthyTransportAvailable)
    }

    async fn execute_fallback(
        &mut self,
        from: TransportProtocol,
        to: TransportProtocol,
        reason: FallbackReason,
    ) -> Result<TransportProtocol, FallbackError> {
        // Record fallback event
        let event = FallbackEvent {
            timestamp: Instant::now(),
            from_transport: from.clone(),
            to_transport: to.clone(),
            reason: reason.clone(),
            success: false, // Will be updated after connection attempt
        };

        self.fallback_history.push_back(event);

        // Update current transport
        self.current_transport = Some(to.clone());

        Ok(to)
    }

    async fn execute_gradual_fallback(
        &mut self,
        from: TransportProtocol,
        steps: Vec<TransportProtocol>,
        reason: FallbackReason,
    ) -> Result<TransportProtocol, FallbackError> {
        let mut current = from;

        for step in steps {
            match self.execute_fallback(current, step.clone(), reason.clone()).await {
                Ok(transport) => {
                    // Test the new transport
                    if self.test_transport(&transport).await {
                        return Ok(transport);
                    }
                    current = transport;
                }
                Err(e) => {
                    // Continue to next step
                    continue;
                }
            }
        }

        Err(FallbackError::AllFallbackStepsFailed)
    }

    async fn test_transport(&self, transport: &TransportProtocol) -> bool {
        // In a real implementation, this would test the transport
        // For now, we'll assume all transports are testable
        true
    }

    pub fn get_fallback_history(&self) -> &VecDeque<FallbackEvent> {
        &self.fallback_history
    }

    pub fn should_consider_fallback(&self, performance_monitor: &PerformanceMonitor) -> bool {
        if let Some(current) = &self.current_transport {
            !performance_monitor.is_transport_healthy(current)
        } else {
            true
        }
    }
}
```

## 🧪 **Testing Strategy**

### **Unit Tests**

- Protocol negotiation logic
- Performance monitoring
- Fallback strategies
- Transport selection algorithms

### **Integration Tests**

- End-to-end adaptive transport
- Real network conditions
- Performance under load
- Fallback scenarios

### **Performance Tests**

- Transport selection speed
- Fallback execution time
- Performance monitoring overhead
- Memory usage

## ✅ **Success Criteria**

### **Functionality**

- ✅ Intelligent protocol negotiation
- ✅ Performance-based transport selection
- ✅ Dynamic fallback mechanisms
- ✅ Transport health monitoring
- ✅ Automatic transport switching

### **Performance**

- ✅ < 100ms protocol negotiation
- ✅ < 1 second fallback execution
- ✅ < 5% performance monitoring overhead
- ✅ 99.9% transport selection accuracy
- ✅ < 1MB memory usage

### **Reliability**

- ✅ Handles network changes gracefully
- ✅ Recovers from transport failures
- ✅ Maintains connection during fallback
- ✅ Prevents cascade failures
- ✅ Monitors and reports performance

## 🚀 **Implementation Timeline**

- **Day 1-2**: Protocol negotiation system
- **Day 3-4**: Performance monitoring
- **Day 5-6**: Dynamic fallback mechanisms
- **Day 7**: Transport switching
- **Day 8**: Testing and optimization

---

**Priority: MEDIUM - Adaptive transport is a nice-to-have feature for production environments.**
