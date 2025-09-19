//! Monitoring System
//!
//! Comprehensive monitoring, metrics collection, health checks, and alerting

pub mod system;
pub mod metrics;
pub mod health;
pub mod alerts;

// Re-export main types
pub use system::{MonitoringSystem, MonitoringConfig, AlertThresholds};
pub use metrics::{MetricsCollector, TimeSeries, DistributedTracer, TraceSpan};
pub use health::{HealthMonitor, HealthCheck, HealthCheckResult, HealthStatus};
pub use alerts::{AlertManager, Alert, AlertLevel, SystemStatus, MetricsSummary, MonitoringError};
