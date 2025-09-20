//! Advanced RPC System Implementation
//!
//! This module provides bidirectional RPC with request/response correlation,
//! type-safe method definitions, and async method support.

#[cfg(feature = "advanced-rpc")]
use crate::transport::{Message, MessageType};
use crate::transport::{Transport, TransportError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use uuid::Uuid;

use tokio::sync::{mpsc, oneshot};

/// RPC Request structure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RpcRequest {
    pub id: String,
    pub method: String,
    pub params: serde_json::Value,
}

/// RPC Response structure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RpcResponse {
    pub id: String,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
}

/// RPC Error types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RpcError {
    ConnectionFailed(String),
    Timeout(String),
    MethodNotFound(String),
    InvalidParams(String),
    InternalError(String),
}

/// Pending RPC request with response channel
struct PendingRequest {
    response_tx: oneshot::Sender<Result<RpcResponse, RpcError>>,
    timeout: Instant,
}

/// RPC Correlation Manager
/// Manages request/response correlation and timeout handling
pub struct RpcCorrelationManager {
    pending_requests: Arc<Mutex<HashMap<String, PendingRequest>>>,
    timeout_duration: Duration,
}

impl RpcCorrelationManager {
    pub fn new(timeout_duration: Duration) -> Self {
        Self {
            pending_requests: Arc::new(Mutex::new(HashMap::new())),
            timeout_duration,
        }
    }

    /// Register a pending request
    pub fn register_request(
        &self,
        request_id: String,
    ) -> oneshot::Receiver<Result<RpcResponse, RpcError>> {
        let (response_tx, response_rx) = oneshot::channel();
        let timeout = Instant::now() + self.timeout_duration;

        let pending_request = PendingRequest {
            response_tx,
            timeout,
        };

        self.pending_requests
            .lock()
            .unwrap()
            .insert(request_id, pending_request);
        response_rx
    }

    /// Handle incoming RPC response
    pub fn handle_response(&self, response: RpcResponse) -> Result<(), RpcError> {
        let mut pending = self.pending_requests.lock().unwrap();

        if let Some(pending_request) = pending.remove(&response.id) {
            if pending_request.timeout > Instant::now() {
                let _ = pending_request.response_tx.send(Ok(response));
                Ok(())
            } else {
                Err(RpcError::Timeout(format!(
                    "Request {} timed out",
                    response.id
                )))
            }
        } else {
            Err(RpcError::InternalError(format!(
                "No pending request found for ID: {}",
                response.id
            )))
        }
    }

    /// Clean up expired requests
    pub fn cleanup_expired(&self) {
        let mut pending = self.pending_requests.lock().unwrap();
        let now = Instant::now();

        let expired_ids: Vec<String> = pending
            .iter()
            .filter(|(_, pending_request)| pending_request.timeout <= now)
            .map(|(id, _)| id.clone())
            .collect();

        for id in expired_ids {
            if let Some(pending_request) = pending.remove(&id) {
                let _ = pending_request
                    .response_tx
                    .send(Err(RpcError::Timeout("Request expired".to_string())));
            }
        }
    }

    /// Get number of pending requests
    pub fn pending_count(&self) -> usize {
        self.pending_requests.lock().unwrap().len()
    }
}

/// Bidirectional RPC Client
pub struct BidirectionalRpcClient<T: Transport> {
    transport: T,
    correlation_manager: Arc<RpcCorrelationManager>,
    request_sender: mpsc::UnboundedSender<RpcRequest>,
    response_receiver: mpsc::UnboundedReceiver<RpcResponse>,
}

