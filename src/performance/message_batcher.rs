//! Message Batcher
//!
//! Batches messages for improved throughput and reduced network overhead

use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use crate::performance::metrics::PerformanceError;

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
        let should_flush = {
            let mut pending = self.pending_messages.lock().await;
            pending.push_back(message);
            pending.len() >= self.batch_size
        };

        // Auto-flush if batch is full
        if should_flush {
            self.flush_messages().await;
        }

        Ok(())
    }

    pub async fn flush_messages(&self) -> Vec<Vec<u8>> {
        let mut pending = self.pending_messages.lock().await;
        let messages: Vec<_> = pending.drain(..).collect();
        *self.last_flush.lock().await = Instant::now();
        messages
    }

    pub async fn should_flush(&self) -> bool {
        let pending = self.pending_messages.lock().await;
        let last_flush = self.last_flush.lock().await;

        pending.len() >= self.batch_size ||
        last_flush.elapsed() >= self.batch_timeout
    }

    pub async fn pending_count(&self) -> usize {
        let pending = self.pending_messages.lock().await;
        pending.len()
    }

    /// Get the length of the current batch
    pub async fn len(&self) -> usize {
        let pending = self.pending_messages.lock().await;
        pending.len()
    }

    /// Get optimal batch size
    pub fn get_optimal_batch_size(&self) -> usize {
        self.batch_size
    }
}
