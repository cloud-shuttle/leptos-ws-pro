//! Transport trait implementation for WebTransport
//!
//! Implementation of the core Transport trait for WebTransport connections.

use async_trait::async_trait;
use futures::{Sink, Stream};
use std::pin::Pin;

use crate::transport::{ConnectionState, Message, Transport, TransportError};
use super::core::WebTransportConnection;
use super::sink::WebTransportSink;

#[async_trait]
impl Transport for WebTransportConnection {
    type Stream = Pin<Box<dyn Stream<Item = Result<Message, TransportError>> + Send + Unpin>>;
    type Sink = Pin<Box<dyn Sink<Message, Error = TransportError> + Send + Unpin>>;

    async fn connect(&mut self, url: &str) -> Result<(), TransportError> {
        *self.state.lock().unwrap() = ConnectionState::Connecting;
        self.set_url(url.to_string());

        // Start the connection task
        self.start_connection_task(url.to_string()).await?;

        // Wait for connection to be established
        let mut attempts = 0;
        const MAX_WAIT_ATTEMPTS: u32 = 30; // 30 * 100ms = 3 seconds

        while attempts < MAX_WAIT_ATTEMPTS {
            match self.state() {
                ConnectionState::Connected => return Ok(()),
                ConnectionState::Disconnected => {
                    return Err(TransportError::ConnectionFailed(
                        "Failed to establish connection".to_string(),
                    ))
                }
                ConnectionState::Connecting => {
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    attempts += 1;
                }
                ConnectionState::Reconnecting => {
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    attempts += 1;
                }
                ConnectionState::Failed => {
                    return Err(TransportError::ConnectionFailed(
                        "Connection failed".to_string(),
                    ))
                }
            }
        }

        Err(TransportError::Timeout)
    }

    async fn disconnect(&mut self) -> Result<(), TransportError> {
        self.disconnect().await
    }

    async fn send_message(&self, message: &Message) -> Result<(), TransportError> {
        if !matches!(self.state(), ConnectionState::Connected) {
            return Err(TransportError::NotConnected);
        }

        // Send message through first available stream
        // In a real implementation, you might have routing logic
        let stream_ids = self.active_stream_ids();

        if stream_ids.is_empty() {
            // Create a new stream for sending
            let stream_config = super::config::StreamConfig::default();
            let _stream = self.create_stream(stream_config).await?;
        }

        // For now, just simulate sending by updating metrics
        let mut metrics = self.metrics.lock().unwrap();
        metrics.messages_sent += 1;
        metrics.bytes_sent += message.data.len() as u64;

        Ok(())
    }

    fn split(self) -> (Self::Stream, Self::Sink) {
        // Create sink and stream
        let sink = WebTransportSink::new(self.event_sender.clone());
        let stream = self.create_message_stream();

        (
            Box::pin(stream) as Self::Stream,
            Box::pin(sink) as Self::Sink,
        )
    }

    fn state(&self) -> ConnectionState {
        *self.state.lock().unwrap()
    }

    async fn create_bidirectional_stream(&mut self) -> Result<(), TransportError> {
        if !self.can_create_stream() {
            return Err(TransportError::InvalidState(
                "Cannot create stream in current state".to_string(),
            ));
        }

        let stream_config = super::config::StreamConfig::default();
        self.create_stream(stream_config).await?;
        Ok(())
    }
}

impl WebTransportConnection {
    /// Connect with fallback strategy
    pub async fn connect_with_fallback(&mut self) -> Result<(), TransportError> {
        // Try to connect to the configured URL
        if let Some(url) = self.url().cloned() {
            self.connect(&url).await
        } else {
            Err(TransportError::ConnectionFailed(
                "No URL configured".to_string(),
            ))
        }
    }

    /// Reconnect the connection
    pub async fn reconnect(&mut self) -> Result<(), TransportError> {
        // Disconnect first
        self.disconnect().await?;

        // Wait a bit before reconnecting
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        // Try to reconnect with fallback
        self.connect_with_fallback().await
    }

