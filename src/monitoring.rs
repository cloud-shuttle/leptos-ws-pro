//! Comprehensive Monitoring and Observability System
//!
//! Production-grade monitoring, metrics collection, tracing, and alerting
//! for WebSocket applications with performance insights and health checks

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use serde::{Serialize, Deserialize};
use tokio::sync::mpsc;

/// Monitoring configuration
#[derive(Debug, Clone)]
pub struct MonitoringConfig {
    pub enable_metrics: bool,
    pub enable_tracing: bool,
    pub enable_health_checks: bool,
    pub enable_alerting: bool,
    pub metrics_retention: Duration,
    pub trace_sampling_rate: f64,
    pub health_check_interval: Duration,
    pub alert_thresholds: AlertThresholds,
    pub export_endpoint: Option<String>,
    pub export_interval: Duration,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            enable_metrics: true,
            enable_tracing: true,
            enable_health_checks: true,
            enable_alerting: true,
            metrics_retention: Duration::from_secs(3600), // 1 hour
            trace_sampling_rate: 0.1, // 10%
            health_check_interval: Duration::from_secs(30),
            alert_thresholds: AlertThresholds::default(),
            export_endpoint: None,
            export_interval: Duration::from_secs(60),
        }
    }
}

/// Alert thresholds configuration
#[derive(Debug, Clone)]
pub struct AlertThresholds {
    pub max_response_time_ms: u64,
    pub max_error_rate: f64,
    pub min_success_rate: f64,
    pub max_connection_failures: u32,
    pub max_memory_usage_mb: u64,
    pub max_cpu_usage_percent: f64,
}

impl Default for AlertThresholds {
    fn default() -> Self {
        Self {
            max_response_time_ms: 1000,
            max_error_rate: 0.05, // 5%
            min_success_rate: 0.95, // 95%
            max_connection_failures: 10,
            max_memory_usage_mb: 512,
            max_cpu_usage_percent: 80.0,
        }
    }
}

/// Central monitoring system coordinating all observability features
pub struct MonitoringSystem {
    config: MonitoringConfig,
    metrics_collector: Option<MetricsCollector>,
    tracer: Option<DistributedTracer>,
    health_monitor: Option<HealthMonitor>,
    alert_manager: Option<AlertManager>,
}

impl MonitoringSystem {
    pub fn new(config: MonitoringConfig) -> Self {
        let metrics_collector = if config.enable_metrics {
            Some(MetricsCollector::new(config.metrics_retention))
        } else {
            None
        };

        let tracer = if config.enable_tracing {
            Some(DistributedTracer::new(config.trace_sampling_rate))
        } else {
            None
        };

        let health_monitor = if config.enable_health_checks {
            Some(HealthMonitor::new(config.health_check_interval))
        } else {
            None
        };

        let alert_manager = if config.enable_alerting {
            Some(AlertManager::new(config.alert_thresholds.clone()))
        } else {
            None
        };

        Self {
            config,
            metrics_collector,
            tracer,
            health_monitor,
            alert_manager,
        }
    }

    /// Record a metric
    pub fn record_metric(&self, name: &str, value: f64, labels: HashMap<String, String>) {
        if let Some(collector) = &self.metrics_collector {
            collector.record(name, value, labels);
        }
    }

    /// Start a distributed trace span
    pub fn start_trace(&self, operation: &str) -> Option<TraceSpan> {
        self.tracer.as_ref().map(|tracer| tracer.start_span(operation))
    }

    /// Record health check result
    pub fn record_health_check(&self, component: &str, healthy: bool, details: Option<String>) {
        if let Some(monitor) = &self.health_monitor {
            monitor.record_check(component, healthy, details);
        }
    }

    /// Check if system should trigger alerts
    pub fn check_alerts(&self) -> Vec<Alert> {
        if let (Some(collector), Some(alert_manager)) = (&self.metrics_collector, &self.alert_manager) {
            let current_metrics = collector.get_current_metrics();
            alert_manager.evaluate_alerts(&current_metrics)
        } else {
            Vec::new()
        }
    }

