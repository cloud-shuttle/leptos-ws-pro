use crate::transport::{ConnectionState, Message, Transport, TransportConfig, TransportError};
use crate::transport::websocket::WebSocketConnection;
use crate::transport::sse::SseConnection;
use crate::transport::webtransport::WebTransportConnection;
use async_trait::async_trait;
use futures::{Sink, SinkExt, Stream};
use std::pin::Pin;
use std::sync::{Arc, Mutex};

/// Transport capabilities detected by the adaptive transport
#[derive(Debug, Clone)]
pub struct TransportCapabilities {
    pub websocket_supported: bool,
    pub webtransport_supported: bool,
    pub sse_supported: bool,
}

impl TransportCapabilities {
    /// Detect available transport capabilities
    pub fn detect() -> Self {
        Self {
            websocket_supported: true, // WebSocket is always supported in our implementation
            webtransport_supported: true, // WebTransport is now implemented
            sse_supported: true, // SSE is now implemented
        }
    }

    /// Check if WebTransport is supported
    pub fn supports_webtransport(&self) -> bool {
        self.webtransport_supported
    }

    /// Check if WebSocket is supported
    pub fn supports_websocket(&self) -> bool {
        self.websocket_supported
    }

    /// Check if SSE is supported
    pub fn supports_sse(&self) -> bool {
        self.sse_supported
    }

    /// Check if streaming is supported (WebTransport feature)
    pub fn supports_streaming(&self) -> bool {
        self.webtransport_supported
    }

    /// Check if multiplexing is supported (WebTransport feature)
    pub fn supports_multiplexing(&self) -> bool {
        self.webtransport_supported
    }

    /// Check if bidirectional communication is supported
    pub fn supports_bidirectional(&self) -> bool {
        self.websocket_supported || self.webtransport_supported
    }

    /// Check if unidirectional communication is supported
    pub fn supports_unidirectional(&self) -> bool {
        self.sse_supported
    }
}

/// Performance metrics for adaptive transport
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub connection_count: u64,
    pub message_count: u64,
    pub error_count: u64,
}

/// Adaptive transport that tries multiple protocols
pub struct AdaptiveTransport {
    config: TransportConfig,
    state: Arc<Mutex<ConnectionState>>,
    selected_transport: Arc<Mutex<String>>,
    websocket_connection: Option<WebSocketConnection>,
    sse_connection: Option<SseConnection>,
    webtransport_connection: Option<WebTransportConnection>,
    capabilities: TransportCapabilities,
    metrics: Arc<Mutex<PerformanceMetrics>>,
}

impl AdaptiveTransport {
    pub async fn new(config: TransportConfig) -> Result<Self, TransportError> {
        let capabilities = Self::detect_capabilities().await;

        Ok(Self {
            config,
            state: Arc::new(Mutex::new(ConnectionState::Disconnected)),
            selected_transport: Arc::new(Mutex::new("None".to_string())),
            websocket_connection: None,
            sse_connection: None,
            webtransport_connection: None,
            capabilities,
            metrics: Arc::new(Mutex::new(PerformanceMetrics {
                connection_count: 0,
                message_count: 0,
                error_count: 0,
            })),
        })
    }

    pub async fn detect_capabilities() -> TransportCapabilities {
        TransportCapabilities::detect()
    }

    pub fn selected_transport(&self) -> String {
        self.selected_transport.lock().unwrap().clone()
    }

