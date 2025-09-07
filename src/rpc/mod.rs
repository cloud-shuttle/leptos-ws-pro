//! Type-safe RPC layer for leptos-ws
//!
//! Provides compile-time guarantees for all WebSocket communications through
//! procedural macros and trait-based routing.

#[cfg(feature = "advanced-rpc")]
pub mod advanced;

pub mod correlation;

use async_trait::async_trait;
use futures::Stream;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::codec::{JsonCodec, WsMessage};
use crate::reactive::WebSocketContext;
use crate::rpc::correlation::RpcCorrelationManager;

/// RPC method types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RpcMethod {
    Call,
    Query,
    Mutation,
    Subscription,
}

/// RPC request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcRequest<T> {
    pub id: String,
    pub method: String,
    pub params: T,
    pub method_type: RpcMethod,
}

/// RPC response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcResponse<T> {
    pub id: String,
    pub result: Option<T>,
    pub error: Option<RpcError>,
}

/// RPC error
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, thiserror::Error)]
#[error("RPC Error {code}: {message}")]
pub struct RpcError {
    pub code: i32,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

/// Trait for RPC services
#[async_trait]
pub trait RpcService: Send + Sync + 'static {
    type Context;

    async fn handle_request<T, R>(
        &self,
        method: &str,
        params: T,
        context: &Self::Context,
    ) -> Result<R, RpcError>
    where
        T: Deserialize<'static> + Send,
        R: Serialize + Send;
}

