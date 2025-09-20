use crate::transport::{
    ConnectionState, Message, MessageType, Transport, TransportCapabilities, TransportConfig,
    TransportError,
};
use async_trait::async_trait;
use futures::{Sink, SinkExt, Stream, StreamExt};
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};

/// WebSocket connection implementation
#[allow(dead_code)]
pub struct WebSocketConnection {
    config: TransportConfig,
    state: Arc<Mutex<ConnectionState>>,
    stream: Option<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    message_sender: Option<mpsc::UnboundedSender<Message>>,
    message_receiver: Option<mpsc::UnboundedReceiver<Message>>,
    connection_task: Option<tokio::task::JoinHandle<()>>,
    // Send channel for outgoing messages
    send_channel: Option<mpsc::UnboundedSender<Message>>,
}

impl WebSocketConnection {
    pub async fn new(config: TransportConfig) -> Result<Self, TransportError> {
        let (message_sender, message_receiver) = mpsc::unbounded_channel();

        Ok(Self {
            config,
            state: Arc::new(Mutex::new(ConnectionState::Disconnected)),
            stream: None,
            message_sender: Some(message_sender),
            message_receiver: Some(message_receiver),
            connection_task: None,
            send_channel: None,
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
        *self.state.lock().unwrap()
    }

    /// Start background task for handling WebSocket messages
    async fn start_message_handling_task(&mut self) -> Result<(), TransportError> {
        let stream = self.stream.take().ok_or_else(|| {
            TransportError::ConnectionFailed("No WebSocket stream available".to_string())
        })?;

        let message_sender = self.message_sender.take().ok_or_else(|| {
            TransportError::ConnectionFailed("No message sender available".to_string())
        })?;

        let (send_sender, mut send_receiver) = mpsc::unbounded_channel::<Message>();
        self.send_channel = Some(send_sender);

        let state = Arc::clone(&self.state);

        let task = tokio::spawn(async move {
            let (mut write, mut read) = stream.split();

            // Spawn task for handling outgoing messages
            tokio::spawn(async move {
                while let Some(message) = send_receiver.recv().await {
                    let ws_msg = match message.message_type {
                        MessageType::Text => {
                            let text = String::from_utf8_lossy(&message.data);
                            tokio_tungstenite::tungstenite::Message::Text(text.to_string().into())
                        }
                        MessageType::Binary => {
                            tokio_tungstenite::tungstenite::Message::Binary(message.data.into())
                        }
                        MessageType::Ping => {
                            tokio_tungstenite::tungstenite::Message::Ping(message.data.into())
                        }
                        MessageType::Pong => {
                            tokio_tungstenite::tungstenite::Message::Pong(message.data.into())
                        }
                        MessageType::Close => {
                            tokio_tungstenite::tungstenite::Message::Close(None)
                        }
                    };

                    if let Err(e) = write.send(ws_msg).await {
                        eprintln!("Failed to send WebSocket message: {}", e);
                        break;
                    }
                }
            });

            // Handle incoming messages
            while let Some(msg) = read.next().await {
                match msg {
                    Ok(ws_msg) => {
                        let message = match ws_msg {
                            tokio_tungstenite::tungstenite::Message::Text(text) => Message {
                                data: text.as_bytes().to_vec(),
                                message_type: MessageType::Text,
                            },
                            tokio_tungstenite::tungstenite::Message::Binary(data) => Message {
                                data: data.to_vec(),
                                message_type: MessageType::Binary,
                            },
                            tokio_tungstenite::tungstenite::Message::Ping(data) => Message {
                                data: data.to_vec(),
                                message_type: MessageType::Ping,
                            },
                            tokio_tungstenite::tungstenite::Message::Pong(data) => Message {
                                data: data.to_vec(),
                                message_type: MessageType::Pong,
                            },
                            tokio_tungstenite::tungstenite::Message::Close(_) => {
                                *state.lock().unwrap() = ConnectionState::Disconnected;
                                break;
                            }
                            tokio_tungstenite::tungstenite::Message::Frame(_) => continue,
                        };

                        if message_sender.send(message).is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        eprintln!("WebSocket error: {}", e);
                        *state.lock().unwrap() = ConnectionState::Failed;
                        break;
                    }
                }
            }
        });

        self.connection_task = Some(task);
        Ok(())
    }
}

#[async_trait]
impl Transport for WebSocketConnection {
    type Stream = Pin<Box<dyn Stream<Item = Result<Message, TransportError>> + Send + Unpin>>;
    type Sink = Pin<Box<dyn Sink<Message, Error = TransportError> + Send + Unpin>>;

    async fn connect(&mut self, url: &str) -> Result<(), TransportError> {
        *self.state.lock().unwrap() = ConnectionState::Connecting;

        // Connect using tokio-tungstenite
        let result = connect_async(url).await;

        match result {
            Ok((ws_stream, _)) => {
                self.stream = Some(ws_stream);
                *self.state.lock().unwrap() = ConnectionState::Connected;

                // Start background task for handling messages
                self.start_message_handling_task().await?;

                Ok(())
            }
            Err(e) => {
                *self.state.lock().unwrap() = ConnectionState::Disconnected;
                Err(TransportError::ConnectionFailed(e.to_string()))
            }
        }
    }

    async fn disconnect(&mut self) -> Result<(), TransportError> {
        // Cancel the connection task
        if let Some(task) = self.connection_task.take() {
            task.abort();
        }

        if let Some(stream) = &mut self.stream {
            // Close the WebSocket connection
            let _ = stream.close(None).await;
        }
        self.stream = None;
        *self.state.lock().unwrap() = ConnectionState::Disconnected;
        Ok(())
    }

    fn split(self) -> (Self::Stream, Self::Sink) {
        if let Some(stream) = self.stream {
            let (write, read) = stream.split();

            // Convert tungstenite messages to our Message type
            let message_stream = read.map(|result| {
                result
                    .map(|ws_msg| match ws_msg {
                        tokio_tungstenite::tungstenite::Message::Text(text) => Message {
                            data: text.as_bytes().to_vec(),
                            message_type: MessageType::Text,
                        },
                        tokio_tungstenite::tungstenite::Message::Binary(data) => Message {
                            data: data.to_vec(),
                            message_type: MessageType::Binary,
                        },
                        tokio_tungstenite::tungstenite::Message::Ping(data) => Message {
                            data: data.to_vec(),
                            message_type: MessageType::Ping,
                        },
                        tokio_tungstenite::tungstenite::Message::Pong(data) => Message {
                            data: data.to_vec(),
                            message_type: MessageType::Pong,
                        },
                        tokio_tungstenite::tungstenite::Message::Close(_) => Message {
                            data: vec![],
                            message_type: MessageType::Close,
                        },
                        tokio_tungstenite::tungstenite::Message::Frame(_) => Message {
                            data: vec![],
                            message_type: MessageType::Binary,
                        },
                    })
                    .map_err(|e| TransportError::ReceiveFailed(e.to_string()))
            });

            // Create a custom sink that converts our Message type to tungstenite messages
            struct MessageSink {
                inner: futures::stream::SplitSink<
                    WebSocketStream<MaybeTlsStream<TcpStream>>,
                    tokio_tungstenite::tungstenite::Message,
                >,
            }

            impl Sink<Message> for MessageSink {
                type Error = TransportError;

                fn poll_ready(
                    self: Pin<&mut Self>,
                    cx: &mut Context<'_>,
                ) -> Poll<Result<(), Self::Error>> {
                    self.get_mut()
                        .inner
                        .poll_ready_unpin(cx)
                        .map_err(|e| TransportError::SendFailed(e.to_string()))
                }

                fn start_send(self: Pin<&mut Self>, item: Message) -> Result<(), Self::Error> {
                    let ws_msg = match item.message_type {
                        MessageType::Text => {
                            let text = String::from_utf8(item.data)
                                .map_err(|e| TransportError::SendFailed(e.to_string()))?;
                            tokio_tungstenite::tungstenite::Message::Text(text.into())
                        }
                        MessageType::Binary => {
                            tokio_tungstenite::tungstenite::Message::Binary(item.data.into())
                        }
                        MessageType::Ping => {
                            tokio_tungstenite::tungstenite::Message::Ping(item.data.into())
                        }
                        MessageType::Pong => {
                            tokio_tungstenite::tungstenite::Message::Pong(item.data.into())
                        }
                        MessageType::Close => tokio_tungstenite::tungstenite::Message::Close(None),
                    };
                    self.get_mut()
                        .inner
                        .start_send_unpin(ws_msg)
                        .map_err(|e| TransportError::SendFailed(e.to_string()))
                }

                fn poll_flush(
                    self: Pin<&mut Self>,
                    cx: &mut Context<'_>,
                ) -> Poll<Result<(), Self::Error>> {
                    self.get_mut()
                        .inner
                        .poll_flush_unpin(cx)
                        .map_err(|e| TransportError::SendFailed(e.to_string()))
                }

                fn poll_close(
                    self: Pin<&mut Self>,
                    cx: &mut Context<'_>,
                ) -> Poll<Result<(), Self::Error>> {
                    self.get_mut()
                        .inner
                        .poll_close_unpin(cx)
                        .map_err(|e| TransportError::SendFailed(e.to_string()))
                }
            }

            let message_sink = MessageSink { inner: write };

            (Box::pin(message_stream), Box::pin(message_sink))
        } else {
            // Return empty stream and sink if not connected
            let empty_stream = Box::pin(futures::stream::empty());
            let empty_sink = Box::pin(
                futures::sink::drain()
                    .sink_map_err(|_| TransportError::SendFailed("Not connected".to_string())),
            );
            (empty_stream, empty_sink)
        }
    }

    async fn send_message(&self, message: &Message) -> Result<(), TransportError> {
        if self.state() != ConnectionState::Connected {
            return Err(TransportError::ConnectionFailed("Not connected".to_string()));
        }

        // Send via the send channel to background task
        if let Some(sender) = &self.send_channel {
            sender.send(message.clone())
                .map_err(|_| TransportError::SendFailed("Failed to send message to background task".to_string()))
        } else {
            Err(TransportError::SendFailed("No send channel available".to_string()))
        }
    }

    async fn receive_message(&self) -> Result<Message, TransportError> {
        // The receive_message method can't borrow mutably from &self
        // This is a limitation of the current Transport trait design
        // Users should use the split() method to get a stream for receiving messages
        Err(TransportError::NotSupported("Use split() to get a stream for receiving messages".to_string()))
    }

    async fn create_bidirectional_stream(&mut self) -> Result<(), TransportError> {
        // WebSocket is inherently bidirectional, so this is a no-op
        Ok(())
    }

    fn state(&self) -> ConnectionState {
        *self.state.lock().unwrap()
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
            state: Arc::new(Mutex::new(ConnectionState::Disconnected)),
            stream: None,
            message_sender: None,
            message_receiver: None,
            connection_task: None,
            send_channel: None,
        };

        let caps = connection.capabilities();
        assert!(caps.websocket);
        assert!(caps.binary);
    }
}
