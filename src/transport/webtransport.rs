use crate::transport::{ConnectionState, Message, MessageType, Transport, TransportConfig, TransportError};
use async_trait::async_trait;
use futures::{Sink, SinkExt, Stream, StreamExt};
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use tokio::sync::mpsc;
use reqwest::Client;

/// WebTransport connection implementation
pub struct WebTransportConnection {
    config: TransportConfig,
    state: Arc<Mutex<ConnectionState>>,
    client: Client,
    event_sender: Option<mpsc::UnboundedSender<Message>>,
    event_receiver: Option<mpsc::UnboundedReceiver<Message>>,
}

impl WebTransportConnection {
    pub async fn new(config: TransportConfig) -> Result<Self, TransportError> {
        let client = Client::new();
        let (event_sender, event_receiver) = mpsc::unbounded_channel();

        Ok(Self {
            config,
            state: Arc::new(Mutex::new(ConnectionState::Disconnected)),
            client,
            event_sender: Some(event_sender),
            event_receiver: Some(event_receiver),
        })
    }

    pub fn state(&self) -> ConnectionState {
        *self.state.lock().unwrap()
    }

    pub async fn create_stream(&self) -> Result<(), TransportError> {
        Err(TransportError::ConnectionFailed(
            "WebTransport not implemented".to_string(),
        ))
    }

    pub async fn create_multiplexed_streams(
        &self,
        _count: usize,
    ) -> Result<Vec<()>, TransportError> {
        Err(TransportError::ConnectionFailed(
            "WebTransport not implemented".to_string(),
        ))
    }

    pub async fn setup_http3_connection(&self) -> Result<(), TransportError> {
        Err(TransportError::ConnectionFailed(
            "WebTransport not implemented".to_string(),
        ))
    }

    pub async fn connect_with_fallback(&mut self) -> Result<(), TransportError> {
        Err(TransportError::ConnectionFailed(
            "WebTransport not implemented".to_string(),
        ))
    }

    pub async fn send_message<T: serde::Serialize>(
        &self,
        _message: &T,
    ) -> Result<(), TransportError> {
        Err(TransportError::SendFailed(
            "WebTransport not implemented".to_string(),
        ))
    }

    pub async fn receive_message<T: for<'de> serde::Deserialize<'de>>(
        &self,
    ) -> Result<T, TransportError> {
        Err(TransportError::ReceiveFailed(
            "WebTransport not implemented".to_string(),
        ))
    }

    pub async fn reconnect(&mut self) -> Result<(), TransportError> {
        Err(TransportError::ConnectionFailed(
            "WebTransport not implemented".to_string(),
        ))
    }

    pub async fn reconnect_with_backoff(&mut self) -> Result<(), TransportError> {
        Err(TransportError::ConnectionFailed(
            "WebTransport not implemented".to_string(),
        ))
    }

    pub fn get_performance_metrics(&self) -> PerformanceMetrics {
        PerformanceMetrics {
            connection_count: 0,
            message_count: 0,
            error_count: 0,
        }
    }

    pub async fn optimize_for_latency(&self) -> Result<(), TransportError> {
        Ok(())
    }

    pub async fn optimize_for_throughput(&self) -> Result<(), TransportError> {
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub connection_count: u64,
    pub message_count: u64,
    pub error_count: u64,
}

#[async_trait]
impl Transport for WebTransportConnection {
    type Stream = Pin<Box<dyn Stream<Item = Result<Message, TransportError>> + Send + Unpin>>;
    type Sink = Pin<Box<dyn Sink<Message, Error = TransportError> + Send + Unpin>>;

    async fn connect(&mut self, url: &str) -> Result<(), TransportError> {
        *self.state.lock().unwrap() = ConnectionState::Connecting;

        // For now, simulate WebTransport with HTTP/2 or HTTP/1.1
        // In a real implementation, this would use HTTP/3 and WebTransport protocol
        let response = self.client
            .get(url)
            .header("Sec-WebTransport-HTTP3-Draft", "draft02")
            .header("Connection", "Upgrade")
            .header("Upgrade", "webtransport")
            .send()
            .await
            .map_err(|e| {
                *self.state.lock().unwrap() = ConnectionState::Disconnected;
                TransportError::ConnectionFailed(format!("Failed to connect: {}", e))
            })?;

        if !response.status().is_success() {
            *self.state.lock().unwrap() = ConnectionState::Disconnected;
            return Err(TransportError::ConnectionFailed(
                format!("HTTP error: {}", response.status())
            ));
        }

        *self.state.lock().unwrap() = ConnectionState::Connected;

        // Start reading WebTransport stream in background
        let sender = self.event_sender.take().unwrap();
        let state = self.state.clone();

        tokio::spawn(async move {
            let mut stream = response.bytes_stream();

            while let Some(chunk) = stream.next().await {
                match chunk {
                    Ok(bytes) => {
                        // Parse WebTransport messages
                        let message = Message {
                            data: bytes.to_vec(),
                            message_type: MessageType::Binary,
                        };

                        if sender.send(message).is_err() {
                            break; // Receiver dropped
                        }
                    }
                    Err(_) => break,
                }
            }

            *state.lock().unwrap() = ConnectionState::Disconnected;
        });

        Ok(())
    }

    async fn disconnect(&mut self) -> Result<(), TransportError> {
        *self.state.lock().unwrap() = ConnectionState::Disconnected;
        self.event_sender = None;
        self.event_receiver = None;
        Ok(())
    }

    fn split(self) -> (Self::Stream, Self::Sink) {
        let receiver = self.event_receiver.unwrap_or_else(|| {
            let (_, recv) = mpsc::unbounded_channel();
            recv
        });

        // Create a custom stream from the receiver
        let stream = Box::pin(WebTransportStream { receiver });

        let sink = Box::pin(
            futures::sink::drain()
                .sink_map_err(|_| TransportError::SendFailed("WebTransport sink not implemented".to_string()))
        );

        (stream, sink)
    }

    fn state(&self) -> ConnectionState {
        *self.state.lock().unwrap()
    }
}

/// Custom stream implementation for WebTransport events
pub struct WebTransportStream {
    receiver: mpsc::UnboundedReceiver<Message>,
}

impl Stream for WebTransportStream {
    type Item = Result<Message, TransportError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.receiver.try_recv() {
            Ok(msg) => Poll::Ready(Some(Ok(msg))),
            Err(mpsc::error::TryRecvError::Empty) => {
                // Register for wakeup when data is available
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Err(mpsc::error::TryRecvError::Disconnected) => Poll::Ready(None),
        }
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
        // For now, return true for testing purposes
        // In a real implementation, this would check for HTTP/3 support
        true
    }
}
