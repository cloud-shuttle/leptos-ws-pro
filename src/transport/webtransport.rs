use crate::transport::{Transport, TransportConfig, TransportError, Message, ConnectionState};
use async_trait::async_trait;
use futures::{Sink, Stream, SinkExt};
use std::pin::Pin;

/// WebTransport connection implementation (placeholder)
pub struct WebTransportConnection {
    config: TransportConfig,
    state: ConnectionState,
}

impl WebTransportConnection {
    pub async fn new(config: TransportConfig) -> Result<Self, TransportError> {
        Ok(Self {
            config,
            state: ConnectionState::Disconnected,
        })
    }

    pub fn state(&self) -> ConnectionState {
        self.state
    }
}

#[async_trait]
impl Transport for WebTransportConnection {
    type Stream = Pin<Box<dyn Stream<Item = Result<Message, TransportError>> + Send + Unpin>>;
    type Sink = Pin<Box<dyn Sink<Message, Error = TransportError> + Send + Unpin>>;
    
    async fn connect(&mut self, _url: &str) -> Result<(), TransportError> {
        // WebTransport is not yet implemented
        Err(TransportError::ConnectionFailed("WebTransport not implemented".to_string()))
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

/// Check if WebTransport is supported
pub fn is_supported() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        // TODO: Check if WebTransport is available in the browser
        false
    }
    
    #[cfg(not(target_arch = "wasm32"))]
    {
        false
    }
}