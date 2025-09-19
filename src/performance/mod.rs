//! Performance Optimization Module
//!
//! High-performance features including connection pooling, message batching,
//! caching, and performance monitoring

pub mod manager;
pub mod connection_pool;
pub mod message_batcher;
pub mod cache;
pub mod metrics;
pub mod middleware;

// Re-export main types for backward compatibility
pub use manager::{PerformanceManager, PerformanceConfig};
pub use connection_pool::{ConnectionPool, ConnectionPoolConfig, PooledConnection};
pub use message_batcher::MessageBatcher;
pub use cache::{MessageCache, CacheStats};
pub use metrics::{MetricsCollector, PerformanceMetrics, PerformanceError, PerformanceProfiler};
pub use middleware::{PerformanceMiddleware, PoolStats, BatchStats};

// Re-export types and enums
pub use manager::{MemoryPressureConfig, EvictionPolicy, MessagePriority, SizeCategory};
pub use metrics::SpanStats;
