//! Performance Metrics
//!
//! Collects and tracks performance metrics, profiling, and monitoring

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Performance metrics collector
pub struct MetricsCollector {
    metrics: Arc<RwLock<HashMap<String, MetricValue>>>,
    start_time: Instant,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(HashMap::new())),
            start_time: Instant::now(),
        }
    }

    pub fn record_metric(&self, name: &str, value: f64, tags: Option<HashMap<String, String>>) {
        let metric = MetricValue {
            value,
            timestamp: Instant::now(),
            tags: tags.unwrap_or_default(),
        };

        tokio::spawn({
            let metrics = self.metrics.clone();
            let name = name.to_string();
            async move {
                let mut metrics = metrics.write().await;
                metrics.insert(name, metric);
            }
        });
    }

    pub fn get_metrics(&self) -> PerformanceMetrics {
        // In async context, we'd need to handle this differently
        // For now, return basic metrics
        PerformanceMetrics {
            uptime: self.start_time.elapsed(),
            total_requests: 0,
            requests_per_second: 0.0,
            average_response_time: Duration::from_millis(0),
            memory_usage: 0,
            cpu_usage: 0.0,
            active_connections: 0,
            message_throughput: 0.0,
            total_operations: 0,
            average_operation_time: Duration::from_millis(0),
            network_throughput: 0.0,
        }
    }
}

#[derive(Debug, Clone)]
struct MetricValue {
    value: f64,
    timestamp: Instant,
    tags: HashMap<String, String>,
}

/// Performance metrics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub uptime: Duration,
    pub total_requests: u64,
    pub requests_per_second: f64,
    pub average_response_time: Duration,
    pub memory_usage: u64,
    pub cpu_usage: f64,
    pub active_connections: u32,
    pub message_throughput: f64,
    pub total_operations: u64,
    pub average_operation_time: Duration,
    pub network_throughput: f64,
}

/// Performance-related errors
#[derive(Debug, thiserror::Error)]
pub enum PerformanceError {
    #[error("Connection pooling is disabled")]
    PoolingDisabled,

    #[error("Connection pool exhausted")]
    PoolExhausted,

    #[error("Message batching is disabled")]
    BatchingDisabled,

    #[error("Cache operation failed: {0}")]
    CacheError(String),

    #[error("Metrics collection failed: {0}")]
    MetricsError(String),
}

/// Performance profiler for hot path optimization
pub struct PerformanceProfiler {
    samples: HashMap<String, Vec<Duration>>,
    active_spans: HashMap<String, Instant>,
}

impl PerformanceProfiler {
    pub fn new() -> Self {
        Self {
            samples: HashMap::new(),
            active_spans: HashMap::new(),
        }
    }

    pub fn start_span(&mut self, name: &str) {
        self.active_spans.insert(name.to_string(), Instant::now());
    }

    pub fn end_span(&mut self, name: &str) {
        if let Some(start_time) = self.active_spans.remove(name) {
            let duration = start_time.elapsed();
            self.samples
                .entry(name.to_string())
                .or_insert_with(Vec::new)
                .push(duration);
        }
    }

    pub fn get_stats(&self, name: &str) -> Option<SpanStats> {
        self.samples.get(name).map(|samples| {
            let sum: Duration = samples.iter().sum();
            let avg = sum / samples.len() as u32;
            let min = *samples.iter().min().unwrap();
            let max = *samples.iter().max().unwrap();

            SpanStats {
                count: samples.len(),
                average: avg,
                min,
                max,
                total: sum,
            }
        })
    }
}

#[derive(Debug, Clone)]
pub struct SpanStats {
    pub count: usize,
    pub average: Duration,
    pub min: Duration,
    pub max: Duration,
    pub total: Duration,
}

impl Default for PerformanceProfiler {
    fn default() -> Self {
        Self::new()
    }
}
