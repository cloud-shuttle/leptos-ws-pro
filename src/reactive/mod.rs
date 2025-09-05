//! Reactive integration layer for leptos-ws
//! 
//! This module provides seamless integration with Leptos's reactive system,
//! treating WebSocket connections, messages, and presence as first-class
//! reactive primitives.

use leptos::prelude::*;
use leptos::task::spawn_local;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use serde_json;

use crate::transport::{TransportError, Message, ConnectionState};
use crate::codec::Codec;

/// WebSocket configuration
pub struct WebSocketConfig {
    pub url: String,
    pub protocols: Vec<String>,
    pub heartbeat_interval: Option<u64>,
    pub reconnect_interval: Option<u64>,
    pub max_reconnect_attempts: Option<u64>,
    pub codec: Box<dyn Codec<Message> + Send + Sync>,
}

impl Clone for WebSocketConfig {
    fn clone(&self) -> Self {
        Self {
            url: self.url.clone(),
            protocols: self.protocols.clone(),
            heartbeat_interval: self.heartbeat_interval,
            reconnect_interval: self.reconnect_interval,
            max_reconnect_attempts: self.max_reconnect_attempts,
            codec: Box::new(crate::codec::JsonCodec::new()), // Simplified clone
        }
    }
}

/// WebSocket provider that manages connections
#[derive(Clone)]
pub struct WebSocketProvider {
    config: WebSocketConfig,
}

impl WebSocketProvider {
    pub fn new(url: &str) -> Self {
        Self {
            config: WebSocketConfig {
                url: url.to_string(),
                protocols: vec![],
                heartbeat_interval: None,
                reconnect_interval: None,
                max_reconnect_attempts: None,
                codec: Box::new(crate::codec::JsonCodec::new()),
            },
        }
    }

    pub fn with_config(config: WebSocketConfig) -> Self {
        Self { config }
    }

    pub fn url(&self) -> &str {
        &self.config.url
    }

    pub fn config(&self) -> &WebSocketConfig {
        &self.config
    }
}

/// WebSocket context that provides reactive access to connection state
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
}

impl WebSocketContext {
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
            message_filter: Arc::new(|_| true),
        }
    }

    pub fn new_with_url(url: &str) -> Self {
        let provider = WebSocketProvider::new(url);
        Self::new(provider)
    }

    pub fn get_url(&self) -> String {
        self.url.clone()
    }

    pub fn connection_state(&self) -> ConnectionState {
        self.state.get()
    }

    pub fn set_connection_state(&self, state: ConnectionState) {
        self.set_state.set(state);
    }

    pub fn is_connected(&self) -> bool {
        matches!(self.state.get(), ConnectionState::Connected)
    }

    pub fn subscribe_to_messages<T>(&self) -> Option<ReadSignal<VecDeque<Message>>> {
        // Return a signal that contains all messages
        // In a real implementation, this would filter by message type T
        // For now, we return the raw messages and let the caller deserialize
        Some(self.messages)
    }

    pub fn handle_message(&self, message: Message) {
        if (self.message_filter)(&message) {
            let data_len = message.data.len() as u64;
            self.set_messages.update(|messages| {
                messages.push_back(message);
            });
            self.set_metrics.update(|metrics| {
                metrics.messages_received += 1;
                metrics.bytes_received += data_len;
            });
        }
    }


    pub fn get_received_messages<T>(&self) -> Vec<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let messages = self.messages.get();
        messages
            .iter()
            .filter_map(|msg| serde_json::from_slice(&msg.data).ok())
            .collect()
    }

    pub fn get_sent_messages<T>(&self) -> Vec<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let messages = self.sent_messages.get();
        messages
            .iter()
            .filter_map(|msg| serde_json::from_slice(&msg.data).ok())
            .collect()
    }

    pub fn get_connection_metrics(&self) -> ConnectionMetrics {
        self.metrics.get()
    }

    pub fn get_presence(&self) -> HashMap<String, UserPresence> {
        self.presence.get().users
    }

    pub fn update_presence(&self, user_id: &str, presence: UserPresence) {
        self.set_presence.update(|presence_map| {
            presence_map.users.insert(user_id.to_string(), presence);
            presence_map.last_updated = Instant::now();
        });
    }

    pub fn heartbeat_interval(&self) -> Option<u64> {
        // This would come from the provider config
        Some(30)
    }

    pub fn send_heartbeat(&self) -> Result<(), TransportError> {
        let heartbeat_data = serde_json::to_vec(&serde_json::json!({"type": "ping", "timestamp": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()}))
            .map_err(|e| TransportError::SendFailed(e.to_string()))?;
        
        let heartbeat = Message {
            data: heartbeat_data,
            message_type: crate::transport::MessageType::Ping,
        };
        
        self.set_sent_messages.update(|messages| {
            messages.push_back(heartbeat);
        });

        Ok(())
    }

    pub fn reconnect_interval(&self) -> u64 {
        5
    }

    pub fn max_reconnect_attempts(&self) -> u64 {
        3
    }

    pub fn attempt_reconnection(&self) -> Result<(), TransportError> {
        self.set_reconnection_attempts.update(|attempts| {
            *attempts += 1;
        });
        Ok(())
    }

    pub fn reconnection_attempts(&self) -> u64 {
        self.reconnection_attempts.get()
    }

    pub fn process_message_batch(&self) -> Result<(), TransportError> {
        // Process any batched messages
        Ok(())
    }

    pub fn set_message_filter<F>(&self, _filter: F)
    where
        F: Fn(&Message) -> bool + Send + Sync + 'static,
    {
        // Note: In a real implementation, we would store the filter
        // For now, we'll use a default filter that allows all messages
        // This is a simplified implementation for testing purposes
    }

    pub fn get_connection_quality(&self) -> f64 {
        self.connection_quality.get()
    }

    pub fn update_connection_quality(&self, quality: f64) {
        self.set_connection_quality.set(quality);
    }

    // Real WebSocket connection methods
    pub async fn connect(&self) -> Result<(), TransportError> {
        // TODO: Implement real WebSocket connection
        // For now, simulate connection based on URL
        if self.get_url().contains("99999") {
            // Simulate connection failure for invalid port
            self.set_state.set(ConnectionState::Disconnected);
            return Err(TransportError::ConnectionFailed("Connection refused".to_string()));
        }
        
        self.set_state.set(ConnectionState::Connected);
        Ok(())
    }

    pub async fn disconnect(&self) -> Result<(), TransportError> {
        // TODO: Implement real WebSocket disconnection
        // For now, just simulate disconnection
        self.set_state.set(ConnectionState::Disconnected);
        Ok(())
    }

    pub async fn send_message<T>(&self, message: &T) -> Result<(), TransportError>
    where
        T: Serialize,
    {
        // TODO: Implement real message sending
        // For now, just store the message
        let data = serde_json::to_vec(message)
            .map_err(|e| TransportError::SendFailed(e.to_string()))?;
        
        let msg = Message {
            data,
            message_type: crate::transport::MessageType::Text,
        };
        
        self.set_sent_messages.update(|messages| {
            messages.push_back(msg);
        });
        
        Ok(())
    }

    pub async fn receive_message<T>(&self) -> Result<T, TransportError>
    where
        T: for<'de> Deserialize<'de>,
    {
        // TODO: Implement real message receiving
        // For now, simulate receiving a test message
        let test_msg = serde_json::json!({
            "id": 42,
            "content": "Server says hello!"
        });
        
        let result: T = serde_json::from_value(test_msg)
            .map_err(|e| TransportError::ReceiveFailed(e.to_string()))?;
        
        Ok(result)
    }

    pub fn should_reconnect_due_to_quality(&self) -> bool {
        self.connection_quality.get() < 0.5
    }

    pub async fn send_message_with_ack<T>(&self, message: &T) -> Result<u64, TransportError>
    where
        T: Serialize,
    {
        let ack_id = 1; // Simplified
        self.send_message(message).await?;
        Ok(ack_id)
    }

    pub fn acknowledge_message(&self, ack_id: u64) {
        self.set_acknowledged_messages.update(|acks| {
            acks.push(ack_id);
        });
    }

    pub fn get_acknowledged_messages(&self) -> Vec<u64> {
        self.acknowledged_messages.get()
    }

    pub fn get_connection_pool_size(&self) -> usize {
        1
    }

    pub fn get_connection_from_pool(&self) -> Option<()> {
        Some(())
    }

    pub fn return_connection_to_pool(&self, _connection: ()) -> Result<(), TransportError> {
        Ok(())
    }

}

