//! Reactive integration layer for leptos-ws
//!
//! This module provides seamless integration with Leptos's reactive system,
//! treating WebSocket connections, messages, and presence as first-class
//! reactive primitives.

// Module declarations
pub mod config;
pub mod websocket;
pub mod presence;
pub mod hooks;

// Re-export main types for convenience
pub use config::{WebSocketConfig, WebSocketProvider};
pub use websocket::WebSocketContext;
pub use presence::{PresenceMap, UserPresence, ConnectionMetrics};
pub use hooks::{
    use_websocket,
    use_websocket_with_config,
    use_websocket_with_reconnect,
    use_websocket_messages,
    use_websocket_status,
    use_websocket_send,
    use_websocket_errors,
    use_connection_status,
    use_connection_metrics,
    use_presence,
    use_message_subscription,
    ConnectionStatus,
};

// Legacy compatibility - re-export everything that was previously in mod.rs
// This ensures existing code doesn't break while we maintain the new modular structure

/// Legacy compatibility function - use hooks::use_websocket instead
pub fn provide_websocket(url: &str) -> WebSocketContext {
    hooks::use_websocket(url)
}

// Additional utility functions that might have been in the original large file
// but didn't fit cleanly into the new modules

/// Create a WebSocket context with custom retry logic
pub fn create_websocket_with_retry(
    url: &str,
    max_attempts: u64,
    base_delay_ms: u64
) -> WebSocketContext {
    let config = WebSocketConfig::new(url)
        .with_max_reconnect_attempts(max_attempts)
        .with_reconnect_interval(base_delay_ms);

    let provider = WebSocketProvider::with_config(config);
    WebSocketContext::new(provider)
}

/// Create a WebSocket context for real-time collaboration
pub fn create_collaborative_websocket(url: &str, user_id: &str) -> WebSocketContext {
    let config = WebSocketConfig::new(url)
        .with_protocols(vec!["collaboration".to_string()])
        .with_heartbeat_interval(15000); // 15 seconds for collaboration

    let provider = WebSocketProvider::with_config(config);
    let mut context = WebSocketContext::new(provider);

    // Set up collaboration-specific message filtering
    let user_id = user_id.to_string();
    context.set_message_filter(move |message| {
        // This is a simplified filter - in reality you'd parse the message
        // and check if it's relevant for this user
        let _ = &user_id;
        let _ = message;
        true
    });

    context
}

/// Connection pool for managing multiple WebSocket connections
pub struct WebSocketPool {
    connections: std::collections::HashMap<String, WebSocketContext>,
}

impl WebSocketPool {
    pub fn new() -> Self {
        Self {
            connections: std::collections::HashMap::new(),
        }
    }

    pub fn add_connection(&mut self, name: String, context: WebSocketContext) {
        self.connections.insert(name, context);
    }

    pub fn get_connection(&self, name: &str) -> Option<&WebSocketContext> {
        self.connections.get(name)
    }

    pub fn remove_connection(&mut self, name: &str) -> Option<WebSocketContext> {
        self.connections.remove(name)
    }

    pub async fn connect_all(&self) -> Vec<Result<(), crate::transport::TransportError>> {
        let mut results = Vec::new();

        for (_, context) in &self.connections {
            results.push(context.connect().await);
        }

        results
    }

    pub async fn disconnect_all(&self) -> Vec<Result<(), crate::transport::TransportError>> {
        let mut results = Vec::new();

        for (_, context) in &self.connections {
            results.push(context.disconnect().await);
        }

        results
    }

    pub fn connection_names(&self) -> Vec<String> {
        self.connections.keys().cloned().collect()
    }

    pub fn connection_count(&self) -> usize {
        self.connections.len()
    }
}

impl Default for WebSocketPool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provide_websocket_legacy_compat() {
        let context = provide_websocket("ws://test");
        assert_eq!(context.url(), "ws://test");
        assert!(context.is_disconnected());
    }

    #[test]
    fn test_websocket_pool() {
        let mut pool = WebSocketPool::new();
        assert_eq!(pool.connection_count(), 0);

        let context = hooks::use_websocket("ws://test");
        pool.add_connection("test".to_string(), context);

        assert_eq!(pool.connection_count(), 1);
        assert!(pool.get_connection("test").is_some());
        assert_eq!(pool.connection_names(), vec!["test"]);
    }

    #[test]
    fn test_create_websocket_with_retry() {
        let context = create_websocket_with_retry("ws://test", 10, 500);
        assert_eq!(context.url(), "ws://test");
    }

    #[test]
    fn test_create_collaborative_websocket() {
        let context = create_collaborative_websocket("ws://collab", "user123");
        assert_eq!(context.url(), "ws://collab");
    }
}
