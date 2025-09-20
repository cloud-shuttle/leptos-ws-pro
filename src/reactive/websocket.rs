//! WebSocket context and core connection management
//!
//! Core reactive WebSocket context that manages connection state, messages, and real-time features.

use futures_util::{SinkExt, StreamExt};
use leptos::prelude::*;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;
use tokio_tungstenite::{connect_async, tungstenite::Message as WsMessage};

use crate::reactive::{WebSocketProvider, PresenceMap, ConnectionMetrics};
use crate::transport::{ConnectionState, Message, TransportError};

/// WebSocket context providing reactive access to connection state and messages
#[derive(Clone)]
pub struct WebSocketContext {
    url: String,
    state: ReadSignal<ConnectionState>,
    set_state: WriteSignal<ConnectionState>,
    pub messages: ReadSignal<VecDeque<Message>>,
    set_messages: WriteSignal<VecDeque<Message>>,
    presence: ReadSignal<PresenceMap>,
    set_presence: WriteSignal<PresenceMap>,
    metrics: ReadSignal<ConnectionMetrics>,
    set_metrics: WriteSignal<ConnectionMetrics>,
    sent_messages: ReadSignal<VecDeque<Message>>,
    set_sent_messages: WriteSignal<VecDeque<Message>>,
    reconnection_attempts: ReadSignal<u64>,
    set_reconnection_attempts: WriteSignal<u64>,
    connection_quality: ReadSignal<f64>,
    set_connection_quality: WriteSignal<f64>,
    acknowledged_messages: ReadSignal<Vec<u64>>,
    set_acknowledged_messages: WriteSignal<Vec<u64>>,
    message_filter: Arc<dyn Fn(&Message) -> bool + Send + Sync>,

    // Real WebSocket connection components
    ws_connection: Arc<
        Mutex<
            Option<
                tokio_tungstenite::WebSocketStream<
                    tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
                >,
            >,
        >,
    >,
    ws_sink: Arc<
        Mutex<
            Option<
                futures_util::stream::SplitSink<
                    tokio_tungstenite::WebSocketStream<
                        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
                    >,
                    WsMessage,
                >,
            >,
        >,
    >,
    ws_stream: Arc<
        Mutex<
            Option<
                futures_util::stream::SplitStream<
                    tokio_tungstenite::WebSocketStream<
                        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
                    >,
                >,
            >,
        >,
    >,
}

impl WebSocketContext {
    /// Create a new WebSocket context from a provider
    pub fn new(provider: WebSocketProvider) -> Self {
        let url = provider.config().url.clone();
        let (state, set_state) = signal(ConnectionState::Disconnected);
        let (messages, set_messages) = signal(VecDeque::new());
        let (presence, set_presence) = signal(PresenceMap {
            users: HashMap::new(),
            last_updated: Instant::now(),
        });
        let (metrics, set_metrics) = signal(ConnectionMetrics::default());
        let (sent_messages, set_sent_messages) = signal(VecDeque::new());
        let (reconnection_attempts, set_reconnection_attempts) = signal(0);
        let (connection_quality, set_connection_quality) = signal(1.0);
        let (acknowledged_messages, set_acknowledged_messages) = signal(Vec::new());

        Self {
            url,
            state,
            set_state,
            messages,
            set_messages,
            presence,
            set_presence,
            metrics,
            set_metrics,
            sent_messages,
            set_sent_messages,
            reconnection_attempts,
            set_reconnection_attempts,
            connection_quality,
            set_connection_quality,
            acknowledged_messages,
            set_acknowledged_messages,
            message_filter: Arc::new(|_| true), // Accept all messages by default
            ws_connection: Arc::new(Mutex::new(None)),
            ws_sink: Arc::new(Mutex::new(None)),
            ws_stream: Arc::new(Mutex::new(None)),
        }
    }

    /// Get the connection state signal
    pub fn connection_state(&self) -> ReadSignal<ConnectionState> {
        self.state
    }

    /// Get the messages signal
    pub fn messages(&self) -> ReadSignal<VecDeque<Message>> {
        self.messages
    }

    /// Get the presence map signal
    pub fn presence(&self) -> ReadSignal<PresenceMap> {
        self.presence
    }

    /// Get connection metrics
    pub fn metrics(&self) -> ReadSignal<ConnectionMetrics> {
        self.metrics
    }

    /// Get sent messages
    pub fn sent_messages(&self) -> ReadSignal<VecDeque<Message>> {
        self.sent_messages
    }

