//! WebSocket configuration and provider types
//!
//! Configuration structures and provider implementations for reactive WebSocket connections.

use crate::codec::Codec;
use crate::transport::Message;
use std::fmt;

/// WebSocket configuration
pub struct WebSocketConfig {
    pub url: String,
    pub protocols: Vec<String>,
    pub heartbeat_interval: Option<u64>,
    pub reconnect_interval: Option<u64>,
    pub max_reconnect_attempts: Option<u64>,
    pub codec: Box<dyn Codec<Message> + Send + Sync>,
}

impl fmt::Debug for WebSocketConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WebSocketConfig")
            .field("url", &self.url)
            .field("protocols", &self.protocols)
            .field("heartbeat_interval", &self.heartbeat_interval)
            .field("reconnect_interval", &self.reconnect_interval)
            .field("max_reconnect_attempts", &self.max_reconnect_attempts)
            .field("codec", &"<dyn Codec<Message>>")
            .finish()
    }
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

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            url: "ws://localhost:8080/ws".to_string(),
            protocols: vec![],
            heartbeat_interval: Some(30000), // 30 seconds
            reconnect_interval: Some(1000),  // 1 second
            max_reconnect_attempts: Some(5),
            codec: Box::new(crate::codec::JsonCodec::new()),
        }
    }
}

impl WebSocketConfig {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            ..Default::default()
        }
    }

    pub fn with_protocols(mut self, protocols: Vec<String>) -> Self {
        self.protocols = protocols;
        self
    }

    pub fn with_heartbeat_interval(mut self, interval_ms: u64) -> Self {
        self.heartbeat_interval = Some(interval_ms);
        self
    }

    pub fn with_reconnect_interval(mut self, interval_ms: u64) -> Self {
        self.reconnect_interval = Some(interval_ms);
        self
    }

    pub fn with_max_reconnect_attempts(mut self, max_attempts: u64) -> Self {
        self.max_reconnect_attempts = Some(max_attempts);
        self
    }

    pub fn with_codec(mut self, codec: Box<dyn Codec<Message> + Send + Sync>) -> Self {
        self.codec = codec;
        self
    }
}

/// WebSocket provider that manages connections
#[derive(Clone)]
pub struct WebSocketProvider {
    config: WebSocketConfig,
}

impl WebSocketProvider {
    /// Create a new WebSocket provider
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            config: WebSocketConfig::new(url),
        }
    }

    /// Create a new WebSocket provider with custom configuration
    pub fn with_config(config: WebSocketConfig) -> Self {
        Self { config }
    }

    /// Get the current configuration
    pub fn config(&self) -> &WebSocketConfig {
        &self.config
    }

    /// Update the configuration
    pub fn set_config(&mut self, config: WebSocketConfig) {
        self.config = config;
    }

    /// Create a new provider with updated URL
    pub fn with_url(mut self, url: impl Into<String>) -> Self {
        self.config.url = url.into();
        self
    }

    /// Create a new provider with protocols
    pub fn with_protocols(mut self, protocols: Vec<String>) -> Self {
        self.config.protocols = protocols;
        self
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.config.url.is_empty() {
            return Err("WebSocket URL cannot be empty".to_string());
        }

        if !self.config.url.starts_with("ws://") && !self.config.url.starts_with("wss://") {
            return Err("WebSocket URL must start with 'ws://' or 'wss://'".to_string());
        }

        if let Some(interval) = self.config.heartbeat_interval {
            if interval == 0 {
                return Err("Heartbeat interval must be greater than 0".to_string());
            }
        }

        if let Some(interval) = self.config.reconnect_interval {
            if interval == 0 {
                return Err("Reconnect interval must be greater than 0".to_string());
            }
        }

        if let Some(attempts) = self.config.max_reconnect_attempts {
            if attempts == 0 {
                return Err("Max reconnect attempts must be greater than 0".to_string());
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_creation() {
        let config = WebSocketConfig::new("ws://example.com/ws");
        assert_eq!(config.url, "ws://example.com/ws");
        assert_eq!(config.protocols, Vec::<String>::new());
        assert_eq!(config.heartbeat_interval, Some(30000));
    }

    #[test]
    fn test_provider_validation() {
        let provider = WebSocketProvider::new("ws://example.com/ws");
        assert!(provider.validate().is_ok());

        let provider = WebSocketProvider::new("http://example.com");
        assert!(provider.validate().is_err());

        let provider = WebSocketProvider::new("");
        assert!(provider.validate().is_err());
    }

    #[test]
    fn test_config_builder() {
        let config = WebSocketConfig::new("ws://example.com/ws")
            .with_protocols(vec!["chat".to_string()])
            .with_heartbeat_interval(60000)
            .with_max_reconnect_attempts(10);

        assert_eq!(config.protocols, vec!["chat"]);
        assert_eq!(config.heartbeat_interval, Some(60000));
        assert_eq!(config.max_reconnect_attempts, Some(10));
    }
}
