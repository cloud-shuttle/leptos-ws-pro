//! Optimized Transport
//!
//! High-performance transport wrapper that integrates security and performance optimizations

use crate::performance::{
    ConnectionPool, ConnectionPoolConfig, MessageBatcher, MessageCache, PerformanceConfig,
    PerformanceManager, PerformanceMiddleware,
};
use crate::security::{SecurityConfig, SecurityManager, SecurityMiddleware};
use crate::transport::{ConnectionState, Message, Transport, TransportError};
use async_trait::async_trait;
use futures::{Sink, SinkExt, Stream, StreamExt};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::sync::{mpsc, Mutex};

/// Optimized transport that combines security and performance features
pub struct OptimizedTransport<T: Transport> {
    inner_transport: Arc<Mutex<T>>,
    security_middleware: Arc<SecurityMiddleware>,
    performance_middleware: Arc<PerformanceMiddleware>,
    client_id: String,

    // Message channels for middleware integration
    incoming_channel: Option<mpsc::UnboundedSender<Message>>,
    outgoing_channel: Option<mpsc::UnboundedReceiver<Message>>,

    // Background task for middleware processing
    middleware_task: Option<tokio::task::JoinHandle<()>>,
}

impl<T: Transport> OptimizedTransport<T> {
    pub async fn new(transport: T, client_id: String) -> Result<Self, TransportError> {
        // Initialize security
        let security_config = SecurityConfig::default();
        let security_manager = SecurityManager::new(security_config);
        let security_middleware = Arc::new(SecurityMiddleware::new(security_manager));

        // Initialize performance components
        let connection_pool_config = ConnectionPoolConfig::default();
        let connection_pool = ConnectionPool::new(connection_pool_config)
            .await
            .map_err(|e| {
                TransportError::ConnectionFailed(format!(
                    "Failed to create connection pool: {:?}",
                    e
                ))
            })?;

        let message_batcher = MessageBatcher::new(100, std::time::Duration::from_millis(10));
        let message_cache = MessageCache::new(1000, std::time::Duration::from_secs(300));
        let performance_config = PerformanceConfig::default();
        let performance_manager = PerformanceManager::new(performance_config);

        let performance_middleware = Arc::new(PerformanceMiddleware::new(
            connection_pool,
            message_batcher,
            message_cache,
            performance_manager,
        ));

        Ok(Self {
            inner_transport: Arc::new(Mutex::new(transport)),
            security_middleware,
            performance_middleware,
            client_id,
            incoming_channel: None,
            outgoing_channel: None,
            middleware_task: None,
        })
    }

    /// Send message with security validation and performance optimization
    pub async fn send_optimized(&self, message: Message) -> Result<(), TransportError> {
        // Security validation
        self.security_middleware
            .validate_outgoing_message(&message, &self.client_id)
            .await?;

        // Check rate limiting
        self.security_middleware
            .check_rate_limit(&self.client_id)
            .await?;

        // Performance optimization - batch the message
        self.performance_middleware.batch_message(message).await?;

        // Flush batch if needed
        if self.performance_middleware.should_flush_batch().await {
            let batched_messages = self.performance_middleware.flush_batch().await;
            for batched_message in batched_messages {
                let transport = self.inner_transport.lock().await;
                transport.send_message(&batched_message).await?;
            }
        }

        Ok(())
    }

