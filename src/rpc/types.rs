//! RPC Types
//!
//! Core types for RPC communication

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// RPC method types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RpcMethod {
    Call,
    Subscription,
    Query,
    Mutation,
    SendMessage,
    GetMessages,
    SubscribeMessages,
    UnsubscribeMessages,
    GetStats,
    Echo,
    Broadcast,
}

impl std::fmt::Display for RpcMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RpcMethod::Call => write!(f, "Call"),
            RpcMethod::Subscription => write!(f, "Subscription"),
            RpcMethod::Query => write!(f, "Query"),
            RpcMethod::Mutation => write!(f, "Mutation"),
            RpcMethod::SendMessage => write!(f, "SendMessage"),
            RpcMethod::GetMessages => write!(f, "GetMessages"),
            RpcMethod::SubscribeMessages => write!(f, "SubscribeMessages"),
            RpcMethod::UnsubscribeMessages => write!(f, "UnsubscribeMessages"),
            RpcMethod::GetStats => write!(f, "GetStats"),
            RpcMethod::Echo => write!(f, "Echo"),
            RpcMethod::Broadcast => write!(f, "Broadcast"),
        }
    }
}

/// RPC request structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcRequest<T> {
    pub id: String,
    pub method: String,
    pub params: T,
    pub method_type: RpcMethod,
}

/// RPC response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcResponse<T> {
    pub id: String,
    pub result: Option<T>,
    pub error: Option<RpcError>,
}

/// RPC error structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcError {
    pub code: i32,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

impl std::fmt::Display for RpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RPC Error {}: {}", self.code, self.message)
    }
}

impl std::error::Error for RpcError {}

impl RpcError {
    pub fn new(code: i32, message: String) -> Self {
        Self {
            code,
            message,
            data: None,
        }
    }

    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(data);
        self
    }
}

/// RPC service trait
pub trait RpcService<T> {
    async fn handle_request(&self, request: RpcRequest<T>) -> Result<RpcResponse<T>, RpcError>;
}

/// Send message parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessageParams {
    pub message: String,
    pub channel: Option<String>,
    pub content: Option<String>,
    pub room_id: Option<String>,
}

/// Get messages parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetMessagesParams {
    pub channel: Option<String>,
    pub limit: Option<usize>,
    pub room_id: Option<String>,
}

/// Subscribe messages parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscribeMessagesParams {
    pub channel: Option<String>,
    pub room_id: Option<String>,
}

/// Message ID
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct MessageId {
    pub id: String,
}

/// Chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub content: String,
    pub timestamp: u64,
    pub channel: Option<String>,
    pub sender: Option<String>,
    pub room_id: Option<String>,
}