    pub async fn connect_with_fallback(&mut self, url: &str) -> Result<(), TransportError> {
        // Try WebSocket first (most reliable)
        if self.capabilities.websocket_supported {
            match self.try_websocket_connection(url).await {
                Ok(()) => {
                    *self.selected_transport.lock().unwrap() = "WebSocket".to_string();
                    *self.state.lock().unwrap() = ConnectionState::Connected;
                    self.metrics.lock().unwrap().connection_count += 1;
                    return Ok(());
                }
                Err(_e) => {
                    self.metrics.lock().unwrap().error_count += 1;
                    // Continue to next transport
                }
            }
        }

        // Try WebTransport as second choice (modern, efficient)
        if self.capabilities.webtransport_supported {
            match self.try_webtransport_connection(url).await {
                Ok(()) => {
                    *self.selected_transport.lock().unwrap() = "WebTransport".to_string();
                    *self.state.lock().unwrap() = ConnectionState::Connected;
                    self.metrics.lock().unwrap().connection_count += 1;
                    return Ok(());
                }
                Err(_e) => {
                    self.metrics.lock().unwrap().error_count += 1;
                    // Continue to next transport
                }
            }
        }

        // Try SSE as final fallback (simple, widely supported)
        if self.capabilities.sse_supported {
            match self.try_sse_connection(url).await {
                Ok(()) => {
                    *self.selected_transport.lock().unwrap() = "SSE".to_string();
                    *self.state.lock().unwrap() = ConnectionState::Connected;
                    self.metrics.lock().unwrap().connection_count += 1;
                    return Ok(());
                }
                Err(_e) => {
                    self.metrics.lock().unwrap().error_count += 1;
                    // All transports failed
                }
            }
        }

        Err(TransportError::ConnectionFailed("All transport methods failed".to_string()))
    }

    async fn try_websocket_connection(&mut self, url: &str) -> Result<(), TransportError> {
        let mut ws_conn = WebSocketConnection::new(self.config.clone()).await?;
        ws_conn.connect(url).await?;
        self.websocket_connection = Some(ws_conn);
        Ok(())
    }

    async fn try_sse_connection(&mut self, url: &str) -> Result<(), TransportError> {
        let mut sse_conn = SseConnection::new(self.config.clone()).await?;
        sse_conn.connect(url).await?;
        self.sse_connection = Some(sse_conn);
        Ok(())
    }

    async fn try_webtransport_connection(&mut self, url: &str) -> Result<(), TransportError> {
        let mut wt_conn = WebTransportConnection::new(self.config.clone()).await?;
        wt_conn.connect(url).await?;
        self.webtransport_connection = Some(wt_conn);
        Ok(())
    }

    pub fn get_performance_metrics(&self) -> PerformanceMetrics {
        self.metrics.lock().unwrap().clone()
    }

    pub fn can_switch_transport(&self) -> bool {
        true // Adaptive transport can always switch
    }

    pub fn get_available_transports(&self) -> Vec<String> {
        let mut transports = Vec::new();
        if self.capabilities.websocket_supported {
            transports.push("WebSocket".to_string());
        }
        if self.capabilities.webtransport_supported {
            transports.push("WebTransport".to_string());
        }
        if self.capabilities.sse_supported {
            transports.push("SSE".to_string());
        }
        transports
    }
}

#[async_trait]
impl Transport for AdaptiveTransport {
    type Stream = Pin<Box<dyn Stream<Item = Result<Message, TransportError>> + Send + Unpin>>;
    type Sink = Pin<Box<dyn Sink<Message, Error = TransportError> + Send + Unpin>>;

    async fn connect(&mut self, url: &str) -> Result<(), TransportError> {
        self.connect_with_fallback(url).await
    }

    async fn disconnect(&mut self) -> Result<(), TransportError> {
        if let Some(mut ws_conn) = self.websocket_connection.take() {
            let _ = ws_conn.disconnect().await;
        }
        if let Some(mut sse_conn) = self.sse_connection.take() {
            let _ = sse_conn.disconnect().await;
        }
        if let Some(mut wt_conn) = self.webtransport_connection.take() {
            let _ = wt_conn.disconnect().await;
        }
        *self.state.lock().unwrap() = ConnectionState::Disconnected;
        *self.selected_transport.lock().unwrap() = "None".to_string();
        Ok(())
    }

