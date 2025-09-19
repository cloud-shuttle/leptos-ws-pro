//! Unified transport layer for leptos-ws
//!
//! This module provides a unified abstraction over different transport protocols
//! including WebSocket, WebTransport, and Server-Sent Events with automatic
//! platform detection and progressive enhancement.

use async_trait::async_trait;
use futures::{Sink, Stream};
// use std::error::Error as StdError; // TODO: Remove when used
// use std::fmt; // TODO: Remove when used
use std::pin::Pin;

pub mod adaptive;
pub mod sse;
pub mod websocket;
pub mod webtransport;
pub mod optimized;

// Re-export main types
// Transport and TransportError are defined below in this module

/// A unified message type that can be sent over any transport
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Message {
    pub data: Vec<u8>,
    pub message_type: MessageType,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum MessageType {
    Text,
    Binary,
    Ping,
    Pong,
    Close,
}

/// Transport-level errors
#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Connection closed")]
    ConnectionClosed,

    #[error("Message send failed: {0}")]
    SendFailed(String),

    #[error("Message receive failed: {0}")]
    ReceiveFailed(String),

    #[error("Protocol error: {0}")]
    ProtocolError(String),

    #[error("Authentication failed: {0}")]
    AuthFailed(String),

    #[error("Rate limit exceeded")]
    RateLimited,

    #[error("Transport not supported: {0}")]
    NotSupported(String),

    #[error("Not connected")]
    NotConnected,
}

/// Connection state for monitoring
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Failed,
}

/// Transport capabilities for progressive enhancement
#[derive(Debug, Clone, Default)]
pub struct TransportCapabilities {
    pub websocket: bool,
    pub webtransport: bool,
    pub sse: bool,
    pub compression: bool,
    pub binary: bool,
}

impl TransportCapabilities {
    pub fn detect() -> Self {
        #[cfg(target_arch = "wasm32")]
        {
            Self {
                websocket: true,
                webtransport: webtransport::is_supported(),
                sse: true,
                compression: false, // Browser handles this
                binary: true,
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            Self {
                websocket: true,
                webtransport: false, // Not yet available in native
                sse: true,
                compression: true,
                binary: true,
            }
        }
    }

    pub fn supports_webtransport(&self) -> bool {
        self.webtransport
    }

    pub fn supports_websocket(&self) -> bool {
        self.websocket
    }

    pub fn supports_sse(&self) -> bool {
        self.sse
    }

    pub fn supports_streaming(&self) -> bool {
        self.websocket || self.sse
    }

    pub fn supports_multiplexing(&self) -> bool {
        self.webtransport
    }

    pub fn supports_server_sent_events(&self) -> bool {
        self.sse
    }

    pub fn supports_automatic_reconnection(&self) -> bool {
        self.websocket || self.sse
    }
}

/// The core transport trait that all implementations must provide
#[async_trait]
pub trait Transport: Send + Sync + 'static {
    type Stream: Stream<Item = Result<Message, TransportError>> + Send + Unpin;
    type Sink: Sink<Message, Error = TransportError> + Send + Unpin;

    /// Connect to the specified URL
    async fn connect(&mut self, url: &str) -> Result<(), TransportError>;

    /// Disconnect from the transport
    async fn disconnect(&mut self) -> Result<(), TransportError>;

    /// Split the connection into separate stream and sink
    fn split(self) -> (Self::Stream, Self::Sink);

    /// Get the connection state
    fn state(&self) -> ConnectionState;

    /// Send a message (default implementation for compatibility)
    async fn send_message(&self, _message: &Message) -> Result<(), TransportError> {
        // Default implementation returns not supported
        Err(TransportError::NotSupported("send_message not implemented".to_string()))
    }

    /// Receive a message (default implementation for compatibility)
    async fn receive_message(&self) -> Result<Message, TransportError> {
        // Default implementation returns not supported
        Err(TransportError::NotSupported("receive_message not implemented".to_string()))
    }

    /// Create a bidirectional stream (WebTransport specific)
    async fn create_bidirectional_stream(&mut self) -> Result<(), TransportError> {
        Err(TransportError::NotSupported("Bidirectional streams not supported".to_string()))
    }
}

