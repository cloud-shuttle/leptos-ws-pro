use crate::transport::{ConnectionState, Message, Transport, TransportConfig, TransportError};
use async_trait::async_trait;
use futures::{Sink, SinkExt, Stream};
use std::pin::Pin;

/// Server-Sent Events connection implementation (placeholder)
pub struct SseConnection {
    config: TransportConfig,
    state: ConnectionState,
}

impl SseConnection {
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
impl Transport for SseConnection {
    type Stream = Pin<Box<dyn Stream<Item = Result<Message, TransportError>> + Send + Unpin>>;
    type Sink = Pin<Box<dyn Sink<Message, Error = TransportError> + Send + Unpin>>;

    async fn connect(&mut self, _url: &str) -> Result<(), TransportError> {
        // SSE is not yet implemented
        Err(TransportError::ConnectionFailed(
            "SSE not implemented".to_string(),
        ))
    }

    async fn disconnect(&mut self) -> Result<(), TransportError> {
        self.state = ConnectionState::Disconnected;
        Ok(())
    }

    fn split(self) -> (Self::Stream, Self::Sink) {
        let empty_stream = Box::pin(futures::stream::empty());
        let empty_sink = Box::pin(
            futures::sink::drain()
                .sink_map_err(|_| TransportError::SendFailed("Empty sink".to_string())),
        );
        (empty_stream, empty_sink)
    }

    fn state(&self) -> ConnectionState {
        self.state
    }
}
