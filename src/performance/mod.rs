//! Performance Optimization Module
//!
//! High-performance features including connection pooling, message batching,
//! caching, and performance monitoring

pub mod cache;
pub mod connection_pool;
pub mod manager;
pub mod message_batcher;
pub mod metrics;
pub mod middleware;

// Re-export main types for backward compatibility
pub use cache::{CacheStats, MessageCache};
pub use connection_pool::{ConnectionPool, ConnectionPoolConfig, PooledConnection};
pub use manager::{PerformanceConfig, PerformanceManager};
pub use message_batcher::MessageBatcher;
pub use metrics::{MetricsCollector, PerformanceError, PerformanceMetrics, PerformanceProfiler};
pub use middleware::{BatchStats, PerformanceMiddleware, PoolStats};

// Re-export types and enums
pub use manager::{EvictionPolicy, MemoryPressureConfig, MessagePriority, SizeCategory};
pub use metrics::SpanStats;