    /// Get reconnection attempts count
    pub fn reconnection_attempts(&self) -> ReadSignal<u64> {
        self.reconnection_attempts
    }

    /// Get connection quality (0.0 to 1.0)
    pub fn connection_quality(&self) -> ReadSignal<f64> {
        self.connection_quality
    }

    /// Get acknowledged messages
    pub fn acknowledged_messages(&self) -> ReadSignal<Vec<u64>> {
        self.acknowledged_messages
    }

    /// Connect to the WebSocket server
    pub async fn connect(&self) -> Result<(), TransportError> {
        if matches!(self.state.get(), ConnectionState::Connected | ConnectionState::Connecting) {
            return Ok(());
        }

        self.set_state.set(ConnectionState::Connecting);

        match connect_async(&self.url).await {
            Ok((ws_stream, _response)) => {
                let (sink, stream) = ws_stream.split();

                // Store connection components
                *self.ws_sink.lock().await = Some(sink);
                *self.ws_stream.lock().await = Some(stream);

                self.set_state.set(ConnectionState::Connected);
                self.set_reconnection_attempts.set(0);

                // Start message processing
                self.start_message_loop().await;

                Ok(())
            }
            Err(e) => {
                self.set_state.set(ConnectionState::Disconnected);
                let attempts = self.reconnection_attempts.get();
                self.set_reconnection_attempts.set(attempts + 1);
                Err(TransportError::ConnectionFailed(e.to_string()))
            }
        }
    }

    /// Disconnect from the WebSocket server
    pub async fn disconnect(&self) -> Result<(), TransportError> {
        self.set_state.set(ConnectionState::Disconnected);

        // Close the sink
        if let Some(mut sink) = self.ws_sink.lock().await.take() {
            let _ = sink.close().await;
        }

        // Clear stream
        *self.ws_stream.lock().await = None;
        *self.ws_connection.lock().await = None;

        Ok(())
    }

    /// Send a message through the WebSocket
    pub async fn send(&self, message: Message) -> Result<(), TransportError> {
        if !matches!(self.state.get(), ConnectionState::Connected) {
            return Err(TransportError::ConnectionClosed);
        }

        let ws_message = self.message_to_ws_message(message.clone())?;

        if let Some(ref mut sink) = *self.ws_sink.lock().await {
            match sink.send(ws_message).await {
                Ok(_) => {
                    // Add to sent messages
                    let mut sent = self.sent_messages.get();
                    sent.push_back(message);
                    // Keep only last 100 sent messages
                    if sent.len() > 100 {
                        sent.pop_front();
                    }
                    self.set_sent_messages.set(sent);

                    // Update metrics
                    let mut metrics = self.metrics.get();
                    metrics.record_message_sent();
                    self.set_metrics.set(metrics);

                    Ok(())
                }
                Err(e) => Err(TransportError::SendFailed(e.to_string())),
            }
        } else {
            Err(TransportError::ConnectionClosed)
        }
    }

    /// Send a text message
    pub async fn send_text(&self, text: String) -> Result<(), TransportError> {
        let message = Message {
            data: text.into_bytes(),
            message_type: crate::transport::MessageType::Text,
        };
        self.send(message).await
    }

    /// Send a binary message
    pub async fn send_binary(&self, data: Vec<u8>) -> Result<(), TransportError> {
        let message = Message {
            data,
            message_type: crate::transport::MessageType::Binary,
        };
        self.send(message).await
    }

    /// Start the message processing loop
    async fn start_message_loop(&self) {
        let stream_arc = Arc::clone(&self.ws_stream);
        let set_messages = self.set_messages;
        let set_metrics = self.set_metrics;
        let set_state = self.set_state;
        let message_filter = Arc::clone(&self.message_filter);
        let metrics_signal = self.metrics;

        // Spawn a task to handle incoming messages
        tokio::spawn(async move {
            let mut stream_guard = stream_arc.lock().await;
            if let Some(ref mut stream) = *stream_guard {
                while let Some(message_result) = stream.next().await {
                    match message_result {
                        Ok(ws_message) => {
                            if let Ok(message) = Self::ws_message_to_message(ws_message) {
                                // Apply message filter
                                if message_filter(&message) {
                                    // Add to messages
                                    set_messages.update(|messages| {
                                        messages.push_back(message);
                                        // Keep only last 1000 messages
                                        if messages.len() > 1000 {
                                            messages.pop_front();
                                        }
                                    });

                                    // Update metrics
                                    let mut metrics = metrics_signal.get();
                                    metrics.record_message_received();
                                    set_metrics.set(metrics);
                                }
                            }
                        }
                        Err(_) => {
                            // Connection error - set state to disconnected
                            set_state.set(ConnectionState::Disconnected);
                            break;
                        }
                    }
                }
            }
        });
    }

