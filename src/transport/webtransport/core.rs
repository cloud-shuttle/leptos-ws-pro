//! WebTransport connection core functionality
//!
//! Core connection management and initialization for WebTransport.

use reqwest::{header, Client};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;

use crate::transport::{ConnectionState, Message, TransportConfig, TransportError};
use super::config::PerformanceMetrics;
use super::stream::AdvancedWebTransportStream;

/// WebTransport connection implementation
pub struct WebTransportConnection {
    pub(super) config: TransportConfig,
    pub(super) state: Arc<Mutex<ConnectionState>>,
    pub(super) client: Client,
    pub(super) event_sender: Option<mpsc::UnboundedSender<Message>>,
    pub(super) event_receiver: Option<mpsc::UnboundedReceiver<Message>>,
    pub(super) streams: Arc<Mutex<HashMap<u32, AdvancedWebTransportStream>>>,
    pub(super) next_stream_id: Arc<Mutex<u32>>,
    pub(super) connection_task: Option<tokio::task::JoinHandle<()>>,
    pub(super) url: Option<String>,
    pub(super) metrics: Arc<Mutex<PerformanceMetrics>>,
}

impl WebTransportConnection {
    /// Create a new WebTransport connection
    pub async fn new(config: TransportConfig) -> Result<Self, TransportError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .http2_prior_knowledge() // Enable HTTP/2 for WebTransport
            .build()
            .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?;

        let (event_sender, event_receiver) = mpsc::unbounded_channel();

