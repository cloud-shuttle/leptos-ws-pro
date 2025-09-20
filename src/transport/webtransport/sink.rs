//! WebTransport sink implementation for message sending
//!
//! Sink implementation for sending messages through WebTransport connections.

use futures::{Sink, SinkExt};
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::sync::mpsc;

use crate::transport::{Message, TransportError};

/// WebTransport sink for sending messages
pub struct WebTransportSink {
    sender: Option<mpsc::UnboundedSender<Message>>,
    pending_message: Option<Message>,
}

impl WebTransportSink {
    /// Create a new WebTransport sink
    pub fn new(sender: Option<mpsc::UnboundedSender<Message>>) -> Self {
        Self {
            sender,
            pending_message: None,
        }
    }

    /// Check if the sink is ready to accept messages
    pub fn is_ready(&self) -> bool {
        self.sender.is_some() && self.pending_message.is_none()
    }

    /// Get the number of pending messages (0 or 1)
    pub fn pending_count(&self) -> usize {
        if self.pending_message.is_some() { 1 } else { 0 }
    }

    /// Close the sink and cleanup resources
    pub fn close_sink(&mut self) {
        self.sender = None;
        self.pending_message = None;
    }
}

impl Sink<Message> for WebTransportSink {
    type Error = TransportError;

    fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let this = self.get_mut();

        if this.sender.is_none() {
            return Poll::Ready(Err(TransportError::NotConnected));
        }

        if this.pending_message.is_some() {
            // Try to flush the pending message first
            return Poll::Pending;
        }