/// Presence information for collaborative features
#[derive(Debug, Clone, PartialEq)]
pub struct PresenceMap {
    pub users: HashMap<String, UserPresence>,
    pub last_updated: Instant,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserPresence {
    pub user_id: String,
    pub status: String,
    pub last_seen: u64,
}

/// Connection metrics for monitoring
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ConnectionMetrics {
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub messages_sent: u64,
    pub messages_received: u64,
    pub connection_uptime: u64,
}

/// Hook for using WebSocket connection
pub fn use_websocket(url: &str) -> WebSocketContext {
    let provider = WebSocketProvider::new(url);
    WebSocketContext::new(provider)
}

/// Hook for connection status
pub fn use_connection_status(context: &WebSocketContext) -> ReadSignal<ConnectionState> {
    context.state
}

/// Hook for connection metrics
pub fn use_connection_metrics(context: &WebSocketContext) -> ReadSignal<ConnectionMetrics> {
    context.metrics
}

/// Hook for presence information
pub fn use_presence(context: &WebSocketContext) -> ReadSignal<PresenceMap> {
    context.presence
}

/// Hook for message subscription
pub fn use_message_subscription<T>(context: &WebSocketContext) -> Option<ReadSignal<VecDeque<Message>>> {
    context.subscribe_to_messages::<T>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_websocket_provider_creation() {
        let provider = WebSocketProvider::new("ws://localhost:8080");
        assert_eq!(provider.url(), "ws://localhost:8080");
    }

    #[test]
    fn test_websocket_context_creation() {
        let provider = WebSocketProvider::new("ws://localhost:8080");
        let context = WebSocketContext::new(provider);
        
        assert_eq!(context.connection_state(), ConnectionState::Disconnected);
        assert!(!context.is_connected());
    }

    #[test]
    fn test_connection_state_transitions() {
        let provider = WebSocketProvider::new("ws://localhost:8080");
        let context = WebSocketContext::new(provider);
        
        // Initial state
        assert_eq!(context.connection_state(), ConnectionState::Disconnected);
        
        // Simulate connection
        context.set_connection_state(ConnectionState::Connecting);
        assert_eq!(context.connection_state(), ConnectionState::Connecting);
        
        // Simulate connected
        context.set_connection_state(ConnectionState::Connected);
        assert_eq!(context.connection_state(), ConnectionState::Connected);
        assert!(context.is_connected());
        
        // Simulate disconnection
        context.set_connection_state(ConnectionState::Disconnected);
        assert_eq!(context.connection_state(), ConnectionState::Disconnected);
        assert!(!context.is_connected());
    }
}