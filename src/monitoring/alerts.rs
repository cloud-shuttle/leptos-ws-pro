//! Alert Management
//!
//! Alert system for monitoring thresholds and system health

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use serde::{Serialize, Deserialize};

/// Alert management system
pub struct AlertManager {
    thresholds: crate::monitoring::system::AlertThresholds,
    active_alerts: Arc<Mutex<HashMap<String, Alert>>>,
}

impl AlertManager {
    pub fn new(thresholds: crate::monitoring::system::AlertThresholds) -> Self {
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
}

/// Alert structure
#[derive(Debug, Clone)]
pub struct Alert {
    pub level: AlertLevel,
    pub title: String,
    pub message: String,
    pub timestamp: Instant,
    pub acknowledged: bool,
}

impl Alert {
    pub fn new(level: AlertLevel, title: String, message: String) -> Self {
        Self {
            level,
            title,
            message,
            timestamp: Instant::now(),
            acknowledged: false,
        }
    }

    pub fn acknowledge(&mut self) {
        self.acknowledged = true;
    }
}

/// Alert severity levels
#[derive(Debug, Clone, PartialEq)]
pub enum AlertLevel {
    Info,
    Warning,
    Critical,
}

/// System status summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStatus {
    pub timestamp: u64,
    pub health_status: crate::monitoring::health::HealthStatus,
    pub metrics_summary: MetricsSummary,
    pub active_alerts: usize,
    pub uptime: Duration,
    pub version: String,
}

/// Metrics summary
#[derive(Debug, Clone, Default)]
pub struct MetricsSummary {
    pub total_metrics: usize,
    pub oldest_timestamp: Instant,
    pub newest_timestamp: Instant,
    pub memory_usage: usize,
}

/// Monitoring errors
#[derive(Debug, thiserror::Error)]
pub enum MonitoringError {
    #[error("Metrics collection is disabled")]
    MetricsDisabled,

    #[error("Tracing is disabled")]
    TracingDisabled,

    #[error("Health monitoring is disabled")]
    HealthMonitoringDisabled,

    #[error("Alerting is disabled")]
    AlertingDisabled,

    #[error("Export failed: {0}")]
    ExportFailed(String),
}