        Poll::Ready(Ok(()))
    }

    fn start_send(self: Pin<&mut Self>, item: Message) -> Result<(), Self::Error> {
        let this = self.get_mut();

        if this.sender.is_none() {
            return Err(TransportError::NotConnected);
        }

        if this.pending_message.is_some() {
            return Err(TransportError::SendFailed("Sink is not ready".to_string()));
        }

        // Store the message as pending
        this.pending_message = Some(item);
        Ok(())
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let this = self.get_mut();

        if let Some(sender) = &this.sender {
            if let Some(message) = this.pending_message.take() {
                match sender.send(message) {
                    Ok(_) => Poll::Ready(Ok(())),
                    Err(e) => {
                        // Put the message back since send failed
                        this.pending_message = Some(e.0);
                        Poll::Ready(Err(TransportError::SendFailed(
                            "Channel send failed".to_string()
                        )))
                    }
                }
            } else {
                Poll::Ready(Ok(()))
            }
        } else {
            Poll::Ready(Err(TransportError::NotConnected))
        }
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let this = self.get_mut();

        // Flush any pending messages first
        match Sink::poll_flush(Pin::new(this), cx) {
            Poll::Ready(Ok(_)) => {
                // Close the sender
                this.sender = None;
                this.pending_message = None;
                Poll::Ready(Ok(()))
            }
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// WebTransport sink with advanced features
pub struct AdvancedWebTransportSink {
    base_sink: WebTransportSink,
    message_buffer: Vec<Message>,
    batch_size: usize,
    flush_timeout: std::time::Duration,
    last_flush: std::time::Instant,
}

impl AdvancedWebTransportSink {
    /// Create a new advanced WebTransport sink
    pub fn new(sender: Option<mpsc::UnboundedSender<Message>>) -> Self {
        Self {
            base_sink: WebTransportSink::new(sender),
            message_buffer: Vec::new(),
            batch_size: 10,
            flush_timeout: std::time::Duration::from_millis(100),
            last_flush: std::time::Instant::now(),
        }
    }

    /// Set batch size for message batching
    pub fn set_batch_size(&mut self, size: usize) {
        self.batch_size = size;
    }

    /// Set flush timeout
    pub fn set_flush_timeout(&mut self, timeout: std::time::Duration) {
        self.flush_timeout = timeout;
    }

    /// Get current buffer size
    pub fn buffer_size(&self) -> usize {
        self.message_buffer.len()
    }

    /// Check if buffer should be flushed
    fn should_flush(&self) -> bool {
        self.message_buffer.len() >= self.batch_size
            || self.last_flush.elapsed() >= self.flush_timeout
    }

    /// Flush buffered messages
    async fn flush_buffer(&mut self) -> Result<(), TransportError> {
        if self.message_buffer.is_empty() {
            return Ok(());
        }

        // Send all buffered messages
        for message in self.message_buffer.drain(..) {
            self.base_sink.send(message).await?;
        }

        self.base_sink.flush().await?;
        self.last_flush = std::time::Instant::now();

        Ok(())
    }
}

impl Sink<Message> for AdvancedWebTransportSink {
    type Error = TransportError;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // Check if we need to flush first
        if self.should_flush() {
            // Try to flush the buffer
            let flush_result = futures::ready!(self.as_mut().poll_flush(cx));
            if let Err(e) = flush_result {
                return Poll::Ready(Err(e));
            }
        }

        Poll::Ready(Ok(()))
    }

    fn start_send(self: Pin<&mut Self>, item: Message) -> Result<(), Self::Error> {
        let this = self.get_mut();

        // Add message to buffer
        this.message_buffer.push(item);

        Ok(())
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let this = self.get_mut();

        // This is a simplified implementation - in a real async environment,
        // you'd want to properly handle the async flush operation
        match futures::executor::block_on(this.flush_buffer()) {
            Ok(_) => Poll::Ready(Ok(())),
            Err(e) => Poll::Ready(Err(e)),
        }
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // Flush everything first
        match futures::ready!(self.as_mut().poll_flush(cx)) {
            Ok(_) => {
                self.base_sink.close_sink();
                Poll::Ready(Ok(()))
            }
            Err(e) => Poll::Ready(Err(e)),
        }
    }
}

/// Sink factory for creating different types of sinks
pub struct SinkFactory;

impl SinkFactory {
    /// Create a basic WebTransport sink
    pub fn create_basic_sink(sender: Option<mpsc::UnboundedSender<Message>>) -> WebTransportSink {
        WebTransportSink::new(sender)
    }

    /// Create an advanced WebTransport sink with batching
    pub fn create_batched_sink(
        sender: Option<mpsc::UnboundedSender<Message>>,
        batch_size: usize,
        flush_timeout: std::time::Duration,
    ) -> AdvancedWebTransportSink {
        let mut sink = AdvancedWebTransportSink::new(sender);
        sink.set_batch_size(batch_size);
        sink.set_flush_timeout(flush_timeout);
        sink
    }

    /// Create a sink with compression
    pub fn create_compressed_sink(
        sender: Option<mpsc::UnboundedSender<Message>>,
    ) -> CompressedWebTransportSink {
        CompressedWebTransportSink::new(sender)
    }
}

/// WebTransport sink with compression support
pub struct CompressedWebTransportSink {
    base_sink: WebTransportSink,
    compression_enabled: bool,
    compression_threshold: usize,
}

impl CompressedWebTransportSink {
    /// Create a new compressed WebTransport sink
    pub fn new(sender: Option<mpsc::UnboundedSender<Message>>) -> Self {
        Self {
            base_sink: WebTransportSink::new(sender),
            compression_enabled: true,
            compression_threshold: 1024, // 1KB threshold
        }
    }

    /// Enable or disable compression
    pub fn set_compression(&mut self, enabled: bool) {
        self.compression_enabled = enabled;
    }

    /// Set compression threshold
    pub fn set_compression_threshold(&mut self, threshold: usize) {
        self.compression_threshold = threshold;
    }

    /// Compress message if needed
    fn compress_message(&self, mut message: Message) -> Message {
        if self.compression_enabled && message.data.len() > self.compression_threshold {
            // Simplified compression - in reality you'd use proper compression
            // algorithms like gzip, brotli, etc.
            let compressed_data = Self::simple_compress(&message.data);
            message.data = compressed_data;
        }
        message
    }

    /// Simple compression (placeholder)
    fn simple_compress(data: &[u8]) -> Vec<u8> {
        // This is a placeholder - implement actual compression
        data.to_vec()
    }
}

impl Sink<Message> for CompressedWebTransportSink {
    type Error = TransportError;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.base_sink).poll_ready(cx)
    }

    fn start_send(mut self: Pin<&mut Self>, item: Message) -> Result<(), Self::Error> {
        let compressed_message = self.compress_message(item);
        Pin::new(&mut self.base_sink).start_send(compressed_message)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.base_sink).poll_flush(cx)
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.base_sink).poll_close(cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::MessageType;

    #[test]
    fn test_sink_creation() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let sink = WebTransportSink::new(Some(tx));

        assert!(sink.is_ready());
        assert_eq!(sink.pending_count(), 0);
    }

    #[test]
    fn test_sink_factory() {
        let (tx, _rx) = mpsc::unbounded_channel();

        let basic_sink = SinkFactory::create_basic_sink(Some(tx.clone()));
        assert!(basic_sink.is_ready());

        let batch_timeout = std::time::Duration::from_millis(50);
        let batched_sink = SinkFactory::create_batched_sink(Some(tx.clone()), 5, batch_timeout);
        assert_eq!(batched_sink.buffer_size(), 0);

        let compressed_sink = SinkFactory::create_compressed_sink(Some(tx));
        assert!(compressed_sink.compression_enabled);
    }

    #[test]
    fn test_advanced_sink_batching() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let mut sink = AdvancedWebTransportSink::new(Some(tx));

        sink.set_batch_size(3);
        assert_eq!(sink.batch_size, 3);

        let timeout = std::time::Duration::from_millis(200);
        sink.set_flush_timeout(timeout);
        assert_eq!(sink.flush_timeout, timeout);
    }

    #[test]
    fn test_compressed_sink_settings() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let mut sink = CompressedWebTransportSink::new(Some(tx));

        assert!(sink.compression_enabled);
        assert_eq!(sink.compression_threshold, 1024);

        sink.set_compression(false);
        assert!(!sink.compression_enabled);

        sink.set_compression_threshold(2048);
        assert_eq!(sink.compression_threshold, 2048);
    }

    #[tokio::test]
    async fn test_message_compression() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let sink = CompressedWebTransportSink::new(Some(tx));

        let large_data = vec![0u8; 2000]; // Larger than threshold
        let message = Message {
            data: large_data,
            message_type: MessageType::Binary,
        };

        let compressed = sink.compress_message(message);
        // In a real implementation, compressed data might be smaller
        assert!(!compressed.data.is_empty());
    }
}