    /// Get comprehensive system status
    pub fn get_system_status(&self) -> SystemStatus {
        let metrics = self.metrics_collector.as_ref()
            .map(|c| c.get_summary())
            .unwrap_or_default();

        let health = self.health_monitor.as_ref()
            .map(|h| h.get_overall_health())
            .unwrap_or(HealthStatus::Unknown);

        let active_alerts = self.check_alerts();

        SystemStatus {
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            health_status: health,
            metrics_summary: metrics,
            active_alerts: active_alerts.len(),
            uptime: self.get_uptime(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    fn get_uptime(&self) -> Duration {
        // In a real implementation, this would track actual start time
        Duration::from_secs(3600) // Placeholder
    }

    /// Export metrics to external system
    pub async fn export_metrics(&self) -> Result<String, MonitoringError> {
        if let Some(collector) = &self.metrics_collector {
            let metrics = collector.export_prometheus_format();

            if let Some(endpoint) = &self.config.export_endpoint {
                // TODO: Actually send metrics to endpoint
                tracing::info!("Would export metrics to {}: {} bytes", endpoint, metrics.len());
            }

            Ok(metrics)
        } else {
            Err(MonitoringError::MetricsDisabled)
        }
    }
}

/// Metrics collector with time-series data
pub struct MetricsCollector {
    metrics: Arc<RwLock<HashMap<String, TimeSeries>>>,
    retention_duration: Duration,
    start_time: Instant,
}

impl MetricsCollector {
    pub fn new(retention: Duration) -> Self {
        Self {
            metrics: Arc::new(RwLock::new(HashMap::new())),
            retention_duration: retention,
            start_time: Instant::now(),
        }
    }

    pub fn record(&self, name: &str, value: f64, labels: HashMap<String, String>) {
        let metric_key = self.build_metric_key(name, &labels);
        let mut metrics = self.metrics.write().unwrap();

        let series = metrics.entry(metric_key).or_insert_with(|| {
            TimeSeries::new(name.to_string(), labels)
        });

        series.add_point(value);
    }

    fn build_metric_key(&self, name: &str, labels: &HashMap<String, String>) -> String {
        let mut key = name.to_string();
        let mut label_pairs: Vec<_> = labels.iter().collect();
        label_pairs.sort_by_key(|(k, _)| *k);

        for (k, v) in label_pairs {
            key.push_str(&format!(",{}={}", k, v));
        }

        key
    }

    pub fn get_current_metrics(&self) -> HashMap<String, f64> {
        let metrics = self.metrics.read().unwrap();
        let mut current = HashMap::new();

        for (key, series) in metrics.iter() {
            if let Some(latest) = series.latest_value() {
                current.insert(key.clone(), latest);
            }
        }

        current
    }

    pub fn get_summary(&self) -> MetricsSummary {
        let metrics = self.metrics.read().unwrap();

        MetricsSummary {
            total_metrics: metrics.len(),
            oldest_timestamp: self.get_oldest_timestamp(),
            newest_timestamp: Instant::now(),
            memory_usage: self.estimate_memory_usage(&metrics),
        }
    }

    fn get_oldest_timestamp(&self) -> Instant {
        self.start_time
    }

    fn estimate_memory_usage(&self, metrics: &HashMap<String, TimeSeries>) -> usize {
        metrics.iter()
            .map(|(key, series)| key.len() + series.estimated_size())
            .sum()
    }

    pub fn export_prometheus_format(&self) -> String {
        let metrics = self.metrics.read().unwrap();
        let mut output = String::new();

        for (key, series) in metrics.iter() {
            output.push_str(&format!("# TYPE {} gauge\n", series.name));

            let label_str = if series.labels.is_empty() {
                String::new()
            } else {
                let labels: Vec<String> = series.labels.iter()
                    .map(|(k, v)| format!("{}=\"{}\"", k, v))
                    .collect();
                format!("{{{}}}", labels.join(","))
            };

            if let Some(value) = series.latest_value() {
                output.push_str(&format!("{}{} {}\n", series.name, label_str, value));
            }
        }

        output
    }

    pub fn cleanup_old_metrics(&self) {
        let mut metrics = self.metrics.write().unwrap();
        let cutoff = Instant::now() - self.retention_duration;

        for series in metrics.values_mut() {
            series.remove_points_before(cutoff);
        }

        metrics.retain(|_, series| !series.is_empty());
    }
}

/// Time series data for a metric
#[derive(Debug)]
pub struct TimeSeries {
    pub name: String,
    pub labels: HashMap<String, String>,
    points: VecDeque<DataPoint>,
    max_points: usize,
}

impl TimeSeries {
    pub fn new(name: String, labels: HashMap<String, String>) -> Self {
        Self {
            name,
            labels,
            points: VecDeque::new(),
            max_points: 1000, // Limit memory usage
        }
    }

    pub fn add_point(&mut self, value: f64) {
        if self.points.len() >= self.max_points {
            self.points.pop_front();
        }

        self.points.push_back(DataPoint {
            timestamp: Instant::now(),
            value,
        });
    }

    pub fn latest_value(&self) -> Option<f64> {
        self.points.back().map(|p| p.value)
    }

    pub fn remove_points_before(&mut self, cutoff: Instant) {
        while let Some(point) = self.points.front() {
            if point.timestamp < cutoff {
                self.points.pop_front();
            } else {
                break;
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    pub fn estimated_size(&self) -> usize {
        self.points.len() * std::mem::size_of::<DataPoint>()
    }
}

#[derive(Debug, Clone)]
struct DataPoint {
    timestamp: Instant,
    value: f64,
}

/// Distributed tracing system
pub struct DistributedTracer {
    sampling_rate: f64,
    active_spans: Arc<Mutex<HashMap<String, TraceSpan>>>,
}

impl DistributedTracer {
    pub fn new(sampling_rate: f64) -> Self {
        Self {
            sampling_rate,
            active_spans: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn start_span(&self, operation: &str) -> TraceSpan {
        let should_sample = rand::random::<f64>() < self.sampling_rate;

        let span = TraceSpan {
            trace_id: self.generate_trace_id(),
            span_id: self.generate_span_id(),
            operation_name: operation.to_string(),
            start_time: Instant::now(),
            end_time: None,
            tags: HashMap::new(),
            sampled: should_sample,
        };

        if should_sample {
            let mut spans = self.active_spans.lock().unwrap();
            spans.insert(span.span_id.clone(), span.clone());
        }

        span
    }

    fn generate_trace_id(&self) -> String {
        format!("{:016x}", rand::random::<u64>())
    }

    fn generate_span_id(&self) -> String {
        format!("{:08x}", rand::random::<u32>())
    }

    pub fn finish_span(&self, span_id: &str) {
        let mut spans = self.active_spans.lock().unwrap();
        if let Some(mut span) = spans.remove(span_id) {
            span.end_time = Some(Instant::now());
            // TODO: Export to tracing backend
            tracing::debug!("Finished span: {} ({}ms)",
                span.operation_name,
                span.duration().map(|d| d.as_millis()).unwrap_or(0)
            );
        }
    }
}

/// Individual trace span
#[derive(Debug, Clone)]
pub struct TraceSpan {
    pub trace_id: String,
    pub span_id: String,
    pub operation_name: String,
    pub start_time: Instant,
    pub end_time: Option<Instant>,
    pub tags: HashMap<String, String>,
    pub sampled: bool,
}

impl TraceSpan {
    pub fn add_tag(&mut self, key: &str, value: &str) {
        self.tags.insert(key.to_string(), value.to_string());
    }

    pub fn finish(mut self) -> Self {
        self.end_time = Some(Instant::now());
        self
    }

    pub fn duration(&self) -> Option<Duration> {
        self.end_time.map(|end| end.duration_since(self.start_time))
    }
}

/// Health monitoring system
pub struct HealthMonitor {
    checks: Arc<RwLock<HashMap<String, HealthCheck>>>,
    check_interval: Duration,
}

impl HealthMonitor {
    pub fn new(interval: Duration) -> Self {
        Self {
            checks: Arc::new(RwLock::new(HashMap::new())),
            check_interval: interval,
        }
    }

    pub fn record_check(&self, component: &str, healthy: bool, details: Option<String>) {
        let mut checks = self.checks.write().unwrap();
        let check = checks.entry(component.to_string()).or_insert_with(|| {
            HealthCheck::new(component.to_string())
        });

        check.record_result(healthy, details);
    }

    pub fn get_overall_health(&self) -> HealthStatus {
        let checks = self.checks.read().unwrap();

        if checks.is_empty() {
            return HealthStatus::Unknown;
        }

        let unhealthy_count = checks.values()
            .filter(|check| !check.is_healthy())
            .count();

        match unhealthy_count {
            0 => HealthStatus::Healthy,
            n if n < checks.len() / 2 => HealthStatus::Degraded,
            _ => HealthStatus::Unhealthy,
        }
    }

    pub fn get_component_health(&self, component: &str) -> Option<HealthStatus> {
        let checks = self.checks.read().unwrap();
        checks.get(component).map(|check| {
            if check.is_healthy() {
                HealthStatus::Healthy
            } else {
                HealthStatus::Unhealthy
            }
        })
    }
}

/// Individual health check
#[derive(Debug, Clone)]
pub struct HealthCheck {
    pub component: String,
    pub last_check: Option<Instant>,
    pub is_healthy: bool,
    pub consecutive_failures: u32,
    pub details: Option<String>,
    pub history: VecDeque<HealthCheckResult>,
}

impl HealthCheck {
    pub fn new(component: String) -> Self {
        Self {
            component,
            last_check: None,
            is_healthy: true,
            consecutive_failures: 0,
            details: None,
            history: VecDeque::with_capacity(100),
        }
    }

    pub fn record_result(&mut self, healthy: bool, details: Option<String>) {
        self.last_check = Some(Instant::now());
        self.is_healthy = healthy;
        self.details = details.clone();

        if healthy {
            self.consecutive_failures = 0;
        } else {
            self.consecutive_failures += 1;
        }

        if self.history.len() >= 100 {
            self.history.pop_front();
        }

        self.history.push_back(HealthCheckResult {
            timestamp: Instant::now(),
            healthy,
            details,
        });
    }

    pub fn is_healthy(&self) -> bool {
        self.is_healthy && self.consecutive_failures < 3
    }
}

#[derive(Debug, Clone)]
pub struct HealthCheckResult {
    pub timestamp: Instant,
    pub healthy: bool,
    pub details: Option<String>,
}

/// Alert management system
pub struct AlertManager {
    thresholds: AlertThresholds,
    active_alerts: Arc<Mutex<HashMap<String, Alert>>>,
}

impl AlertManager {
    pub fn new(thresholds: AlertThresholds) -> Self {
        Self {
            thresholds,
            active_alerts: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn evaluate_alerts(&self, metrics: &HashMap<String, f64>) -> Vec<Alert> {
        let mut alerts = Vec::new();
        let mut active_alerts = self.active_alerts.lock().unwrap();

        // Response time alert
        if let Some(&response_time) = metrics.get("response_time_ms") {
            if response_time > self.thresholds.max_response_time_ms as f64 {
                let alert = Alert::new(
                    AlertLevel::Warning,
                    "High Response Time".to_string(),
                    format!("Response time {}ms exceeds threshold {}ms",
                        response_time, self.thresholds.max_response_time_ms),
                );
                active_alerts.insert("response_time".to_string(), alert.clone());
                alerts.push(alert);
            } else {
                active_alerts.remove("response_time");
            }
        }

        // Error rate alert
        if let Some(&error_rate) = metrics.get("error_rate") {
            if error_rate > self.thresholds.max_error_rate {
                let alert = Alert::new(
                    AlertLevel::Critical,
                    "High Error Rate".to_string(),
                    format!("Error rate {:.2}% exceeds threshold {:.2}%",
                        error_rate * 100.0, self.thresholds.max_error_rate * 100.0),
                );
                active_alerts.insert("error_rate".to_string(), alert.clone());
                alerts.push(alert);
            } else {
                active_alerts.remove("error_rate");
            }
        }

        alerts
    }

    pub fn get_active_alerts(&self) -> Vec<Alert> {
        let alerts = self.active_alerts.lock().unwrap();
        alerts.values().cloned().collect()
    }
}

/// Alert representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: String,
    pub level: AlertLevel,
    pub title: String,
    pub description: String,
    pub timestamp: u64,
    pub resolved: bool,
}

impl Alert {
    pub fn new(level: AlertLevel, title: String, description: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            level,
            title,
            description,
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            resolved: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum AlertLevel {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStatus {
    pub timestamp: u64,
    pub health_status: HealthStatus,
    pub metrics_summary: MetricsSummary,
    pub active_alerts: usize,
    pub uptime: Duration,
    pub version: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MetricsSummary {
    pub total_metrics: usize,
    pub oldest_timestamp: Instant,
    pub newest_timestamp: Instant,
    pub memory_usage: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum MonitoringError {
    #[error("Metrics collection is disabled")]
    MetricsDisabled,

    #[error("Tracing is disabled")]
    TracingDisabled,

    #[error("Health monitoring is disabled")]
    HealthMonitoringDisabled,

    #[error("Export failed: {0}")]
    ExportFailed(String),

    #[error("Alert configuration error: {0}")]
    AlertConfigError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_collector() {
        let collector = MetricsCollector::new(Duration::from_secs(60));
        let labels = HashMap::new();

        collector.record("test_metric", 42.0, labels);

        let current = collector.get_current_metrics();
        assert!(current.contains_key("test_metric"));
        assert_eq!(current.get("test_metric"), Some(&42.0));
    }

    #[test]
    fn test_health_monitor() {
        let monitor = HealthMonitor::new(Duration::from_secs(30));

        monitor.record_check("database", true, None);
        monitor.record_check("redis", false, Some("Connection timeout".to_string()));

        assert_eq!(monitor.get_component_health("database"), Some(HealthStatus::Healthy));
        assert_eq!(monitor.get_component_health("redis"), Some(HealthStatus::Unhealthy));
        assert_eq!(monitor.get_overall_health(), HealthStatus::Degraded);
    }

    #[test]
    fn test_alert_manager() {
        let thresholds = AlertThresholds::default();
        let manager = AlertManager::new(thresholds);

        let mut metrics = HashMap::new();
        metrics.insert("response_time_ms".to_string(), 2000.0); // Above threshold

        let alerts = manager.evaluate_alerts(&metrics);
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].level, AlertLevel::Warning);
    }

    #[test]
    fn test_trace_span() {
        let tracer = DistributedTracer::new(1.0); // Always sample
        let mut span = tracer.start_span("test_operation");

        span.add_tag("user_id", "123");
        span.add_tag("endpoint", "/api/test");

        let finished_span = span.finish();
        assert!(finished_span.duration().is_some());
        assert_eq!(finished_span.tags.get("user_id"), Some(&"123".to_string()));
    }

    #[test]
    fn test_prometheus_export() {
        let collector = MetricsCollector::new(Duration::from_secs(60));
        let mut labels = HashMap::new();
        labels.insert("service".to_string(), "websocket".to_string());

        collector.record("connections_active", 15.0, labels);

        let prometheus = collector.export_prometheus_format();
        assert!(prometheus.contains("connections_active"));
        assert!(prometheus.contains("service=\"websocket\""));
        assert!(prometheus.contains("15"));
    }
}
