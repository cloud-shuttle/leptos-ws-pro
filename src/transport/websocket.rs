use crate::transport::{Transport, TransportConfig, TransportError, Message, MessageType, ConnectionState};
use async_trait::async_trait;
use futures::{Sink, Stream, SinkExt, StreamExt};
use std::pin::Pin;
use std::task::{Context, Poll};

/// WebSocket connection implementation
pub struct WebSocketConnection {
    config: TransportConfig,
    state: ConnectionState,
}

impl WebSocketConnection {
    pub async fn new(config: TransportConfig) -> Result<Self, TransportError> {
        Ok(Self {
            config,
            state: ConnectionState::Disconnected,
        })
    }
    
    pub fn capabilities(&self) -> TransportCapabilities {
        TransportCapabilities {
            websocket: true,
            binary: true,
            compression: false,
            multiplexing: false,
        }
    }
}

#[async_trait]
impl Transport for WebSocketConnection {
    type Stream = Pin<Box<dyn Stream<Item = Result<Message, TransportError>> + Send + Unpin>>;
    type Sink = Pin<Box<dyn Sink<Message, Error = TransportError> + Send + Unpin>>;
    
    async fn connect(&mut self, url: &str) -> Result<(), TransportError> {
        self.state = ConnectionState::Connecting;
        // TODO: Implement actual connection logic
        self.state = ConnectionState::Connected;
        Ok(())
    }
    
    async fn disconnect(&mut self) -> Result<(), TransportError> {
        self.state = ConnectionState::Disconnected;
        Ok(())
    }
    
    fn split(self) -> (Self::Stream, Self::Sink) {
        let empty_stream = Box::pin(futures::stream::empty());
        let empty_sink = Box::pin(futures::sink::drain().sink_map_err(|_| TransportError::SendFailed("Empty sink".to_string())));
        (empty_stream, empty_sink)
    }
    
    fn state(&self) -> ConnectionState {
        self.state
    }
}

#[derive(Debug, Clone)]
pub struct TransportCapabilities {
    pub websocket: bool,
    pub binary: bool,
    pub compression: bool,
    pub multiplexing: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_websocket_connection_creation() {
        let config = TransportConfig {
            url: "ws://localhost:8080".to_string(),
            ..Default::default()
        };
        
        let connection = WebSocketConnection::new(config).await;
        assert!(connection.is_ok());
    }
    
    #[test]
    fn test_websocket_capabilities() {
        let config = TransportConfig::default();
        let connection = WebSocketConnection {
            config,
            state: ConnectionState::Disconnected,
        };
        
        let caps = connection.capabilities();
        assert!(caps.websocket);
        assert!(caps.binary);
    }
}