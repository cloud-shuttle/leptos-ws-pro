//! Metrics and observability for leptos-ws
//! 
//! Provides comprehensive metrics collection, tracing, and monitoring
//! capabilities for production deployments.

use std::sync::Arc;
use std::time::{Duration, Instant};
use std::collections::HashMap;

#[cfg(feature = "metrics")]
use metrics::{Counter, Histogram, Gauge};

/// WebSocket connection metrics
#[derive(Debug, Clone)]
pub struct ConnectionMetrics {
    /// Number of messages sent
    pub messages_sent: u64,
    /// Number of messages received
    pub messages_received: u64,
    /// Total bytes sent
    pub bytes_sent: u64,
    /// Total bytes received
    pub bytes_received: u64,
    /// Current active connections
    pub active_connections: u64,
    /// Number of reconnection attempts
    pub reconnection_attempts: u64,
    /// Average message latency in milliseconds
    pub avg_latency_ms: Option<f64>,
    /// Connection uptime
    pub uptime: Duration,
    /// Last heartbeat timestamp
    pub last_heartbeat: Option<Instant>,
}

impl Default for ConnectionMetrics {
    fn default() -> Self {
        Self {
            messages_sent: 0,
            messages_received: 0,
            bytes_sent: 0,
            bytes_received: 0,
            active_connections: 0,
            reconnection_attempts: 0,
            avg_latency_ms: None,
            uptime: Duration::ZERO,
            last_heartbeat: None,
        }
    }
}

/// Metrics collector
pub struct MetricsCollector {
    metrics: Arc<ConnectionMetrics>,
    #[cfg(feature = "metrics")]
    counters: MetricsCounters,
    start_time: Instant,
}

#[cfg(feature = "metrics")]
struct MetricsCounters {
    messages_sent: Counter,
    messages_received: Counter,
    bytes_sent: Counter,
    bytes_received: Counter,
    active_connections: Gauge,
    reconnection_attempts: Counter,
    message_latency: Histogram,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(ConnectionMetrics::default()),
            #[cfg(feature = "metrics")]
            counters: MetricsCounters {
                messages_sent: Counter::new("websocket_messages_sent_total"),
                messages_received: Counter::new("websocket_messages_received_total"),
                bytes_sent: Counter::new("websocket_bytes_sent_total"),
                bytes_received: Counter::new("websocket_bytes_received_total"),
                active_connections: Gauge::new("websocket_active_connections"),
                reconnection_attempts: Counter::new("websocket_reconnection_attempts_total"),
                message_latency: Histogram::new("websocket_message_latency_ms"),
            },
            start_time: Instant::now(),
        }
    }
    
    pub fn record_message_sent(&self, bytes: usize) {
        let mut metrics = Arc::get_mut(&mut self.metrics.clone()).unwrap();
        metrics.messages_sent += 1;
        metrics.bytes_sent += bytes as u64;
        
        #[cfg(feature = "metrics")]
        {
            self.counters.messages_sent.increment(1);
            self.counters.bytes_sent.increment(bytes as u64);
        }
    }
    
    pub fn record_message_received(&self, bytes: usize) {
        let mut metrics = Arc::get_mut(&mut self.metrics.clone()).unwrap();
        metrics.messages_received += 1;
        metrics.bytes_received += bytes as u64;
        
        #[cfg(feature = "metrics")]
        {
            self.counters.messages_received.increment(1);
            self.counters.bytes_received.increment(bytes as u64);
        }
    }
    
    pub fn record_connection_established(&self) {
        let mut metrics = Arc::get_mut(&mut self.metrics.clone()).unwrap();
        metrics.active_connections += 1;
        
        #[cfg(feature = "metrics")]
        {
            self.counters.active_connections.set(metrics.active_connections as f64);
        }
    }
    
    pub fn record_connection_closed(&self) {
        let mut metrics = Arc::get_mut(&mut self.metrics.clone()).unwrap();
        if metrics.active_connections > 0 {
            metrics.active_connections -= 1;
        }
        
        #[cfg(feature = "metrics")]
        {
            self.counters.active_connections.set(metrics.active_connections as f64);
        }
    }
    
    pub fn record_reconnection_attempt(&self) {
        let mut metrics = Arc::get_mut(&mut self.metrics.clone()).unwrap();
        metrics.reconnection_attempts += 1;
        
        #[cfg(feature = "metrics")]
        {
            self.counters.reconnection_attempts.increment(1);
        }
    }
    
    pub fn record_latency(&self, latency: Duration) {
        let mut metrics = Arc::get_mut(&mut self.metrics.clone()).unwrap();
        let latency_ms = latency.as_millis() as f64;
        
        // Update average latency
        if let Some(avg) = metrics.avg_latency_ms {
            metrics.avg_latency_ms = Some((avg + latency_ms) / 2.0);
        } else {
            metrics.avg_latency_ms = Some(latency_ms);
        }
        
        #[cfg(feature = "metrics")]
        {
            self.counters.message_latency.record(latency_ms);
        }
    }
    
    pub fn record_heartbeat(&self) {
        let mut metrics = Arc::get_mut(&mut self.metrics.clone()).unwrap();
        metrics.last_heartbeat = Some(Instant::now());
    }
    
    pub fn get_metrics(&self) -> ConnectionMetrics {
        let mut metrics = (*self.metrics).clone();
        metrics.uptime = self.start_time.elapsed();
        metrics
    }
}

/// Performance profiler
pub struct PerformanceProfiler {
    measurements: HashMap<String, Vec<Duration>>,
    max_samples: usize,
}