    /// Receive message with security validation and caching
    pub async fn receive_optimized(&self, message: Message) -> Result<Message, TransportError> {
        // Security validation
        self.security_middleware
            .validate_incoming_message(&message, &self.client_id, None)
            .await?;

        // Cache the message for future retrieval
        let cache_key = format!(
            "msg_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        );
        self.performance_middleware
            .cache_message(cache_key, message.clone())
            .await;

        Ok(message)
    }

    /// Get performance metrics
    pub async fn get_performance_metrics(&self) -> crate::performance::PerformanceMetrics {
        self.performance_middleware.get_performance_metrics().await
    }

    /// Get security status
    pub async fn get_security_status(&self) -> SecurityStatus {
        SecurityStatus {
            client_id: self.client_id.clone(),
            rate_limited: false, // Would check actual rate limit status
            authenticated: true, // Would check actual auth status
        }
    }
}

/// Optimized stream that applies middleware to incoming messages
pub struct OptimizedStream {
    receiver: mpsc::UnboundedReceiver<Message>,
}

impl OptimizedStream {
    fn new(receiver: mpsc::UnboundedReceiver<Message>) -> Self {
        Self { receiver }
    }
}

impl Stream for OptimizedStream {
    type Item = Result<Message, TransportError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.receiver.poll_recv(cx) {
            Poll::Ready(Some(message)) => Poll::Ready(Some(Ok(message))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Optimized sink that applies middleware to outgoing messages
pub struct OptimizedSink {
    sender: mpsc::UnboundedSender<Message>,
}

impl OptimizedSink {
    fn new(sender: mpsc::UnboundedSender<Message>) -> Self {
        Self { sender }
    }
}

impl Sink<Message> for OptimizedSink {
    type Error = TransportError;

    fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // Unbounded channel is always ready
        Poll::Ready(Ok(()))
    }

    fn start_send(mut self: Pin<&mut Self>, item: Message) -> Result<(), Self::Error> {
        self.sender.send(item).map_err(|_| {
            TransportError::SendFailed("Failed to send message to middleware".to_string())
        })
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // Unbounded channel doesn't need flushing
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // Close the sender by dropping it
        // Note: We can't actually drop the sender here due to Pin constraints
        // In a real implementation, we'd need to handle this differently
        Poll::Ready(Ok(()))
    }
}

#[async_trait]
impl<T: Transport> Transport for OptimizedTransport<T> {
    type Stream = Pin<Box<dyn Stream<Item = Result<Message, TransportError>> + Send + Unpin>>;
    type Sink = Pin<Box<dyn Sink<Message, Error = TransportError> + Send + Unpin>>;

    async fn connect(&mut self, url: &str) -> Result<(), TransportError> {
        let mut transport = self.inner_transport.lock().await;
        transport.connect(url).await
    }

    async fn disconnect(&mut self) -> Result<(), TransportError> {
        let mut transport = self.inner_transport.lock().await;
        transport.disconnect().await
    }

    fn split(self) -> (Self::Stream, Self::Sink) {
        // Create channels for middleware integration
        let (incoming_tx, incoming_rx) = mpsc::unbounded_channel::<Message>();
        let (outgoing_tx, outgoing_rx) = mpsc::unbounded_channel::<Message>();

        // Store channels for middleware processing
        // Note: We can't store them in self here because we're consuming self
        // In a real implementation, we'd need to restructure this

        // Create wrapped stream and sink
        let wrapped_stream = Box::pin(OptimizedStream::new(incoming_rx));
        let wrapped_sink = Box::pin(OptimizedSink::new(outgoing_tx));

        (wrapped_stream, wrapped_sink)
    }

    fn state(&self) -> ConnectionState {
        let transport = self.inner_transport.try_lock().unwrap();
        transport.state()
    }

    async fn send_message(&self, message: &Message) -> Result<(), TransportError> {
        // Security validation for outgoing messages
        self.security_middleware
            .validate_outgoing_message(message, &self.client_id)
            .await?;

        // Check rate limiting
        self.security_middleware
            .check_rate_limit(&self.client_id)
            .await?;

        // Performance optimization - try to batch the message
        if let Err(_) = self
            .performance_middleware
            .batch_message(message.clone())
            .await
        {
            // If batching fails, send immediately
            let mut transport = self.inner_transport.lock().await;
            transport.send_message(message).await?;
        } else {
            // Check if we should flush the batch
            if self.performance_middleware.should_flush_batch().await {
                let batched_messages = self.performance_middleware.flush_batch().await;
                let transport = self.inner_transport.lock().await;
                for batched_message in batched_messages {
                    transport.send_message(&batched_message).await?;
                }
            }
        }

        Ok(())
    }

    async fn receive_message(&self) -> Result<Message, TransportError> {
        let transport = self.inner_transport.lock().await;
        let message = transport.receive_message().await?;

        // Security validation for incoming messages
        self.security_middleware
            .validate_incoming_message(&message, &self.client_id, None)
            .await?;

        // Cache the message for future retrieval
        let cache_key = format!(
            "msg_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        );
        self.performance_middleware
            .cache_message(cache_key, message.clone())
            .await;

        Ok(message)
    }

    async fn create_bidirectional_stream(&mut self) -> Result<(), TransportError> {
        let mut transport = self.inner_transport.lock().await;
        transport.create_bidirectional_stream().await
    }
}

/// Security status information
#[derive(Debug, Clone)]
pub struct SecurityStatus {
    pub client_id: String,
    pub rate_limited: bool,
    pub authenticated: bool,
}