    /// Convert internal Message to WebSocket message
    fn message_to_ws_message(&self, message: Message) -> Result<WsMessage, TransportError> {
        match message.message_type {
            crate::transport::MessageType::Text => {
                let text = String::from_utf8(message.data)
                    .map_err(|e| TransportError::SendFailed(format!("Invalid UTF-8: {}", e)))?;
                Ok(WsMessage::Text(text.into()))
            }
            crate::transport::MessageType::Binary => Ok(WsMessage::Binary(message.data.into())),
            crate::transport::MessageType::Ping => Ok(WsMessage::Ping(message.data.into())),
            crate::transport::MessageType::Pong => Ok(WsMessage::Pong(message.data.into())),
            crate::transport::MessageType::Close => Ok(WsMessage::Close(None)),
        }
    }

    /// Convert WebSocket message to internal Message
    fn ws_message_to_message(ws_message: WsMessage) -> Result<Message, TransportError> {
        match ws_message {
            WsMessage::Text(text) => Ok(Message {
                data: text.to_string().into_bytes(),
                message_type: crate::transport::MessageType::Text,
            }),
            WsMessage::Binary(data) => Ok(Message {
                data: data.to_vec(),
                message_type: crate::transport::MessageType::Binary,
            }),
            WsMessage::Ping(data) => Ok(Message {
                data: data.to_vec(),
                message_type: crate::transport::MessageType::Ping,
            }),
            WsMessage::Pong(data) => Ok(Message {
                data: data.to_vec(),
                message_type: crate::transport::MessageType::Pong,
            }),
            WsMessage::Close(_) => Ok(Message {
                data: vec![],
                message_type: crate::transport::MessageType::Close,
            }),
            _ => Err(TransportError::ReceiveFailed("Unsupported message type".to_string())),
        }
    }

    /// Set a message filter function
    pub fn set_message_filter<F>(&mut self, filter: F)
    where
        F: Fn(&Message) -> bool + Send + Sync + 'static,
    {
        self.message_filter = Arc::new(filter);
    }

    /// Clear all messages
    pub fn clear_messages(&self) {
        self.set_messages.set(VecDeque::new());
    }

    /// Clear sent messages
    pub fn clear_sent_messages(&self) {
        self.set_sent_messages.set(VecDeque::new());
    }

    /// Get the WebSocket URL
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Check if connected
    pub fn is_connected(&self) -> bool {
        matches!(self.state.get(), ConnectionState::Connected)
    }

    /// Check if connecting
    pub fn is_connecting(&self) -> bool {
        matches!(self.state.get(), ConnectionState::Connecting)
    }

    /// Check if disconnected
    pub fn is_disconnected(&self) -> bool {
        matches!(self.state.get(), ConnectionState::Disconnected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reactive::WebSocketConfig;

    #[test]
    fn test_websocket_context_creation() {
        let config = WebSocketConfig::new("ws://localhost:8080/test");
        let provider = WebSocketProvider::with_config(config);
        let context = WebSocketContext::new(provider);

        assert!(context.is_disconnected());
        assert_eq!(context.url(), "ws://localhost:8080/test");
        assert_eq!(context.messages().get().len(), 0);
    }

    #[test]
    fn test_message_conversion() {
        let message = Message {
            data: b"test".to_vec(),
            message_type: crate::transport::MessageType::Text,
        };

        let config = WebSocketConfig::new("ws://test");
        let provider = WebSocketProvider::with_config(config);
        let context = WebSocketContext::new(provider);

        let ws_message = context.message_to_ws_message(message).unwrap();
        match ws_message {
            WsMessage::Text(text) => assert_eq!(text, "test"),
            _ => panic!("Expected text message"),
        }
    }

    #[test]
    fn test_connection_state_signals() {
        let config = WebSocketConfig::new("ws://test");
        let provider = WebSocketProvider::with_config(config);
        let context = WebSocketContext::new(provider);

        let state = context.connection_state();
        assert_eq!(state.get(), ConnectionState::Disconnected);

        let messages = context.messages();
        assert!(messages.get().is_empty());
    }
}
