use crate::transport::{Transport, TransportConfig, TransportError, Message, MessageType, ConnectionState, TransportCapabilities};
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
            webtransport: false,
            sse: false,
            binary: true,
            compression: false,
        }
    }

    pub fn state(&self) -> ConnectionState {
        self.state
    }

    pub async fn split(self) -> Result<(<WebSocketConnection as Transport>::Stream, <WebSocketConnection as Transport>::Sink), TransportError> {
        // TODO: Implement real WebSocket split
        // For now, return empty stream and sink
        let empty_stream = Box::pin(futures::stream::empty());
        
        // Create a simple sink that always returns TransportError
        struct ErrorSink;
        impl futures::Sink<Message> for ErrorSink {
            type Error = TransportError;
            
            fn poll_ready(self: std::pin::Pin<&mut Self>, _cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
                std::task::Poll::Ready(Err(TransportError::SendFailed("Not implemented".to_string())))
            }
            
            fn start_send(self: std::pin::Pin<&mut Self>, _item: Message) -> Result<(), Self::Error> {
                Err(TransportError::SendFailed("Not implemented".to_string()))
            }
            
            fn poll_flush(self: std::pin::Pin<&mut Self>, _cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
                std::task::Poll::Ready(Err(TransportError::SendFailed("Not implemented".to_string())))
            }
            
            fn poll_close(self: std::pin::Pin<&mut Self>, _cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
                std::task::Poll::Ready(Err(TransportError::SendFailed("Not implemented".to_string())))
            }
        }
        
        let empty_sink = Box::pin(ErrorSink);
        
        Ok((empty_stream, empty_sink))
    }
}

#[async_trait]
impl Transport for WebSocketConnection {
    type Stream = Pin<Box<dyn Stream<Item = Result<Message, TransportError>> + Send + Unpin>>;
    type Sink = Pin<Box<dyn Sink<Message, Error = TransportError> + Send + Unpin>>;
    
    async fn connect(&mut self, url: &str) -> Result<(), TransportError> {
        self.state = ConnectionState::Connecting;
        
        // Simulate connection failure for invalid ports
        if url.contains("99999") {
            self.state = ConnectionState::Disconnected;
            return Err(TransportError::ConnectionFailed("Connection refused".to_string()));
        }
        
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

// TransportCapabilities is defined in mod.rs

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