        Ok(Self {
            config,
            state: Arc::new(Mutex::new(ConnectionState::Disconnected)),
            client,
            event_sender: Some(event_sender),
            event_receiver: Some(event_receiver),
            streams: Arc::new(Mutex::new(HashMap::new())),
            next_stream_id: Arc::new(Mutex::new(1)),
            connection_task: None,
            url: None,
            metrics: Arc::new(Mutex::new(PerformanceMetrics::default())),
        })
    }

    /// Get current connection state
    pub fn state(&self) -> ConnectionState {
        *self.state.lock().unwrap()
    }

    /// Create a new stream with configuration
    pub async fn create_stream(&self, stream_config: super::config::StreamConfig) -> Result<AdvancedWebTransportStream, TransportError> {
        let stream_id = {
            let mut next_id = self.next_stream_id.lock().unwrap();
            let id = *next_id;
            *next_id += 1;
            id
        };

        let stream = AdvancedWebTransportStream::new(stream_id, stream_config);

        self.streams.lock().unwrap().insert(stream_id, stream.clone());

        // Update metrics
        let mut metrics = self.metrics.lock().unwrap();
        metrics.active_streams += 1;
        metrics.total_streams += 1;

        Ok(stream)
    }

    /// Close a specific stream
    pub async fn close_stream(&self, stream_id: u32) -> Result<(), TransportError> {
        if let Some(stream) = self.streams.lock().unwrap().remove(&stream_id) {
            // Close the stream
            drop(stream);

            // Update metrics
            let mut metrics = self.metrics.lock().unwrap();
            metrics.active_streams = metrics.active_streams.saturating_sub(1);

            Ok(())
        } else {
            Err(TransportError::InvalidState("Stream not found".to_string()))
        }
    }

    /// Get all active stream IDs
    pub fn active_stream_ids(&self) -> Vec<u32> {
        self.streams.lock().unwrap().keys().copied().collect()
    }

    /// Get stream count
    pub fn stream_count(&self) -> usize {
        self.streams.lock().unwrap().len()
    }

    /// Get performance metrics
    pub fn get_metrics(&self) -> PerformanceMetrics {
        self.metrics.lock().unwrap().clone()
    }

    /// Reset metrics
    pub fn reset_metrics(&self) {
        *self.metrics.lock().unwrap() = PerformanceMetrics::default();
    }

    /// Check if connection is ready for new streams
    pub fn can_create_stream(&self) -> bool {
        matches!(self.state(), ConnectionState::Connected)
            && self.stream_count() < 100 // Max streams limit
    }

    /// Get configuration
    pub fn config(&self) -> &TransportConfig {
        &self.config
    }

    /// Update configuration
    pub fn update_config(&mut self, config: TransportConfig) {
        self.config = config;
    }

    /// Get connection URL
    pub fn url(&self) -> Option<&String> {
        self.url.as_ref()
    }

    /// Set connection URL
    pub fn set_url(&mut self, url: String) {
        self.url = Some(url);
    }

    /// Start the connection establishment process
    pub async fn start_connection_task(&mut self, url: String) -> Result<(), TransportError> {
        if self.connection_task.is_some() {
            return Err(TransportError::InvalidState("Connection task already running".to_string()));
        }

        let state = Arc::clone(&self.state);
        let metrics = Arc::clone(&self.metrics);
        let client = self.client.clone();
        let url_clone = url.clone();

        let task = tokio::spawn(async move {
            // Simulate WebTransport connection establishment
            // In a real implementation, this would use WebTransport APIs

            let mut connection_attempts = 0u64;
            const MAX_ATTEMPTS: u64 = 3;

            while connection_attempts < MAX_ATTEMPTS {
                match Self::attempt_connection(&client, &url_clone).await {
                    Ok(_) => {
                        *state.lock().unwrap() = ConnectionState::Connected;

                        // Update metrics
                        let mut m = metrics.lock().unwrap();
                        m.connection_attempts = connection_attempts + 1;
                        m.successful_connections += 1;

                        return;
                    }
                    Err(_) => {
                        connection_attempts += 1;
                        if connection_attempts < MAX_ATTEMPTS {
                            tokio::time::sleep(Duration::from_millis(1000 * connection_attempts as u64)).await;
                        }
                    }
                }
            }

            // Failed to connect after all attempts
            *state.lock().unwrap() = ConnectionState::Disconnected;
            let mut m = metrics.lock().unwrap();
            m.connection_attempts = connection_attempts;
            m.failed_connections += 1;
        });

        self.connection_task = Some(task);
        self.url = Some(url);

        Ok(())
    }

    /// Attempt a single connection
    async fn attempt_connection(client: &Client, url: &str) -> Result<(), TransportError> {
        // Simulate connection attempt with HTTP/3
        let response = client
            .get(url)
            .header("Upgrade", "webtransport")
            .header("Connection", "Upgrade")
            .send()
            .await
            .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(TransportError::ConnectionFailed(format!(
                "HTTP {} response", response.status()
            )))
        }
    }

    /// Stop the connection and cleanup resources
    pub async fn disconnect(&mut self) -> Result<(), TransportError> {
        // Update state first
        *self.state.lock().unwrap() = ConnectionState::Disconnected;

        // Cancel connection task
        if let Some(task) = self.connection_task.take() {
            task.abort();
        }

        // Close all streams
        let stream_ids: Vec<u32> = self.streams.lock().unwrap().keys().copied().collect();
        for stream_id in stream_ids {
            let _ = self.close_stream(stream_id).await;
        }

        // Clear URL
        self.url = None;

        Ok(())
    }

    /// Check connection health
    pub async fn health_check(&self) -> Result<bool, TransportError> {
        match self.state() {
            ConnectionState::Connected => {
                // Perform actual health check if needed
                // For now, just return true if connected
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    /// Get connection statistics
    pub fn connection_stats(&self) -> ConnectionStats {
        let metrics = self.get_metrics();
        let stream_count = self.stream_count();
        let state = self.state();

        ConnectionStats {
            state,
            stream_count,
            metrics,
            url: self.url.clone(),
        }
    }
}

/// Connection statistics
#[derive(Debug, Clone)]
pub struct ConnectionStats {
    pub state: ConnectionState,
    pub stream_count: usize,
    pub metrics: PerformanceMetrics,
    pub url: Option<String>,
}

/// Helper function to find message boundary in buffer
pub(super) fn find_message_boundary(buffer: &[u8]) -> Option<usize> {
    // Look for newline as message separator
    buffer.windows(2).position(|w| w == b"\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_webtransport_creation() {
        let config = TransportConfig::default();
        let connection = WebTransportConnection::new(config).await.unwrap();

        assert!(matches!(connection.state(), ConnectionState::Disconnected));
        assert_eq!(connection.stream_count(), 0);
        assert!(connection.url().is_none());
    }

    #[tokio::test]
    async fn test_stream_management() {
        let config = TransportConfig::default();
        let connection = WebTransportConnection::new(config).await.unwrap();

        // Initially no streams
        assert_eq!(connection.stream_count(), 0);
        assert!(connection.active_stream_ids().is_empty());
    }

    #[test]
    fn test_message_boundary_detection() {
        let buffer = b"hello\n\nworld";
        assert_eq!(find_message_boundary(buffer), Some(5));

        let buffer_no_boundary = b"hello world";
        assert_eq!(find_message_boundary(buffer_no_boundary), None);
    }

    #[tokio::test]
    async fn test_connection_stats() {
        let config = TransportConfig::default();
        let connection = WebTransportConnection::new(config).await.unwrap();

        let stats = connection.connection_stats();
        assert!(matches!(stats.state, ConnectionState::Disconnected));
        assert_eq!(stats.stream_count, 0);
        assert!(stats.url.is_none());
    }
}
