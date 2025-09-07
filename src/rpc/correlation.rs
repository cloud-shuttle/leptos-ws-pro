//! Real RPC Request/Response Correlation System
//!
//! Provides production-ready correlation of RPC requests with WebSocket responses

use crate::rpc::{RpcError, RpcResponse};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::oneshot;
use serde::{Deserialize, Serialize};

/// Pending RPC request awaiting response
struct PendingRequest {
    /// Channel to send response back to caller
    response_tx: oneshot::Sender<Result<RpcResponse<serde_json::Value>, RpcError>>,
    /// When this request times out
    timeout_at: Instant,
    /// Method name for debugging
    method: String,
}

/// RPC Correlation Manager handles request/response correlation
#[derive(Clone)]
pub struct RpcCorrelationManager {
    /// Map of request ID -> pending request
    pending_requests: Arc<Mutex<HashMap<String, PendingRequest>>>,
    /// Default timeout for requests
    default_timeout: Duration,
}

impl RpcCorrelationManager {
    /// Create new correlation manager with default 30-second timeout
    pub fn new() -> Self {
        Self::with_timeout(Duration::from_secs(30))
    }

    /// Create correlation manager with custom timeout
    pub fn with_timeout(timeout: Duration) -> Self {
        Self {
            pending_requests: Arc::new(Mutex::new(HashMap::new())),
            default_timeout: timeout,
        }
    }

    /// Register a new pending request
    /// Returns a receiver that will get the response when it arrives
    pub fn register_request(
        &self,
        request_id: String,
        method: String,
    ) -> oneshot::Receiver<Result<RpcResponse<serde_json::Value>, RpcError>> {
        let (response_tx, response_rx) = oneshot::channel();

        let pending_request = PendingRequest {
            response_tx,
            timeout_at: Instant::now() + self.default_timeout,
            method,
        };

        {
            let mut pending = self.pending_requests.lock().unwrap();
            pending.insert(request_id, pending_request);
        }

        response_rx
    }

    /// Handle incoming RPC response, correlating it with pending request
    pub fn handle_response(&self, response: RpcResponse<serde_json::Value>) -> Result<(), RpcError> {
        let mut pending = self.pending_requests.lock().unwrap();

        if let Some(pending_request) = pending.remove(&response.id) {
            // Check if request has timed out
            if Instant::now() > pending_request.timeout_at {
                return Err(RpcError {
                    code: -32603,
                    message: format!("Request {} timed out", response.id),
                    data: None,
                });
            }

            // Send response back to caller
            match pending_request.response_tx.send(Ok(response)) {
                Ok(_) => Ok(()),
                Err(_) => Err(RpcError {
                    code: -32603,
                    message: "Caller dropped request before response arrived".to_string(),
                    data: None,
                }),
            }
        } else {
            Err(RpcError {
                code: -32603,
                message: format!("No pending request found for ID: {}", response.id),
                data: None,
            })
        }
    }

    /// Handle incoming RPC error response
    pub fn handle_error_response(&self, request_id: String, error: RpcError) -> Result<(), RpcError> {
        let mut pending = self.pending_requests.lock().unwrap();

        if let Some(pending_request) = pending.remove(&request_id) {
            // Send error back to caller
            match pending_request.response_tx.send(Err(error)) {
                Ok(_) => Ok(()),
                Err(_) => Err(RpcError {
                    code: -32603,
                    message: "Caller dropped request before error response arrived".to_string(),
                    data: None,
                }),
            }
        } else {
            Err(RpcError {
                code: -32603,
                message: format!("No pending request found for error response ID: {}", request_id),
                data: None,
            })
        }
    }

    /// Clean up expired/timed out requests
    /// Returns number of requests cleaned up
    pub fn cleanup_expired(&self) -> usize {
        let mut pending = self.pending_requests.lock().unwrap();
        let now = Instant::now();

        let expired_ids: Vec<String> = pending
            .iter()
            .filter(|(_, request)| now > request.timeout_at)
            .map(|(id, _)| id.clone())
            .collect();

        let cleanup_count = expired_ids.len();

        for id in expired_ids {
            if let Some(expired_request) = pending.remove(&id) {
                let timeout_error = RpcError {
                    code: -32603,
                    message: format!("Request {} timed out after {:?}", id, self.default_timeout),
                    data: Some(serde_json::json!({
                        "method": expired_request.method,
                        "timeout_duration_secs": self.default_timeout.as_secs()
                    })),
                };

                // Try to notify caller of timeout (may fail if caller dropped)
                let _ = expired_request.response_tx.send(Err(timeout_error));
            }
        }

        cleanup_count
    }

    /// Get number of currently pending requests
    pub fn pending_count(&self) -> usize {
        self.pending_requests.lock().unwrap().len()
    }

    /// Get list of pending request IDs (for debugging)
    pub fn pending_request_ids(&self) -> Vec<String> {
        self.pending_requests.lock().unwrap().keys().cloned().collect()
    }

