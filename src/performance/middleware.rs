//! Performance Middleware
//!
//! Middleware for integrating performance optimizations with transport layer

use crate::performance::{
    ConnectionPool, MessageBatcher, MessageCache, PerformanceManager,
    connection_pool::PooledConnection,
};
use crate::transport::{Message, TransportError};
use std::sync::Arc;
use tokio::sync::Mutex;
use std::time::Duration;

/// Performance middleware for transport layer
pub struct PerformanceMiddleware {
    connection_pool: Arc<ConnectionPool>,
    message_batcher: Arc<MessageBatcher>,
    message_cache: Arc<MessageCache>,
    performance_manager: Arc<Mutex<PerformanceManager>>,
}

impl PerformanceMiddleware {
    pub fn new(
        connection_pool: ConnectionPool,
        message_batcher: MessageBatcher,
        message_cache: MessageCache,
        performance_manager: PerformanceManager,
    ) -> Self {
        Self {
            connection_pool: Arc::new(connection_pool),
            message_batcher: Arc::new(message_batcher),
            message_cache: Arc::new(message_cache),
            performance_manager: Arc::new(Mutex::new(performance_manager)),
        }
    }

    /// Get a pooled connection for improved performance
    pub async fn get_pooled_connection(&self, url: &str) -> Result<PooledConnection, TransportError> {
        self.connection_pool.get_connection(url).await
            .map_err(|e| TransportError::ConnectionFailed(format!("Failed to get pooled connection: {:?}", e)))
    }

    /// Return a connection to the pool
    pub async fn return_connection(&self, connection: PooledConnection) {
        self.connection_pool.return_connection(connection).await;
    }

    /// Add message to batch for improved throughput
    pub async fn batch_message(&self, message: Message) -> Result<(), TransportError> {
        self.message_batcher.add_message(message.data).await
            .map_err(|e| TransportError::SendFailed(format!("Failed to batch message: {:?}", e)))
    }

    /// Flush batched messages
    pub async fn flush_batch(&self) -> Vec<Message> {
        let batched_data = self.message_batcher.flush_messages().await;
        batched_data.into_iter()
            .map(|data| Message {
                data,
                message_type: crate::transport::MessageType::Text,
            })
            .collect()
    }

    /// Check if batch should be flushed
    pub async fn should_flush_batch(&self) -> bool {
        self.message_batcher.should_flush().await
    }

    /// Cache a message for future retrieval
    pub async fn cache_message(&self, key: String, message: Message) {
        self.message_cache.set(key, message.data).await;
    }

    /// Retrieve a cached message
    pub async fn get_cached_message(&self, key: &str) -> Option<Message> {
        if let Some(data) = self.message_cache.get(key).await {
            Some(Message {
                data,
                message_type: crate::transport::MessageType::Text,
            })
        } else {
            None
        }
    }

    /// Get performance metrics
    pub async fn get_performance_metrics(&self) -> crate::performance::PerformanceMetrics {
        let manager = self.performance_manager.lock().await;
        manager.get_metrics().unwrap_or_else(|| crate::performance::PerformanceMetrics {
            uptime: std::time::Duration::from_secs(0),
            total_requests: 0,
            requests_per_second: 0.0,
            average_response_time: std::time::Duration::from_millis(0),
            memory_usage: 0,
            cpu_usage: 0.0,
            active_connections: 0,
            message_throughput: 0.0,
            total_operations: 0,
            average_operation_time: std::time::Duration::from_millis(0),
            network_throughput: 0.0,
        })
    }

    /// Optimize connection for performance
    pub async fn optimize_connection(&self, connection: &mut PooledConnection) -> Result<(), TransportError> {
        // Mark connection as used
        connection.mark_used();

        // Check if connection needs optimization
        if connection.request_count % 100 == 0 {
            // Perform periodic optimization
            self.cleanup_idle_connections().await;
        }

        Ok(())
    }

    /// Cleanup idle connections
    pub async fn cleanup_idle_connections(&self) {
        self.connection_pool.cleanup_idle_connections().await;
    }

    /// Get connection pool statistics
    pub async fn get_pool_stats(&self) -> PoolStats {
        PoolStats {
            active_connections: self.connection_pool.active_connections(),
            max_connections: self.connection_pool.max_connections(),
            available_connections: self.connection_pool.available_connections().await,
        }
    }

    /// Get batch statistics
    pub async fn get_batch_stats(&self) -> BatchStats {
        BatchStats {
            pending_messages: self.message_batcher.pending_count().await,
            batch_size: self.message_batcher.get_optimal_batch_size(),
            should_flush: self.message_batcher.should_flush().await,
        }
    }

    /// Get cache statistics
    pub async fn get_cache_stats(&self) -> crate::performance::CacheStats {
        self.message_cache.stats().await
    }
}

/// Connection pool statistics
#[derive(Debug, Clone)]
pub struct PoolStats {
    pub active_connections: usize,
    pub max_connections: usize,
    pub available_connections: usize,
}

/// Message batch statistics
#[derive(Debug, Clone)]
pub struct BatchStats {
    pub pending_messages: usize,
    pub batch_size: usize,
    pub should_flush: bool,
}