/// RPC client for making type-safe calls
#[allow(dead_code)]
pub struct RpcClient<T> {
    context: WebSocketContext,
    codec: JsonCodec,
    pub next_id: std::sync::atomic::AtomicU64,
    correlation_manager: RpcCorrelationManager,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> RpcClient<T>
where
    T: Serialize + for<'de> Deserialize<'de> + Clone + Send + Sync + 'static,
{
    pub fn new(context: WebSocketContext, codec: JsonCodec) -> Self {
        Self {
            context,
            codec,
            next_id: std::sync::atomic::AtomicU64::new(1),
            correlation_manager: RpcCorrelationManager::new(),
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn context(&self) -> &WebSocketContext {
        &self.context
    }

    pub fn context_mut(&mut self) -> &mut WebSocketContext {
        &mut self.context
    }

    /// Make a query call
    pub async fn query<R>(&self, method: &str, params: T) -> Result<R, RpcError>
    where
        R: for<'de> Deserialize<'de> + Send + 'static,
    {
        self.call(method, params, RpcMethod::Query).await
    }

    /// Make a mutation call
    pub async fn mutation<R>(&self, method: &str, params: T) -> Result<R, RpcError>
    where
        R: for<'de> Deserialize<'de> + Send + 'static,
    {
        self.call(method, params, RpcMethod::Mutation).await
    }

    /// Subscribe to a stream
    pub fn subscribe<R>(&self, method: &str, params: &T) -> RpcSubscription<R>
    where
        R: for<'de> Deserialize<'de> + Clone + Send + Sync + 'static,
    {
        let id = self.generate_id();
        let request = RpcRequest {
            id: id.clone(),
            method: method.to_string(),
            params: params.clone(),
            method_type: RpcMethod::Subscription,
        };

        let wrapped = WsMessage::new(request);

        // Send subscription request
        // Note: In a real implementation, this would need to be async
        // For now, we'll just store the message
        let _ = serde_json::to_vec(&wrapped);

        RpcSubscription {
            id,
            context: self.context.clone(),
            _phantom: std::marker::PhantomData,
        }
    }

    pub async fn call<R>(
        &self,
        method: &str,
        params: T,
        method_type: RpcMethod,
    ) -> Result<R, RpcError>
    where
        R: for<'de> Deserialize<'de> + Send + 'static,
    {
        let id = self.generate_id();
        let request = RpcRequest {
            id: id.clone(),
            method: method.to_string(),
            params,
            method_type,
        };

        // Encode request as JSON
        let request_json = serde_json::to_string(&request)
            .map_err(|e| RpcError {
                code: -32700,
                message: format!("Parse error: {}", e),
                data: None,
            })?;

        // Send request through WebSocket context
        let send_result = self.context.send_message(&request_json).await;

        match send_result {
            Ok(_) => {
                // Register request for correlation and wait for response
                let response_rx = self.correlation_manager.register_request(
                    id.clone(),
                    method.to_string(),
                );

                // Wait for actual response from WebSocket
                match response_rx.await {
                    Ok(Ok(response)) => {
                        // Got successful response
                        if let Some(result) = response.result {
                            serde_json::from_value(result).map_err(|e| RpcError {
                                code: -32603,
                                message: format!("Deserialization error: {}", e),
                                data: None,
                            })
                        } else if let Some(error) = response.error {
                            Err(error)
                        } else {
                            Err(RpcError {
                                code: -32603,
                                message: "Empty response received".to_string(),
                                data: None,
                            })
                        }
                    }
                    Ok(Err(rpc_error)) => {
                        // Got error response
                        Err(rpc_error)
                    }
                    Err(_) => {
                        // Channel was dropped (timeout or cancellation)
                        Err(RpcError {
                            code: -32603,
                            message: "Request was cancelled or timed out".to_string(),
                            data: None,
                        })
                    }
                }
            }
            Err(transport_error) => {
                Err(RpcError {
                    code: -32603,
                    message: format!("Transport error: {}", transport_error),
                    data: None,
                })
            }
        }
    }

    pub fn generate_id(&self) -> String {
        let id = self
            .next_id
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        format!("rpc_{}", id)
    }
}

/// RPC subscription stream
#[allow(dead_code)]
pub struct RpcSubscription<T> {
    pub id: String,
    context: WebSocketContext,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> Stream for RpcSubscription<T>
where
    T: for<'de> Deserialize<'de> + Clone + Send + Sync + 'static,
{
    type Item = Result<T, RpcError>;

    fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // Try to get messages from the WebSocket context
        let received_messages: Vec<String> = self.context.get_received_messages();

        // Filter messages for this subscription ID
        for message_json in received_messages {
            // Try to parse as RPC response
            if let Ok(response) = serde_json::from_str::<RpcResponse<serde_json::Value>>(&message_json) {
                if response.id == self.id {
                    // This is for our subscription
                    if let Some(result) = response.result {
                        // Try to deserialize the result to our target type
                        match serde_json::from_value::<T>(result) {
                            Ok(data) => return Poll::Ready(Some(Ok(data))),
                            Err(e) => return Poll::Ready(Some(Err(RpcError {
                                code: -32603,
                                message: format!("Deserialization error: {}", e),
                                data: None,
                            }))),
                        }
                    } else if let Some(error) = response.error {
                        return Poll::Ready(Some(Err(error)));
                    }
                }
            }
        }

        // No matching messages found, return Pending
        // In a real implementation, this would register a waker
        Poll::Pending
    }
}

/// Hook for using RPC client
pub fn use_rpc_client<T>(context: WebSocketContext) -> RpcClient<T>
where
    T: Serialize + for<'de> Deserialize<'de> + Clone + Send + Sync + 'static,
{
    RpcClient::<T>::new(context, JsonCodec)
}

/// Macro for defining RPC services
#[macro_export]
macro_rules! rpc_service {
    (
        $service_name:ident {
            $(
                $(#[$attr:meta])*
                $method_name:ident($params:ty) -> $return_type:ty
            ),* $(,)?
        }
    ) => {
        pub struct $service_name;

        impl $service_name {
            $(
                $(#[$attr])*
                pub async fn $method_name(
                    _params: $params,
                ) -> Result<$return_type, RpcError> {
                    // Implementation would be generated here
                    todo!("Generated implementation for {}", stringify!($method_name))
                }
            )*
        }
    };
}

// Example RPC service definition
rpc_service! {
    ChatService {
        send_message(SendMessageParams) -> MessageId,
        get_messages(GetMessagesParams) -> Vec<ChatMessage>,
        subscribe_messages(SubscribeMessagesParams) -> ChatMessage,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessageParams {
    pub room_id: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetMessagesParams {
    pub room_id: String,
    pub limit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscribeMessagesParams {
    pub room_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageId {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub room_id: String,
    pub content: String,
    pub sender: String,
    pub timestamp: u64,
}

/// Component for using RPC in Leptos
#[component]
pub fn RpcProvider(children: Children, context: WebSocketContext) -> impl IntoView {
    // For now, we'll provide a simple context
    // In a real implementation, this would create an RpcClient
    provide_context(context);

    children()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rpc_request_creation() {
        let request = RpcRequest {
            id: "test_id".to_string(),
            method: "test_method".to_string(),
            params: "test_params",
            method_type: RpcMethod::Query,
        };

        assert_eq!(request.id, "test_id");
        assert_eq!(request.method, "test_method");
        assert_eq!(request.method_type, RpcMethod::Query);
    }

    #[test]
    fn test_rpc_response_creation() {
        let response = RpcResponse {
            id: "test_id".to_string(),
            result: Some("test_result"),
            error: None,
        };

        assert_eq!(response.id, "test_id");
        assert_eq!(response.result, Some("test_result"));
        assert!(response.error.is_none());
    }

    #[test]
    fn test_rpc_error_creation() {
        let error = RpcError {
            code: 404,
            message: "Not found".to_string(),
            data: None,
        };

        assert_eq!(error.code, 404);
        assert_eq!(error.message, "Not found");
    }

    #[tokio::test]
    async fn test_chat_service_definition() {
        let _params = SendMessageParams {
            room_id: "room1".to_string(),
            content: "Hello, World!".to_string(),
        };

        // This would call the generated implementation
        // let result = ChatService::send_message(params).await;
        // assert!(result.is_ok());
    }
}