    /// Cancel a specific pending request
    pub fn cancel_request(&self, request_id: &str) -> bool {
        let mut pending = self.pending_requests.lock().unwrap();

        if let Some(cancelled_request) = pending.remove(request_id) {
            let cancel_error = RpcError {
                code: -32603,
                message: format!("Request {} was cancelled", request_id),
                data: None,
            };

            // Notify caller of cancellation
            let _ = cancelled_request.response_tx.send(Err(cancel_error));
            true
        } else {
            false
        }
    }

    /// Cancel all pending requests
    pub fn cancel_all(&self) -> usize {
        let mut pending = self.pending_requests.lock().unwrap();
        let count = pending.len();

        for (request_id, cancelled_request) in pending.drain() {
            let cancel_error = RpcError {
                code: -32603,
                message: format!("Request {} was cancelled due to shutdown", request_id),
                data: None,
            };

            let _ = cancelled_request.response_tx.send(Err(cancel_error));
        }

        count
    }
}

impl Default for RpcCorrelationManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Background task that periodically cleans up expired requests
pub struct CorrelationCleanupTask {
    manager: RpcCorrelationManager,
    cleanup_interval: Duration,
}

impl CorrelationCleanupTask {
    /// Create new cleanup task
    pub fn new(manager: RpcCorrelationManager) -> Self {
        Self {
            manager,
            cleanup_interval: Duration::from_secs(10), // Clean up every 10 seconds
        }
    }

    /// Create cleanup task with custom interval
    pub fn with_interval(manager: RpcCorrelationManager, interval: Duration) -> Self {
        Self {
            manager,
            cleanup_interval: interval,
        }
    }

    /// Run the cleanup task (should be spawned as background task)
    pub async fn run(&self) {
        let mut interval = tokio::time::interval(self.cleanup_interval);

        loop {
            interval.tick().await;
            let cleaned_up = self.manager.cleanup_expired();

            if cleaned_up > 0 {
                tracing::debug!("Cleaned up {} expired RPC requests", cleaned_up);
            }
        }
    }
}

/// Statistics about correlation manager performance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrelationStats {
    pub pending_requests: usize,
    pub total_requests_processed: u64,
    pub total_timeouts: u64,
    pub total_cancellations: u64,
    pub average_response_time_ms: f64,
}

impl CorrelationStats {
    pub fn new() -> Self {
        Self {
            pending_requests: 0,
            total_requests_processed: 0,
            total_timeouts: 0,
            total_cancellations: 0,
            average_response_time_ms: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_correlation_manager_basic() {
        let manager = RpcCorrelationManager::new();

        // Register a request
        let request_id = "test_123".to_string();
        let method = "test_method".to_string();
        let response_rx = manager.register_request(request_id.clone(), method.clone());

        assert_eq!(manager.pending_count(), 1);

        // Simulate response
        let response = RpcResponse {
            id: request_id.clone(),
            result: Some(serde_json::json!({"success": true})),
            error: None,
        };

        // Handle response
        assert!(manager.handle_response(response).is_ok());

        // Should have received response
        let result = response_rx.await.unwrap();
        assert!(result.is_ok());
        let rpc_response = result.unwrap();
        assert_eq!(rpc_response.id, request_id);

        // Should no longer be pending
        assert_eq!(manager.pending_count(), 0);
    }

    #[tokio::test]
    async fn test_correlation_manager_timeout() {
        let manager = RpcCorrelationManager::with_timeout(Duration::from_millis(100));

        // Register a request
        let request_id = "timeout_test".to_string();
        let method = "timeout_method".to_string();
        let response_rx = manager.register_request(request_id.clone(), method);

        // Wait for timeout
        sleep(Duration::from_millis(200)).await;

        // Clean up expired requests
        let cleaned_up = manager.cleanup_expired();
        assert_eq!(cleaned_up, 1);

        // Should have received timeout error
        let result = response_rx.await.unwrap();
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.message.contains("timed out"));
    }

    #[tokio::test]
    async fn test_correlation_manager_error_response() {
        let manager = RpcCorrelationManager::new();

        // Register a request
        let request_id = "error_test".to_string();
        let method = "error_method".to_string();
        let response_rx = manager.register_request(request_id.clone(), method);

        // Simulate error response
        let error = RpcError {
            code: 404,
            message: "Method not found".to_string(),
            data: None,
        };

        assert!(manager.handle_error_response(request_id, error.clone()).is_ok());

        // Should have received error
        let result = response_rx.await.unwrap();
        assert!(result.is_err());
        let received_error = result.unwrap_err();
        assert_eq!(received_error.code, 404);
        assert_eq!(received_error.message, "Method not found");
    }

    #[tokio::test]
    async fn test_correlation_manager_cancellation() {
        let manager = RpcCorrelationManager::new();

        // Register a request
        let request_id = "cancel_test".to_string();
        let method = "cancel_method".to_string();
        let response_rx = manager.register_request(request_id.clone(), method);

        // Cancel the request
        let cancelled = manager.cancel_request(&request_id);
        assert!(cancelled);

        // Should have received cancellation error
        let result = response_rx.await.unwrap();
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.message.contains("cancelled"));
    }
}
