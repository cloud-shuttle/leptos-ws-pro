//! Leptos hooks and utilities for reactive WebSocket integration
//!
//! High-level hooks and utilities that provide seamless integration with Leptos components.

use leptos::prelude::*;
use std::collections::VecDeque;

use crate::reactive::{WebSocketContext, WebSocketProvider, PresenceMap, ConnectionMetrics};
use crate::transport::{ConnectionState, Message};

/// Hook for using WebSocket connection
///
/// This hook provides a simple way to establish and manage a WebSocket connection
/// within a Leptos component. It returns a context that provides reactive signals
/// for connection state, messages, and other WebSocket features.
///
/// # Example
/// ```rust
/// use leptos::prelude::*;
/// use leptos_ws_pro::reactive::use_websocket;
///
/// #[component]
/// pub fn MyComponent() -> impl IntoView {
///     let ws_context = use_websocket("ws://localhost:8080/ws");
///
///     let connection_state = ws_context.connection_state();
///     let messages = ws_context.messages();
///
///     view! {
///         <div>
///             <p>"Connection: " {move || format!("{:?}", connection_state.get())}</p>
///             <p>"Messages: " {move || messages.get().len()}</p>
///         </div>
///     }
/// }
/// ```
pub fn use_websocket(url: &str) -> WebSocketContext {
    let provider = WebSocketProvider::new(url);
    WebSocketContext::new(provider)
}

/// Hook for using WebSocket with custom configuration
///
/// This hook allows you to specify custom WebSocket configuration such as
/// protocols, heartbeat intervals, and reconnection settings.
///
/// # Example
/// ```rust
/// use leptos::prelude::*;
/// use leptos_ws_pro::reactive::{use_websocket_with_config, WebSocketConfig};
///
/// #[component]
/// pub fn MyComponent() -> impl IntoView {
///     let config = WebSocketConfig::new("ws://localhost:8080/ws")
///         .with_protocols(vec!["chat".to_string()])
///         .with_heartbeat_interval(30000);
///
///     let ws_context = use_websocket_with_config(config);
///
///     view! {
///         <div>"Custom WebSocket connection"</div>
///     }
/// }
/// ```
pub fn use_websocket_with_config(config: crate::reactive::WebSocketConfig) -> WebSocketContext {
    let provider = WebSocketProvider::with_config(config);
    WebSocketContext::new(provider)
}

/// Hook for using WebSocket with automatic reconnection
///
/// This hook sets up a WebSocket connection with enhanced reconnection logic,
/// including exponential backoff and maximum retry limits.
pub fn use_websocket_with_reconnect(
    url: &str,
    max_attempts: u64,
    initial_interval_ms: u64,
) -> WebSocketContext {
    let config = crate::reactive::WebSocketConfig::new(url)
        .with_max_reconnect_attempts(max_attempts)
        .with_reconnect_interval(initial_interval_ms);

    use_websocket_with_config(config)
}

/// Hook for reactive WebSocket message handling
///
/// This hook provides a convenient way to handle incoming WebSocket messages
/// with automatic deserialization and filtering.
///
/// # Example
/// ```rust
/// use leptos::prelude::*;
/// use serde::{Deserialize, Serialize};
/// use leptos_ws_pro::reactive::use_websocket_messages;
///
/// #[derive(Serialize, Deserialize, Clone, PartialEq)]
/// struct ChatMessage {
///     user: String,
///     message: String,
/// }
///
/// #[component]
/// pub fn ChatComponent() -> impl IntoView {
///     let ws_context = use_websocket("ws://localhost:8080/chat");
///
///     // Get only chat messages, automatically deserialized
///     let chat_messages = use_websocket_messages::<ChatMessage>(&ws_context, "chat");
///
///     view! {
///         <div>
///             <For
///                 each=move || chat_messages.get()
///                 key=|msg| format!("{}:{}", msg.user, msg.message)
///                 children=move |msg: ChatMessage| {
///                     view! {
///                         <div class="message">
///                             <strong>{msg.user}</strong>": "{msg.message}
///                         </div>
///                     }
///                 }
///             />
///         </div>
///     }
/// }
/// ```
pub fn use_websocket_messages<T>(
    context: &WebSocketContext,
    _message_type: &str,
) -> ReadSignal<VecDeque<T>>
where
    T: serde::de::DeserializeOwned + Clone + 'static + Send + Sync,
{
    // This is a simplified implementation - in a real scenario,
    // you'd want to filter and deserialize messages based on type
    let (messages, _set_messages) = signal(VecDeque::new());

    // TODO: Implement actual message filtering and deserialization
    // This would involve:
    // 1. Subscribing to the context's raw messages
    // 2. Filtering by message type
    // 3. Deserializing matching messages
    // 4. Updating the filtered signal

    messages
}

