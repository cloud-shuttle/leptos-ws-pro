//! Message Cache
//!
//! High-performance caching system for frequently accessed messages

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

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

        cache.insert(
            key,
            CacheEntry {
                value,
                created_at: Instant::now(),
                expires_at: Instant::now() + self.ttl,
                access_count: 1,
            },
        );
    }

    fn evict_oldest(&self, cache: &mut HashMap<String, CacheEntry>) {
        if let Some(oldest_key) = cache
            .iter()
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
