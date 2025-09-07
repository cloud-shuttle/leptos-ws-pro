//! Performance Optimization Module
//!
//! High-performance features including connection pooling, message batching,
//! caching, and performance monitoring

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use serde::{Serialize, Deserialize};
use tokio::sync::RwLock;

/// Performance configuration
#[derive(Debug, Clone)]
pub struct PerformanceConfig {
    pub enable_connection_pooling: bool,
    pub max_pool_size: usize,
    pub enable_message_batching: bool,
    pub batch_size: usize,
    pub batch_timeout: Duration,
    pub enable_caching: bool,
    pub cache_size: usize,
    pub cache_ttl: Duration,
    pub enable_compression: bool,
    pub compression_threshold: usize,
    pub enable_metrics: bool,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            enable_connection_pooling: true,
            max_pool_size: 10,
            enable_message_batching: true,
            batch_size: 100,
            batch_timeout: Duration::from_millis(10),
            enable_caching: true,
            cache_size: 1000,
            cache_ttl: Duration::from_secs(300),
            enable_compression: true,
            compression_threshold: 1024,
            enable_metrics: true,
        }
    }
}

/// Performance manager coordinating all optimizations
pub struct PerformanceManager {
    config: PerformanceConfig,
    connection_pool: Option<ConnectionPool>,
    message_batcher: Option<MessageBatcher>,
    cache: Option<MessageCache>,
    metrics_collector: Option<MetricsCollector>,
}

impl PerformanceManager {
    pub fn new(config: PerformanceConfig) -> Self {
        let connection_pool = if config.enable_connection_pooling {
            Some(ConnectionPool::new(config.max_pool_size))
        } else {
            None
        };

        let message_batcher = if config.enable_message_batching {
            Some(MessageBatcher::new(config.batch_size, config.batch_timeout))
        } else {
            None
        };

        let cache = if config.enable_caching {
            Some(MessageCache::new(config.cache_size, config.cache_ttl))
        } else {
            None
        };

        let metrics_collector = if config.enable_metrics {
            Some(MetricsCollector::new())
        } else {
            None
        };

        Self {
            config,
            connection_pool,
            message_batcher,
            cache,
            metrics_collector,
        }
    }

    /// Get or create a connection from the pool
    pub async fn get_connection(&self, url: &str) -> Result<PooledConnection, PerformanceError> {
        if let Some(pool) = &self.connection_pool {
            pool.get_connection(url).await
        } else {
            Err(PerformanceError::PoolingDisabled)
        }
    }

    /// Return connection to pool
    pub async fn return_connection(&self, connection: PooledConnection) {
        if let Some(pool) = &self.connection_pool {
            pool.return_connection(connection).await;
        }
    }

    /// Add message to batch queue
    pub async fn queue_message(&self, message: Vec<u8>) -> Result<(), PerformanceError> {
        if let Some(batcher) = &self.message_batcher {
            batcher.add_message(message).await
        } else {
            Err(PerformanceError::BatchingDisabled)
        }
    }

    /// Flush pending batched messages
    pub async fn flush_messages(&self) -> Result<Vec<Vec<u8>>, PerformanceError> {
        if let Some(batcher) = &self.message_batcher {
            Ok(batcher.flush_messages().await)
        } else {
            Ok(vec![])
        }
    }

    /// Get cached message
    pub async fn get_cached(&self, key: &str) -> Option<Vec<u8>> {
        if let Some(cache) = &self.cache {
            cache.get(key).await
        } else {
            None
        }
    }

    /// Set cached message
    pub async fn set_cached(&self, key: String, value: Vec<u8>) {
        if let Some(cache) = &self.cache {
            cache.set(key, value).await;
        }
    }

    /// Record performance metric
    pub fn record_metric(&self, name: &str, value: f64, tags: Option<HashMap<String, String>>) {
        if let Some(collector) = &self.metrics_collector {
            collector.record_metric(name, value, tags);
        }
    }

    /// Get current performance metrics
    pub fn get_metrics(&self) -> Option<PerformanceMetrics> {
        self.metrics_collector.as_ref().map(|c| c.get_metrics())
    }

    /// Check if compression should be used for message
    pub fn should_compress(&self, message_size: usize) -> bool {
        self.config.enable_compression && message_size >= self.config.compression_threshold
    }
}