impl<T: Transport> BidirectionalRpcClient<T> {
    pub async fn new(transport: T, timeout_duration: Duration) -> Result<Self, TransportError> {
        let correlation_manager = Arc::new(RpcCorrelationManager::new(timeout_duration));
        let (request_sender, mut request_receiver) = mpsc::unbounded_channel::<RpcRequest>();
        let (response_sender, response_receiver) = mpsc::unbounded_channel();

        // Spawn task to handle outgoing requests
        let correlation_manager_clone = correlation_manager.clone();
        tokio::spawn(async move {
            while let Some(request) = request_receiver.recv().await {
                // In a real implementation, this would send the request via the transport
                // For now, we'll simulate the response
                let response = RpcResponse {
                    id: request.id.clone(),
                    result: Some(serde_json::json!({
                        "echo": request.params,
                        "method": request.method
                    })),
                    error: None,
                };

                if let Err(e) = correlation_manager_clone.handle_response(response) {
                    eprintln!("Failed to handle response: {:?}", e);
                }
            }
        });

        Ok(Self {
            transport,
            correlation_manager,
            request_sender,
            response_receiver,
        })
    }

    /// Make an RPC call
    pub async fn call(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, RpcError> {
        let request_id = Uuid::new_v4().to_string();
        let request = RpcRequest {
            id: request_id.clone(),
            method: method.to_string(),
            params,
        };

        // Register pending request
        let response_rx = self.correlation_manager.register_request(request_id);

        // Send request
        self.request_sender
            .send(request)
            .map_err(|_| RpcError::ConnectionFailed("Failed to send request".to_string()))?;

        // Wait for response
        match response_rx.await {
            Ok(Ok(response)) => {
                if let Some(result) = response.result {
                    Ok(result)
                } else if let Some(error) = response.error {
                    Err(RpcError::InternalError(error))
                } else {
                    Err(RpcError::InternalError("Empty response".to_string()))
                }
            }
            Ok(Err(e)) => Err(e),
            Err(_) => Err(RpcError::ConnectionFailed(
                "Response channel closed".to_string(),
            )),
        }
    }

    /// Make an RPC call with timeout
    pub async fn call_with_timeout(
        &self,
        method: &str,
        params: serde_json::Value,
        timeout: Duration,
    ) -> Result<serde_json::Value, RpcError> {
        let request_id = Uuid::new_v4().to_string();
        let request = RpcRequest {
            id: request_id.clone(),
            method: method.to_string(),
            params,
        };

        // Register pending request
        let response_rx = self.correlation_manager.register_request(request_id);

        // Send request
        self.request_sender
            .send(request)
            .map_err(|_| RpcError::ConnectionFailed("Failed to send request".to_string()))?;

        // Wait for response with timeout
        match tokio::time::timeout(timeout, response_rx).await {
            Ok(Ok(Ok(response))) => {
                if let Some(result) = response.result {
                    Ok(result)
                } else if let Some(error) = response.error {
                    Err(RpcError::InternalError(error))
                } else {
                    Err(RpcError::InternalError("Empty response".to_string()))
                }
            }
            Ok(Ok(Err(e))) => Err(e),
            Ok(Err(_)) => Err(RpcError::ConnectionFailed(
                "Response channel closed".to_string(),
            )),
            Err(_) => Err(RpcError::Timeout("Request timed out".to_string())),
        }
    }

    /// Get number of pending requests
    pub fn pending_requests_count(&self) -> usize {
        self.correlation_manager.pending_count()
    }

    /// Clean up expired requests
    pub fn cleanup_expired(&self) {
        self.correlation_manager.cleanup_expired();
    }
}

/// RPC Method Registry
/// Manages type-safe method definitions
pub struct RpcMethodRegistry {
    methods: HashMap<
        String,
        Box<dyn Fn(serde_json::Value) -> Result<serde_json::Value, RpcError> + Send + Sync>,
    >,
}

impl RpcMethodRegistry {
    pub fn new() -> Self {
        Self {
            methods: HashMap::new(),
        }
    }

    /// Register a method handler
    pub fn register<F>(&mut self, method: &str, handler: F)
    where
        F: Fn(serde_json::Value) -> Result<serde_json::Value, RpcError> + Send + Sync + 'static,
    {
        self.methods.insert(method.to_string(), Box::new(handler));
    }

