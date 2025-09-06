use crate::transport::{ConnectionState, Message, MessageType, Transport, TransportConfig, TransportError};
use async_trait::async_trait;
use futures::{Sink, SinkExt, Stream, StreamExt};
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use reqwest::Client;
use tokio::sync::mpsc;

/// Server-Sent Events connection implementation
pub struct SseConnection {
    config: TransportConfig,
    state: Arc<Mutex<ConnectionState>>,
    client: Client,
    event_sender: Option<mpsc::UnboundedSender<Message>>,
    event_receiver: Option<mpsc::UnboundedReceiver<Message>>,
}

impl SseConnection {
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

    async fn parse_sse_event(&self, line: &str) -> Option<Message> {
        if line.trim().is_empty() {
            return None;
        }

        // Parse SSE event format: "data: <content>"
        if let Some(data) = line.strip_prefix("data: ") {
            return Some(Message {
                data: data.as_bytes().to_vec(),
                message_type: MessageType::Text,
            });
        }

        // Parse other SSE fields (id, event, retry) - for now just return as text
        Some(Message {
            data: line.as_bytes().to_vec(),
            message_type: MessageType::Text,
        })
    }
}

#[async_trait]
impl Transport for SseConnection {
    type Stream = Pin<Box<dyn Stream<Item = Result<Message, TransportError>> + Send + Unpin>>;
    type Sink = Pin<Box<dyn Sink<Message, Error = TransportError> + Send + Unpin>>;

    async fn connect(&mut self, url: &str) -> Result<(), TransportError> {
        *self.state.lock().unwrap() = ConnectionState::Connecting;

        // Create a GET request with SSE headers
        let response = self.client
            .get(url)
            .header("Accept", "text/event-stream")
            .header("Cache-Control", "no-cache")
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

        // Start reading SSE stream in background
        let sender = self.event_sender.take().unwrap();
        let state = self.state.clone();

        tokio::spawn(async move {
            let mut lines = response.bytes_stream();
            use futures::StreamExt;

            while let Some(chunk) = lines.next().await {
                match chunk {
                    Ok(bytes) => {
                        let text = String::from_utf8_lossy(&bytes);
                        for line in text.lines() {
                            if let Some(message) = Self::parse_sse_event_static(line).await {
                                if sender.send(message).is_err() {
                                    break; // Receiver dropped
                                }
                            }
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
        let stream = Box::pin(SseStream { receiver });

        let sink = Box::pin(
            futures::sink::drain()
                .sink_map_err(|_| TransportError::SendFailed("SSE is receive-only".to_string()))
        );

        (stream, sink)
    }

    fn state(&self) -> ConnectionState {
        *self.state.lock().unwrap()
    }
}

/// Custom stream implementation for SSE events
pub struct SseStream {
    receiver: mpsc::UnboundedReceiver<Message>,
}

impl Stream for SseStream {
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

impl SseConnection {
    async fn parse_sse_event_static(line: &str) -> Option<Message> {
        if line.trim().is_empty() {
            return None;
        }

        // Parse SSE event format: "data: <content>"
        if let Some(data) = line.strip_prefix("data: ") {
            return Some(Message {
                data: data.as_bytes().to_vec(),
                message_type: MessageType::Text,
            });
        }

        // Parse other SSE fields (id, event, retry) - for now just return as text
        Some(Message {
            data: line.as_bytes().to_vec(),
            message_type: MessageType::Text,
        })
    }
}
