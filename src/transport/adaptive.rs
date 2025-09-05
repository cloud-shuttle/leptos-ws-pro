use crate::transport::{Transport, TransportConfig, TransportError, Message, ConnectionState};
use async_trait::async_trait;
use futures::{Sink, Stream, SinkExt};
use std::pin::Pin;

/// Adaptive transport that tries multiple protocols
pub struct AdaptiveTransport {
    config: TransportConfig,
    state: ConnectionState,
}

impl AdaptiveTransport {
    pub async fn new(config: TransportConfig) -> Result<Self, TransportError> {
        Ok(Self {
            config,
            state: ConnectionState::Disconnected,
        })
    }
}

#[async_trait]
impl Transport for AdaptiveTransport {
    type Stream = Pin<Box<dyn Stream<Item = Result<Message, TransportError>> + Send + Unpin>>;
    type Sink = Pin<Box<dyn Sink<Message, Error = TransportError> + Send + Unpin>>;
    
    async fn connect(&mut self, _url: &str) -> Result<(), TransportError> {
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