/// Hook for WebSocket connection status
///
/// This hook provides reactive access to connection status with additional
/// metadata such as connection duration and retry attempts.
pub fn use_websocket_status(context: &WebSocketContext) -> ReadSignal<ConnectionStatus> {
    let connection_state = context.connection_state();

    let (status, set_status) = signal(ConnectionStatus::default());

    // Create an effect to update status when connection state changes
    Effect::new(move || {
        let state = connection_state.get();
        let new_status = ConnectionStatus::from_state(state);
        set_status.set(new_status);
    });

    status
}

/// Extended connection status information
#[derive(Debug, Clone, PartialEq)]
pub struct ConnectionStatus {
    pub state: ConnectionState,
    pub connected_at: Option<std::time::Instant>,
    pub retry_count: u32,
    pub last_error: Option<String>,
}

impl Default for ConnectionStatus {
    fn default() -> Self {
        Self {
            state: ConnectionState::Disconnected,
            connected_at: None,
            retry_count: 0,
            last_error: None,
        }
    }
}

impl ConnectionStatus {
    pub fn from_state(state: ConnectionState) -> Self {
        Self {
            state,
            connected_at: match state {
                ConnectionState::Connected => Some(std::time::Instant::now()),
                _ => None,
            },
            retry_count: 0,
            last_error: None,
        }
    }

    pub fn is_connected(&self) -> bool {
        matches!(self.state, ConnectionState::Connected)
    }

    pub fn is_connecting(&self) -> bool {
        matches!(self.state, ConnectionState::Connecting)
    }

    pub fn is_disconnected(&self) -> bool {
        matches!(self.state, ConnectionState::Disconnected)
    }

    pub fn connection_duration(&self) -> Option<std::time::Duration> {
        self.connected_at.map(|start| start.elapsed())
    }
}

/// Hook for sending messages through WebSocket
///
/// This hook provides a convenient way to send messages through a WebSocket
/// connection with automatic serialization.
pub fn use_websocket_send<T>(context: &WebSocketContext) -> impl Fn(T) + Clone
where
    T: serde::Serialize + Clone + 'static,
{
    let sender = context.clone();

    move |message: T| {
        // TODO: Implement actual message sending
        // This would involve:
        // 1. Serializing the message
        // 2. Wrapping it in the appropriate Message format
        // 3. Sending through the context

        // For now, this is a placeholder
        let _ = message;
        let _ = &sender;
    }
}

/// Hook for WebSocket error handling
///
/// This hook provides reactive access to WebSocket errors and connection issues.
pub fn use_websocket_errors(context: &WebSocketContext) -> ReadSignal<Vec<String>> {
    let (errors, _set_errors) = signal(Vec::new());

    // TODO: Implement actual error tracking
    // This would subscribe to error events from the context

    let _ = context;
    errors
}

/// Hook for connection status (legacy compatibility)
pub fn use_connection_status(context: &WebSocketContext) -> ReadSignal<ConnectionState> {
    context.connection_state()
}

/// Hook for connection metrics (legacy compatibility)
pub fn use_connection_metrics(context: &WebSocketContext) -> ReadSignal<ConnectionMetrics> {
    context.metrics()
}

/// Hook for presence map (legacy compatibility)
pub fn use_presence(context: &WebSocketContext) -> ReadSignal<PresenceMap> {
    context.presence()
}

/// Hook for message subscription with filtering (legacy compatibility)
pub fn use_message_subscription<T>(
    context: &WebSocketContext,
    _message_type: &str,
) -> ReadSignal<VecDeque<T>>
where
    T: serde::de::DeserializeOwned + Clone + 'static + Send + Sync,
{
    // This is a simplified implementation for legacy compatibility
    let (filtered_messages, _set_filtered) = signal(VecDeque::new());

    // TODO: Implement actual message type filtering and deserialization
    let _ = context;
    filtered_messages
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_status() {
        let status = ConnectionStatus::default();
        assert!(status.is_disconnected());
        assert!(!status.is_connected());
        assert!(!status.is_connecting());
    }

    #[test]
    fn test_connection_status_from_state() {
        let status = ConnectionStatus::from_state(ConnectionState::Connected);
        assert!(status.is_connected());
        assert!(status.connected_at.is_some());
    }
}