impl PerformanceProfiler {
    pub fn new(max_samples: usize) -> Self {
        Self {
            measurements: HashMap::new(),
            max_samples,
        }
    }
    
    pub fn start_measurement(&self, name: &str) -> Measurement {
        Measurement {
            name: name.to_string(),
            start: Instant::now(),
        }
    }
    
    pub fn record_measurement(&mut self, measurement: Measurement) {
        let duration = measurement.start.elapsed();
        let samples = self.measurements.entry(measurement.name).or_insert_with(Vec::new);
        
        samples.push(duration);
        
        // Keep only the most recent samples
        if samples.len() > self.max_samples {
            samples.remove(0);
        }
    }
    
    pub fn get_statistics(&self, name: &str) -> Option<PerformanceStats> {
        let samples = self.measurements.get(name)?;
        if samples.is_empty() {
            return None;
        }
        
        let mut sorted = samples.clone();
        sorted.sort();
        
        let count = sorted.len();
        let min = sorted[0];
        let max = sorted[count - 1];
        let median = if count % 2 == 0 {
            (sorted[count / 2 - 1] + sorted[count / 2]) / 2
        } else {
            sorted[count / 2]
        };
        
        let sum: Duration = sorted.iter().sum();
        let avg = sum / count as u32;
        
        Some(PerformanceStats {
            count,
            min,
            max,
            median,
            avg,
            p95: sorted[(count as f64 * 0.95) as usize],
            p99: sorted[(count as f64 * 0.99) as usize],
        })
    }
}

pub struct Measurement {
    name: String,
    start: Instant,
}

#[derive(Debug, Clone)]
pub struct PerformanceStats {
    pub count: usize,
    pub min: Duration,
    pub max: Duration,
    pub median: Duration,
    pub avg: Duration,
    pub p95: Duration,
    pub p99: Duration,
}

/// Health check system
pub struct HealthChecker {
    checks: Vec<Box<dyn HealthCheck + Send + Sync>>,
}

pub trait HealthCheck: Send + Sync {
    fn name(&self) -> &str;
    fn check(&self) -> HealthStatus;
}

#[derive(Debug, Clone, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Unhealthy(String),
    Unknown,
}

impl HealthChecker {
    pub fn new() -> Self {
        Self {
            checks: Vec::new(),
        }
    }
    
    pub fn add_check<C>(&mut self, check: C)
    where
        C: HealthCheck + Send + Sync + 'static,
    {
        self.checks.push(Box::new(check));
    }
    
    pub fn run_checks(&self) -> HashMap<String, HealthStatus> {
        let mut results = HashMap::new();
        
        for check in &self.checks {
            let status = check.check();
            results.insert(check.name().to_string(), status);
        }
        
        results
    }
    
    pub fn is_healthy(&self) -> bool {
        for check in &self.checks {
            match check.check() {
                HealthStatus::Healthy => continue,
                HealthStatus::Unhealthy(_) | HealthStatus::Unknown => return false,
            }
        }
        true
    }
}

/// Connection health check
pub struct ConnectionHealthCheck {
    last_heartbeat: Option<Instant>,
    timeout: Duration,
}

impl ConnectionHealthCheck {
    pub fn new(timeout: Duration) -> Self {
        Self {
            last_heartbeat: None,
            timeout,
        }
    }
    
    pub fn record_heartbeat(&mut self) {
        self.last_heartbeat = Some(Instant::now());
    }
}

impl HealthCheck for ConnectionHealthCheck {
    fn name(&self) -> &str {
        "connection"
    }
    
    fn check(&self) -> HealthStatus {
        match self.last_heartbeat {
            Some(last) => {
                if last.elapsed() < self.timeout {
                    HealthStatus::Healthy
                } else {
                    HealthStatus::Unhealthy("Heartbeat timeout".to_string())
                }
            }
            None => HealthStatus::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_connection_metrics_default() {
        let metrics = ConnectionMetrics::default();
        assert_eq!(metrics.messages_sent, 0);
        assert_eq!(metrics.messages_received, 0);
        assert_eq!(metrics.active_connections, 0);
    }
    
    #[test]
    fn test_metrics_collector_creation() {
        let collector = MetricsCollector::new();
        let metrics = collector.get_metrics();
        assert_eq!(metrics.messages_sent, 0);
    }
    
    #[test]
    fn test_performance_profiler() {
        let mut profiler = PerformanceProfiler::new(100);
        let measurement = profiler.start_measurement("test");
        
        // Simulate some work
        std::thread::sleep(Duration::from_millis(10));
        
        profiler.record_measurement(measurement);
        
        let stats = profiler.get_statistics("test");
        assert!(stats.is_some());
        
        let stats = stats.unwrap();
        assert_eq!(stats.count, 1);
        assert!(stats.avg >= Duration::from_millis(10));
    }
    
    #[test]
    fn test_health_checker() {
        let mut checker = HealthChecker::new();
        let mut connection_check = ConnectionHealthCheck::new(Duration::from_secs(5));
        
        checker.add_check(connection_check);
        
        let results = checker.run_checks();
        assert_eq!(results.len(), 1);
        assert_eq!(results.get("connection"), Some(&HealthStatus::Unknown));
    }
    
    #[test]
    fn test_connection_health_check() {
        let mut check = ConnectionHealthCheck::new(Duration::from_secs(1));
        
        // Initially unknown
        assert_eq!(check.check(), HealthStatus::Unknown);
        
        // Record heartbeat
        check.record_heartbeat();
        assert_eq!(check.check(), HealthStatus::Healthy);
    }
}
