//! RPC Client
//!
//! Client implementation for RPC communication

use crate::codec::JsonCodec;
use crate::rpc::correlation::RpcCorrelationManager;
use crate::rpc::types::*;
use crate::transport::{Message, MessageType};
use futures::Stream;
use serde_json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};

/// RPC client for WebSocket communication
pub struct RpcClient<T> {
    correlation_manager: Arc<RpcCorrelationManager>,
    subscriptions: Arc<RwLock<HashMap<String, RpcSubscription<T>>>>,
    message_sender: mpsc::UnboundedSender<Message>,
    response_receiver: Arc<RwLock<Option<mpsc::UnboundedReceiver<RpcResponse<T>>>>>,
    codec: JsonCodec,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> RpcClient<T>
where
    T: serde::Serialize + for<'de> serde::Deserialize<'de> + Send + Sync + 'static,
{
    pub fn new(message_sender: mpsc::UnboundedSender<Message>, codec: JsonCodec) -> Self {
        let (response_tx, response_rx) = mpsc::unbounded_channel();

        Self {
            correlation_manager: Arc::new(RpcCorrelationManager::new()),
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            message_sender,
            response_receiver: Arc::new(RwLock::new(Some(response_rx))),
            codec,
            _phantom: std::marker::PhantomData,
        }
    }

    pub async fn call<U>(
        &self,
        method_name: &str,
        params: U,
        method_type: RpcMethod,
    ) -> Result<RpcResponse<T>, RpcError>
    where
        U: serde::Serialize,
    {
        let request_id = uuid::Uuid::new_v4().to_string();
        let request = RpcRequest {
            id: request_id.clone(),
            method: method_name.to_string(),
            params,
            method_type,
        };

        // Encode the request to JSON
        let request_json = serde_json::to_string(&request).map_err(|e| RpcError {
            code: -32700,
            message: format!("Parse error: {}", e),
            data: None,
        })?;

        // Create WebSocket message
        let message = Message {
            data: request_json.into_bytes(),
            message_type: MessageType::Text,
        };

        // Send the request via WebSocket
        self.message_sender.send(message).map_err(|_| RpcError {
            code: -32603,
            message: "Internal error: Failed to send message".to_string(),
            data: None,
        })?;

        // Wait for response with timeout
        self.wait_for_response(&request_id, Duration::from_secs(30))
            .await
    }

    /// Wait for a response to a specific request ID
    async fn wait_for_response(
        &self,
        request_id: &str,
        timeout: Duration,
    ) -> Result<RpcResponse<T>, RpcError> {
        // Use the correlation manager to wait for the response
        let response_result = self
            .correlation_manager
            .wait_for_response(request_id, timeout)
            .await
            .map_err(|e| RpcError {
                code: -32603,
                message: format!("Request failed: {}", e),
                data: None,
            })?;

        match response_result {
            Ok(response) => {
                // Convert from serde_json::Value to T
                let converted_response = RpcResponse {
                    id: response.id,
                    result: response.result.and_then(|v| serde_json::from_value(v).ok()),
                    error: response.error,
                };
                Ok(converted_response)
            }
            Err(error) => Err(error),
        }
    }

    pub async fn send_request<U>(
        &self,
        method: RpcMethod,
        params: U,
    ) -> Result<RpcResponse<T>, RpcError>
    where
        U: serde::Serialize,
    {
        let method_string = method.to_string();
        self.call(&method_string, params, method).await
    }

    pub async fn subscribe(
        &self,
        params: SubscribeMessagesParams,
    ) -> Result<RpcSubscription<T>, RpcError> {
        let subscription_id = uuid::Uuid::new_v4().to_string();
        let subscription = RpcSubscription::new(subscription_id.clone());

        // For now, just return the subscription without storing it
        // In a real implementation, we'd store it and return a reference
        Ok(subscription)
    }

    pub async fn unsubscribe(&self, subscription_id: &str) -> Result<(), RpcError> {
        self.subscriptions.write().await.remove(subscription_id);
        Ok(())
    }

    /// Generate a unique ID for RPC requests
    pub fn generate_id(&self) -> String {
        uuid::Uuid::new_v4().to_string()
    }

    /// Query method for RPC calls
    pub async fn query<U>(&self, method: &str, params: U) -> Result<RpcResponse<U>, RpcError>
    where
        U: serde::Serialize + for<'de> serde::Deserialize<'de> + Send + Sync + 'static,
    {
        // Create a temporary client with the correct type
        let (message_sender, _) = mpsc::unbounded_channel();
        let codec = JsonCodec::new();
        let temp_client: RpcClient<U> = RpcClient::new(message_sender, codec);

        // Use the real RPC call implementation
        temp_client.call(method, params, RpcMethod::Query).await
    }

    /// Mutation method for RPC calls
    pub async fn mutation<U>(&self, method: &str, params: U) -> Result<RpcResponse<U>, RpcError>
    where
        U: serde::Serialize + for<'de> serde::Deserialize<'de> + Send + Sync + 'static,
    {
        // Create a temporary client with the correct type
        let (message_sender, _) = mpsc::unbounded_channel();
        let codec = JsonCodec::new();
        let temp_client: RpcClient<U> = RpcClient::new(message_sender, codec);

        // Use the real RPC call implementation
        temp_client.call(method, params, RpcMethod::Mutation).await
    }

    /// Handle incoming RPC response
    pub async fn handle_response(&self, response_data: &[u8]) -> Result<(), RpcError> {
        // Decode the response as serde_json::Value first
        let response: RpcResponse<serde_json::Value> = serde_json::from_slice(response_data)
            .map_err(|e| RpcError {
                code: -32700,
                message: format!("Parse error: {}", e),
                data: None,
            })?;

        // Send the response to the waiting request via the correlation manager
        let response_id = response.id.clone();
        self.correlation_manager
            .complete_request(&response_id, Ok(response))
            .map_err(|_| RpcError {
                code: -32603,
                message: "Failed to complete request".to_string(),
                data: None,
            })?;

        Ok(())
    }
}

/// RPC subscription for streaming responses
#[derive(Debug)]
pub struct RpcSubscription<T> {
    pub id: String,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> RpcSubscription<T> {
    pub fn new(id: String) -> Self {
        Self {
            id,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T> Stream for RpcSubscription<T> {
    type Item = T;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        // This would be implemented with actual streaming logic
        std::task::Poll::Ready(None)
    }
}