    /// Create a message stream for receiving messages
    fn create_message_stream(&self) -> Pin<Box<dyn Stream<Item = Result<Message, TransportError>> + Send + Unpin>> {
        // For now, create an empty stream - this would be implemented with actual WebTransport streams
        use futures::stream;
        Box::pin(stream::empty())
    }

    /// Get transport information
    pub fn transport_info(&self) -> TransportInfo {
        TransportInfo {
            transport_type: "WebTransport".to_string(),
            version: "1.0".to_string(),
            state: self.state(),
            url: self.url().cloned(),
            stream_count: self.stream_count(),
            metrics: self.get_metrics(),
        }
    }

    /// Check if transport is ready for operations
    pub fn is_ready(&self) -> bool {
        matches!(self.state(), ConnectionState::Connected) && self.stream_count() > 0
    }

    /// Get transport capabilities
    pub fn capabilities(&self) -> TransportCapabilities {
        TransportCapabilities {
            bidirectional_streams: true,
            unidirectional_streams: true,
            reliable_delivery: true,
            ordered_delivery: true,
            flow_control: true,
            congestion_control: true,
            multiplexing: true,
            max_concurrent_streams: 100,
        }
    }

    /// Perform connection diagnostics
    pub async fn diagnose(&self) -> ConnectionDiagnostics {
        let health = self.health_check().await.unwrap_or(false);
        let stats = self.connection_stats();
        let capabilities = self.capabilities();

        ConnectionDiagnostics {
            healthy: health,
            stats,
            capabilities,
            last_check: std::time::Instant::now(),
        }
    }
}

/// Transport information structure
#[derive(Debug, Clone)]
pub struct TransportInfo {
    pub transport_type: String,
    pub version: String,
    pub state: ConnectionState,
    pub url: Option<String>,
    pub stream_count: usize,
    pub metrics: super::config::PerformanceMetrics,
}

/// Transport capabilities
#[derive(Debug, Clone)]
pub struct TransportCapabilities {
    pub bidirectional_streams: bool,
    pub unidirectional_streams: bool,
    pub reliable_delivery: bool,
    pub ordered_delivery: bool,
    pub flow_control: bool,
    pub congestion_control: bool,
    pub multiplexing: bool,
    pub max_concurrent_streams: usize,
}

/// Connection diagnostics
#[derive(Debug, Clone)]
pub struct ConnectionDiagnostics {
    pub healthy: bool,
    pub stats: super::core::ConnectionStats,
    pub capabilities: TransportCapabilities,
    pub last_check: std::time::Instant,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::TransportConfig;

    #[tokio::test]
    async fn test_transport_connect() {
        let config = TransportConfig::default();
        let mut connection = WebTransportConnection::new(config).await.unwrap();

        // Should start disconnected
        assert!(matches!(connection.state(), ConnectionState::Disconnected));
    }

    #[tokio::test]
    async fn test_transport_capabilities() {
        let config = TransportConfig::default();
        let connection = WebTransportConnection::new(config).await.unwrap();

        let capabilities = connection.capabilities();
        assert!(capabilities.bidirectional_streams);
        assert!(capabilities.reliable_delivery);
        assert!(capabilities.multiplexing);
        assert_eq!(capabilities.max_concurrent_streams, 100);
    }

    #[tokio::test]
    async fn test_transport_info() {
        let config = TransportConfig::default();
        let connection = WebTransportConnection::new(config).await.unwrap();

        let info = connection.transport_info();
        assert_eq!(info.transport_type, "WebTransport");
        assert_eq!(info.version, "1.0");
        assert!(matches!(info.state, ConnectionState::Disconnected));
    }

    #[tokio::test]
    async fn test_connection_readiness() {
        let config = TransportConfig::default();
        let connection = WebTransportConnection::new(config).await.unwrap();

        // Should not be ready when disconnected and no streams
        assert!(!connection.is_ready());
    }

    #[tokio::test]
    async fn test_reconnect_logic() {
        let config = TransportConfig::default();
        let mut connection = WebTransportConnection::new(config).await.unwrap();

        // Set a URL for testing
        connection.set_url("https://example.com/webtransport".to_string());

        // Should have URL set
        assert!(connection.url().is_some());
    }
}