/// Connection pool for reusing WebSocket connections
pub struct ConnectionPool {
    max_size: usize,
    connections: Arc<RwLock<HashMap<String, VecDeque<PooledConnection>>>>,
    total_connections: Arc<Mutex<usize>>,
}

impl ConnectionPool {
    pub fn new(max_size: usize) -> Self {
        Self {
            max_size,
            connections: Arc::new(RwLock::new(HashMap::new())),
            total_connections: Arc::new(Mutex::new(0)),
        }
    }

    pub async fn get_connection(&self, url: &str) -> Result<PooledConnection, PerformanceError> {
        let mut connections = self.connections.write().await;

        if let Some(pool) = connections.get_mut(url) {
            if let Some(connection) = pool.pop_front() {
                return Ok(connection);
            }
        }

        // No available connection, create new one if under limit
        let total = *self.total_connections.lock().unwrap();
        if total < self.max_size {
            *self.total_connections.lock().unwrap() += 1;
            Ok(PooledConnection::new(url.to_string()))
        } else {
            Err(PerformanceError::PoolExhausted)
        }
    }

    pub async fn return_connection(&self, connection: PooledConnection) {
        if connection.is_healthy() {
            let mut connections = self.connections.write().await;
            let pool = connections.entry(connection.url.clone()).or_insert_with(VecDeque::new);
            pool.push_back(connection);
        } else {
            // Unhealthy connection, don't return to pool
            *self.total_connections.lock().unwrap() -= 1;
        }
    }

    pub async fn cleanup_idle_connections(&self) {
        let mut connections = self.connections.write().await;
        let cutoff = Instant::now() - Duration::from_secs(300); // 5 minutes

        for pool in connections.values_mut() {
            let original_len = pool.len();
            pool.retain(|conn| conn.last_used > cutoff);
            let removed = original_len - pool.len();

            if removed > 0 {
                *self.total_connections.lock().unwrap() -= removed;
            }
        }
    }
}

/// Pooled connection wrapper
#[derive(Debug, Clone)]
pub struct PooledConnection {
    pub url: String,
    pub created_at: Instant,
    pub last_used: Instant,
    pub request_count: u64,
    pub is_connected: bool,
}

impl PooledConnection {
    pub fn new(url: String) -> Self {
        let now = Instant::now();
        Self {
            url,
            created_at: now,
            last_used: now,
            request_count: 0,
            is_connected: true,
        }
    }

    pub fn is_healthy(&self) -> bool {
        self.is_connected && self.last_used.elapsed() < Duration::from_secs(60)
    }

    pub fn mark_used(&mut self) {
        self.last_used = Instant::now();
        self.request_count += 1;
    }
}

/// Message batcher for improving throughput
pub struct MessageBatcher {
    batch_size: usize,
    batch_timeout: Duration,
    pending_messages: Arc<Mutex<VecDeque<Vec<u8>>>>,
    last_flush: Arc<Mutex<Instant>>,
}

impl MessageBatcher {
    pub fn new(batch_size: usize, batch_timeout: Duration) -> Self {
        Self {
            batch_size,
            batch_timeout,
            pending_messages: Arc::new(Mutex::new(VecDeque::new())),
            last_flush: Arc::new(Mutex::new(Instant::now())),
        }
    }

    pub async fn add_message(&self, message: Vec<u8>) -> Result<(), PerformanceError> {
        let mut pending = self.pending_messages.lock().unwrap();
        pending.push_back(message);

        // Auto-flush if batch is full
        if pending.len() >= self.batch_size {
            drop(pending);
            self.flush_messages().await;
        }

        Ok(())
    }

    pub async fn flush_messages(&self) -> Vec<Vec<u8>> {
        let mut pending = self.pending_messages.lock().unwrap();
        let messages: Vec<_> = pending.drain(..).collect();
        *self.last_flush.lock().unwrap() = Instant::now();
        messages
    }

    pub fn should_flush(&self) -> bool {
        let pending = self.pending_messages.lock().unwrap();
        let last_flush = self.last_flush.lock().unwrap();

        !pending.is_empty() &&
        (pending.len() >= self.batch_size ||
         last_flush.elapsed() >= self.batch_timeout)
    }

