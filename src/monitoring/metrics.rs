//! Metrics Collection and Distributed Tracing
//!
//! Time-series metrics collection and distributed tracing for observability

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
use serde::{Serialize, Deserialize};

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

    pub fn get_summary(&self) -> crate::monitoring::alerts::MetricsSummary {
        let metrics = self.metrics.read().unwrap();

        crate::monitoring::alerts::MetricsSummary {
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
        format!("{:016x}", rand::random::<u64>())
    }
}

/// A distributed trace span
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
    pub fn finish(&mut self) {
        self.end_time = Some(Instant::now());
    }

    pub fn add_tag(&mut self, key: String, value: String) {
        self.tags.insert(key, value);
    }

    pub fn duration(&self) -> Option<Duration> {
        self.end_time.map(|end| end.duration_since(self.start_time))
    }
}