    fn split(self) -> (Self::Stream, Self::Sink) {
        // Delegate to the active connection
        if let Some(ws_conn) = self.websocket_connection {
            ws_conn.split()
        } else if let Some(sse_conn) = self.sse_connection {
            sse_conn.split()
        } else if let Some(wt_conn) = self.webtransport_connection {
            wt_conn.split()
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

    fn state(&self) -> ConnectionState {
        *self.state.lock().unwrap()
    }
}

impl AdaptiveTransport {
    /// Negotiate the best protocol from a list of supported protocols
    pub async fn negotiate_protocol(&mut self, supported_protocols: Vec<String>) -> Result<(), TransportError> {
        // Try protocols in order of preference
        for protocol in supported_protocols {
            match protocol.as_str() {
                "websocket" if self.capabilities.websocket_supported => {
                    if self.try_websocket_connection("ws://localhost:8080").await.is_ok() {
                        *self.selected_transport.lock().unwrap() = "WebSocket".to_string();
                        return Ok(());
                    }
                }
                "webtransport" if self.capabilities.webtransport_supported => {
                    if self.try_webtransport_connection("https://localhost:8080").await.is_ok() {
                        *self.selected_transport.lock().unwrap() = "WebTransport".to_string();
                        return Ok(());
                    }
                }
                "sse" if self.capabilities.sse_supported => {
                    if self.try_sse_connection("http://localhost:8080").await.is_ok() {
                        *self.selected_transport.lock().unwrap() = "SSE".to_string();
                        return Ok(());
                    }
                }
                _ => continue,
            }
        }
        Err(TransportError::ConnectionFailed("No supported protocol available".to_string()))
    }

    /// Check if WebTransport is available
    pub fn is_webtransport_available(&self) -> bool {
        self.capabilities.webtransport_supported
    }

    /// Check if WebSocket is available
    pub fn is_websocket_available(&self) -> bool {
        self.capabilities.websocket_supported
    }

    /// Check if SSE is available
    pub fn is_sse_available(&self) -> bool {
        self.capabilities.sse_supported
    }

    /// Get the current protocol being used
    pub async fn current_protocol(&self) -> String {
        self.selected_transport.lock().unwrap().clone()
    }

    /// Check if the transport is connected
    pub fn is_connected(&self) -> bool {
        matches!(*self.state.lock().unwrap(), ConnectionState::Connected)
    }

    /// Simulate a protocol failure for testing
    pub async fn simulate_protocol_failure(&mut self, protocol: String) {
        if *self.selected_transport.lock().unwrap() == protocol {
            *self.state.lock().unwrap() = ConnectionState::Failed;
        }
    }

    /// Fallback to the next available protocol
    pub async fn fallback_to_next_protocol(&mut self, fallback_protocols: Vec<String>) -> Result<(), TransportError> {
        *self.state.lock().unwrap() = ConnectionState::Reconnecting;

        for protocol in fallback_protocols {
            match protocol.as_str() {
                "websocket" if self.capabilities.websocket_supported => {
                    if self.try_websocket_connection("ws://localhost:8080").await.is_ok() {
                        *self.selected_transport.lock().unwrap() = "WebSocket".to_string();
                        *self.state.lock().unwrap() = ConnectionState::Connected;
                        return Ok(());
                    }
                }
                "webtransport" if self.capabilities.webtransport_supported => {
                    if self.try_webtransport_connection("https://localhost:8080").await.is_ok() {
                        *self.selected_transport.lock().unwrap() = "WebTransport".to_string();
                        *self.state.lock().unwrap() = ConnectionState::Connected;
                        return Ok(());
                    }
                }
                "sse" if self.capabilities.sse_supported => {
                    if self.try_sse_connection("http://localhost:8080").await.is_ok() {
                        *self.selected_transport.lock().unwrap() = "SSE".to_string();
                        *self.state.lock().unwrap() = ConnectionState::Connected;
                        return Ok(());
                    }
                }
                _ => continue,
            }
        }
        Err(TransportError::ConnectionFailed("All fallback protocols failed".to_string()))
    }
}