    pub fn pending_count(&self) -> usize {
        self.pending_messages.lock().unwrap().len()
    }
}

/// High-performance message cache
pub struct MessageCache {
    cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
    max_size: usize,
    ttl: Duration,
}

impl MessageCache {
    pub fn new(max_size: usize, ttl: Duration) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            max_size,
            ttl,
        }
    }

    pub async fn get(&self, key: &str) -> Option<Vec<u8>> {
        let cache = self.cache.read().await;

        if let Some(entry) = cache.get(key) {
            if entry.expires_at > Instant::now() {
                Some(entry.value.clone())
            } else {
                None // Expired
            }
        } else {
            None
        }
    }

    pub async fn set(&self, key: String, value: Vec<u8>) {
        let mut cache = self.cache.write().await;

        // Evict oldest entries if at capacity
        if cache.len() >= self.max_size {
            self.evict_oldest(&mut cache);
        }

        cache.insert(key, CacheEntry {
            value,
            created_at: Instant::now(),
            expires_at: Instant::now() + self.ttl,
            access_count: 1,
        });
    }

    fn evict_oldest(&self, cache: &mut HashMap<String, CacheEntry>) {
        if let Some(oldest_key) = cache.iter()
            .min_by_key(|(_, entry)| entry.created_at)
            .map(|(key, _)| key.clone())
        {
            cache.remove(&oldest_key);
        }
    }

    pub async fn cleanup_expired(&self) {
        let mut cache = self.cache.write().await;
        let now = Instant::now();

        cache.retain(|_, entry| entry.expires_at > now);
    }

    pub async fn stats(&self) -> CacheStats {
        let cache = self.cache.read().await;

        CacheStats {
            size: cache.len(),
            capacity: self.max_size,
            hit_ratio: 0.0, // Would need hit/miss tracking
        }
    }
}

#[derive(Debug, Clone)]
struct CacheEntry {
    value: Vec<u8>,
    created_at: Instant,
    expires_at: Instant,
    access_count: u64,
}

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub size: usize,
    pub capacity: usize,
    pub hit_ratio: f64,
}

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
            self.samples.entry(name.to_string()).or_insert_with(Vec::new).push(duration);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_connection_pool() {
        let pool = ConnectionPool::new(2);

        let conn1 = pool.get_connection("ws://localhost:8080").await.unwrap();
        let conn2 = pool.get_connection("ws://localhost:8080").await.unwrap();

        // Pool should be exhausted
        assert!(pool.get_connection("ws://localhost:8080").await.is_err());

        // Return connection
        pool.return_connection(conn1).await;

        // Should be able to get connection again
        assert!(pool.get_connection("ws://localhost:8080").await.is_ok());
    }

    #[tokio::test]
    async fn test_message_batcher() {
        let batcher = MessageBatcher::new(3, Duration::from_millis(100));

        batcher.add_message(b"message1".to_vec()).await.unwrap();
        batcher.add_message(b"message2".to_vec()).await.unwrap();

        assert_eq!(batcher.pending_count(), 2);

        batcher.add_message(b"message3".to_vec()).await.unwrap(); // Should auto-flush

        assert_eq!(batcher.pending_count(), 0);
    }

    #[tokio::test]
    async fn test_message_cache() {
        let cache = MessageCache::new(2, Duration::from_secs(1));

        cache.set("key1".to_string(), b"value1".to_vec()).await;
        cache.set("key2".to_string(), b"value2".to_vec()).await;

        assert_eq!(cache.get("key1").await, Some(b"value1".to_vec()));
        assert_eq!(cache.get("key2").await, Some(b"value2".to_vec()));

        // Should evict oldest when at capacity
        cache.set("key3".to_string(), b"value3".to_vec()).await;

        let stats = cache.stats().await;
        assert_eq!(stats.size, 2);
    }

    #[test]
    fn test_profiler() {
        let mut profiler = PerformanceProfiler::new();

        profiler.start_span("test_operation");
        std::thread::sleep(Duration::from_millis(10));
        profiler.end_span("test_operation");

        let stats = profiler.get_stats("test_operation").unwrap();
        assert_eq!(stats.count, 1);
        assert!(stats.average >= Duration::from_millis(10));
    }
}
