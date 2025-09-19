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
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};

/// WebSocket connection implementation
#[allow(dead_code)]
pub struct WebSocketConnection {
    config: TransportConfig,
    state: Arc<Mutex<ConnectionState>>,
    stream: Option<WebSocketStream<MaybeTlsStream<TcpStream>>>,
}

impl WebSocketConnection {
    pub async fn new(config: TransportConfig) -> Result<Self, TransportError> {
        Ok(Self {
            config,
            state: Arc::new(Mutex::new(ConnectionState::Disconnected)),
            stream: None,
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
                Ok(())
            }
            Err(e) => {
                *self.state.lock().unwrap() = ConnectionState::Disconnected;
                Err(TransportError::ConnectionFailed(e.to_string()))
            }
        }
    }

    async fn disconnect(&mut self) -> Result<(), TransportError> {
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
        if let Some(stream) = &self.stream {
            let ws_msg = match message.message_type {
                MessageType::Text => {
                    let text = String::from_utf8(message.data.clone())
                        .map_err(|e| TransportError::SendFailed(e.to_string()))?;
                    tokio_tungstenite::tungstenite::Message::Text(text.into())
                }
                MessageType::Binary => {
                    tokio_tungstenite::tungstenite::Message::Binary(message.data.clone().into())
                }
                MessageType::Ping => {
                    tokio_tungstenite::tungstenite::Message::Ping(message.data.clone().into())
                }
                MessageType::Pong => {
                    tokio_tungstenite::tungstenite::Message::Pong(message.data.clone().into())
                }
                MessageType::Close => {
                    tokio_tungstenite::tungstenite::Message::Close(None)
                }
            };

            // We need to use a different approach since we can't borrow mutably
            // For now, return an error indicating this needs to be implemented differently
            Err(TransportError::NotSupported("send_message requires mutable access to stream".to_string()))
        } else {
            Err(TransportError::ConnectionFailed("Not connected".to_string()))
        }
    }

    async fn receive_message(&self) -> Result<Message, TransportError> {
        // We need to use a different approach since we can't borrow mutably
        // For now, return an error indicating this needs to be implemented differently
        Err(TransportError::NotSupported("receive_message requires mutable access to stream".to_string()))
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
        };

        let caps = connection.capabilities();
        assert!(caps.websocket);
        assert!(caps.binary);
    }
}