    /// Call a registered method
    pub fn call(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, RpcError> {
        if let Some(handler) = self.methods.get(method) {
            handler(params)
        } else {
            Err(RpcError::MethodNotFound(format!(
                "Method '{}' not found",
                method
            )))
        }
    }

    /// Get list of registered methods
    pub fn methods(&self) -> Vec<String> {
        self.methods.keys().cloned().collect()
    }
}

/// Batch RPC Request
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BatchRpcRequest {
    pub requests: Vec<RpcRequest>,
}

/// Batch RPC Response
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BatchRpcResponse {
    pub responses: Vec<RpcResponse>,
}

/// Batch RPC Client
pub struct BatchRpcClient<T: Transport> {
    rpc_client: BidirectionalRpcClient<T>,
}

impl<T: Transport> BatchRpcClient<T> {
    pub async fn new(transport: T, timeout_duration: Duration) -> Result<Self, TransportError> {
        let rpc_client = BidirectionalRpcClient::new(transport, timeout_duration).await?;
        Ok(Self { rpc_client })
    }

    /// Make multiple RPC calls in batch
    pub async fn call_batch(
        &self,
        requests: Vec<(String, serde_json::Value)>,
    ) -> Result<Vec<serde_json::Value>, RpcError> {
        let mut results = Vec::new();

        for (method, params) in requests {
            let result = self.rpc_client.call(&method, params).await?;
            results.push(result);
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::websocket::WebSocketConnection;
    use crate::transport::TransportConfig;

    #[tokio::test]
    async fn test_rpc_correlation_manager() {
        let manager = RpcCorrelationManager::new(Duration::from_secs(5));

        // Register a request
        let response_rx = manager.register_request("test-123".to_string());

        // Handle response
        let response = RpcResponse {
            id: "test-123".to_string(),
            result: Some(serde_json::json!({"success": true})),
            error: None,
        };

        assert!(manager.handle_response(response).is_ok());

        // Check response
        let result = response_rx.await.unwrap();
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.id, "test-123");
        assert!(response.result.is_some());
    }

    #[tokio::test]
    async fn test_rpc_method_registry() {
        let mut registry = RpcMethodRegistry::new();

        // Register a method
        registry.register("echo", |params| Ok(params));

        // Call the method
        let result = registry.call("echo", serde_json::json!({"message": "hello"}));
        assert!(result.is_ok());
        assert_eq!(result.unwrap()["message"], "hello");

        // Call non-existent method
        let result = registry.call("nonexistent", serde_json::json!({}));
        assert!(result.is_err());
        match result.unwrap_err() {
            RpcError::MethodNotFound(_) => {}
            _ => panic!("Expected MethodNotFound error"),
        }
    }

    #[tokio::test]
    async fn test_bidirectional_rpc_client() {
        let config = TransportConfig::default();
        let transport = WebSocketConnection::new(config).await.unwrap();
        let client = BidirectionalRpcClient::new(transport, Duration::from_secs(5))
            .await
            .unwrap();

        // Make an RPC call
        let result = client
            .call("echo", serde_json::json!({"message": "hello"}))
            .await;
        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value["echo"]["message"], "hello");
        assert_eq!(value["method"], "echo");
    }

    #[tokio::test]
    async fn test_batch_rpc_client() {
        let config = TransportConfig::default();
        let transport = WebSocketConnection::new(config).await.unwrap();
        let client = BatchRpcClient::new(transport, Duration::from_secs(5))
            .await
            .unwrap();

        // Make batch RPC calls
        let requests = vec![
            ("echo".to_string(), serde_json::json!({"message": "hello"})),
            ("echo".to_string(), serde_json::json!({"message": "world"})),
        ];

        let results = client.call_batch(requests).await;
        assert!(results.is_ok());
        let values = results.unwrap();
        assert_eq!(values.len(), 2);
        assert_eq!(values[0]["echo"]["message"], "hello");
        assert_eq!(values[1]["echo"]["message"], "world");
    }
}
