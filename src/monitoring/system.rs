//! Monitoring System
//!
//! Central monitoring system coordinating all observability features

use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use crate::monitoring::metrics::{MetricsCollector, DistributedTracer, TraceSpan};
use crate::monitoring::health::{HealthMonitor, HealthStatus};
use crate::monitoring::alerts::{AlertManager, Alert, SystemStatus, MetricsSummary, MonitoringError};

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
