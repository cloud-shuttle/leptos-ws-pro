//! Connection Pool
//!
//! Manages a pool of reusable WebSocket connections for improved performance

use crate::performance::metrics::PerformanceError;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Connection pool configuration
#[derive(Debug, Clone)]
pub struct ConnectionPoolConfig {
    pub max_connections: usize,
    pub min_connections: usize,
}

impl Default for ConnectionPoolConfig {
    fn default() -> Self {
        Self {
            max_connections: 10,
            min_connections: 2,
        }
    }
}

/// Connection pool for reusing WebSocket connections
pub struct ConnectionPool {
    max_size: usize,
    connections: Arc<RwLock<HashMap<String, VecDeque<PooledConnection>>>>,
    total_connections: Arc<Mutex<usize>>,
}

impl ConnectionPool {
    pub fn new_simple(max_size: usize) -> Self {
        Self {
            max_size,
            connections: Arc::new(RwLock::new(HashMap::new())),
            total_connections: Arc::new(Mutex::new(0)),
        }
    }

    /// Create a connection pool with configuration
    pub async fn new(config: ConnectionPoolConfig) -> Result<Self, PerformanceError> {
        let pool = Self {
            max_size: config.max_connections,
            connections: Arc::new(RwLock::new(HashMap::new())),
            total_connections: Arc::new(Mutex::new(0)),
        };

        // Initialize with minimum connections
        for i in 0..config.min_connections {
            let url = format!("ws://localhost:8080/{}", i);
            let connection = PooledConnection::new(url);
            pool.connections
                .write()
                .await
                .entry(connection.url.clone())
                .or_insert_with(VecDeque::new)
                .push_back(connection);
        }
        *pool.total_connections.lock().unwrap() = config.min_connections;

        Ok(pool)
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
            let pool = connections
                .entry(connection.url.clone())
                .or_insert_with(VecDeque::new);
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

    /// Get the number of active connections
    pub fn active_connections(&self) -> usize {
        *self.total_connections.lock().unwrap()
    }

    /// Get the maximum number of connections
    pub fn max_connections(&self) -> usize {
        self.max_size
    }

    /// Get the number of available connections
    pub async fn available_connections(&self) -> usize {
        let connections = self.connections.read().await;
        connections.values().map(|pool| pool.len()).sum()
    }

    /// Simulate connection failure for testing
    pub async fn simulate_connection_failure(&self, count: usize) {
        let mut connections = self.connections.write().await;
        let mut removed = 0;

        for pool in connections.values_mut() {
            while removed < count && !pool.is_empty() {
                pool.pop_front();
                removed += 1;
            }
            if removed >= count {
                break;
            }
        }

        *self.total_connections.lock().unwrap() -= removed;
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