/// A connection that can be split into separate stream and sink
pub trait Splittable: Transport {
    /// Split the connection into separate stream and sink
    fn split(self) -> (Self::Stream, Self::Sink);
}

/// Configuration for transport connections
#[derive(Debug, Clone)]
pub struct TransportConfig {
    pub url: String,
    pub protocols: Vec<String>,
    pub headers: std::collections::HashMap<String, String>,
    pub timeout: std::time::Duration,
    pub connection_timeout: std::time::Duration,
    pub heartbeat_interval: Option<std::time::Duration>,
    pub max_reconnect_attempts: Option<usize>,
    pub reconnect_delay: std::time::Duration,
    pub max_message_size: usize,
    pub enable_compression: bool,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            protocols: Vec::new(),
            headers: std::collections::HashMap::new(),
            timeout: std::time::Duration::from_secs(30),
            connection_timeout: std::time::Duration::from_secs(10),
            heartbeat_interval: Some(std::time::Duration::from_secs(30)),
            max_reconnect_attempts: Some(5),
            reconnect_delay: std::time::Duration::from_secs(1),
            max_message_size: 1024 * 1024, // 1MB
            enable_compression: false,
        }
    }
}

/// Transport factory for creating connections
pub struct TransportFactory;

impl TransportFactory {
    /// Create the best available transport for the given URL
    pub async fn create_adaptive(
        config: TransportConfig,
    ) -> Result<
        Box<
            dyn Transport<
                    Stream = Pin<
                        Box<dyn Stream<Item = Result<Message, TransportError>> + Send + Unpin>,
                    >,
                    Sink = Pin<Box<dyn Sink<Message, Error = TransportError> + Send + Unpin>>,
                >,
        >,
        TransportError,
    > {
        let capabilities = TransportCapabilities::detect();

        // Try WebTransport first if available
        if capabilities.webtransport && config.url.starts_with("https://") {
            if let Ok(transport) = webtransport::WebTransportConnection::new(config.clone()).await {
                return Ok(Box::new(transport));
            }
        }

        // Fallback to WebSocket
        if capabilities.websocket {
            if let Ok(transport) = websocket::WebSocketConnection::new(config.clone()).await {
                return Ok(Box::new(transport));
            }
        }

        // Final fallback to SSE
        if capabilities.sse {
            if let Ok(transport) = sse::SseConnection::new(config).await {
                return Ok(Box::new(transport));
            }
        }

        Err(TransportError::NotSupported("No suitable transport available".to_string()))
    }

    /// Create a specific transport type
    pub async fn create_websocket(
        config: TransportConfig,
    ) -> Result<websocket::WebSocketConnection, TransportError> {
        websocket::WebSocketConnection::new(config).await
    }

    pub async fn create_webtransport(
        config: TransportConfig,
    ) -> Result<webtransport::WebTransportConnection, TransportError> {
        webtransport::WebTransportConnection::new(config).await
    }

    pub async fn create_sse(config: TransportConfig) -> Result<sse::SseConnection, TransportError> {
        sse::SseConnection::new(config).await
    }
}

impl From<tokio::sync::mpsc::error::SendError<crate::transport::Message>> for TransportError {
    fn from(_err: tokio::sync::mpsc::error::SendError<crate::transport::Message>) -> Self {
        TransportError::ConnectionFailed("Channel send failed".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transport_capabilities_detection() {
        let caps = TransportCapabilities::detect();

        #[cfg(target_arch = "wasm32")]
        {
            assert!(caps.websocket);
            assert!(caps.sse);
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            assert!(caps.websocket);
            assert!(caps.sse);
            assert!(caps.compression);
        }
    }

    #[test]
    fn test_message_creation() {
        let msg = Message {
            data: b"hello".to_vec(),
            message_type: MessageType::Text,
        };

        assert_eq!(msg.data, b"hello");
        assert_eq!(msg.message_type, MessageType::Text);
    }

    #[test]
    fn test_transport_config_default() {
        let config = TransportConfig::default();
        assert_eq!(config.timeout, std::time::Duration::from_secs(30));
        assert_eq!(config.reconnect_delay, std::time::Duration::from_secs(1));
        assert_eq!(config.max_reconnect_attempts, Some(5));
    }
}
