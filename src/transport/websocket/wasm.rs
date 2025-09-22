//! WASM WebSocket implementation for browser environments
//!
//! This module provides WebSocket functionality for WebAssembly targets,
//! using the browser's native WebSocket API via web-sys.

use crate::transport::{
    ConnectionState, Message, MessageType, Transport, TransportCapabilities, TransportConfig,
    TransportError,
};
use async_trait::async_trait;
use futures::{Sink, Stream};
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::sync::mpsc;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::JsFuture;
#[cfg(target_arch = "wasm32")]
use web_sys::{WebSocket, MessageEvent, CloseEvent, ErrorEvent, ReadyState as WebSocketReadyState};
#[cfg(target_arch = "wasm32")]
use js_sys::{JsString, Uint8Array};

/// WASM WebSocket connection implementation
#[cfg(target_arch = "wasm32")]
pub struct WasmWebSocketConnection {
    config: TransportConfig,
    state: Arc<Mutex<ConnectionState>>,
    ws: Option<WebSocket>,
    message_sender: Option<mpsc::UnboundedSender<Message>>,
    message_receiver: Option<mpsc::UnboundedReceiver<Message>>,
    // Store closures to prevent them from being dropped
    _onmessage: Option<Closure<dyn FnMut(MessageEvent)>>,
    _onclose: Option<Closure<dyn FnMut(CloseEvent)>>,
    _onerror: Option<Closure<dyn FnMut(ErrorEvent)>>,
}

#[cfg(target_arch = "wasm32")]
impl WasmWebSocketConnection {
    /// Create a new WASM WebSocket connection
    pub async fn new(config: TransportConfig) -> Result<Self, TransportError> {
        let (message_sender, message_receiver) = mpsc::unbounded_channel();

        Ok(Self {
            config,
            state: Arc::new(Mutex::new(ConnectionState::Disconnected)),
            ws: None,
            message_sender: Some(message_sender),
            message_receiver: Some(message_receiver),
            _onmessage: None,
            _onclose: None,
            _onerror: None,
        })
    }

    /// Get transport capabilities
    pub fn capabilities(&self) -> TransportCapabilities {
        TransportCapabilities {
            websocket: true,
            webtransport: false,
            sse: false,
            binary: true,
            compression: false,
        }
    }

    /// Setup event handlers for the WebSocket
    fn setup_event_handlers(&mut self) -> Result<(), TransportError> {
        let ws = self.ws.as_ref().ok_or_else(|| {
            TransportError::InvalidState("WebSocket not initialized".to_string())
        })?;

        let state = Arc::clone(&self.state);
        let sender = self.message_sender.as_ref().ok_or_else(|| {
            TransportError::InvalidState("Message sender not available".to_string())
        })?;

        // Setup message handler
        let onmessage = Closure::wrap(Box::new(move |event: MessageEvent| {
            if let Ok(data) = event.data().dyn_into::<JsString>() {
                if let Some(text) = data.as_string() {
                    let message = Message {
                        data: text.into_bytes(),
                        message_type: MessageType::Text,
                    };
                    let _ = sender.send(message);
                }
            } else if let Ok(array_buffer) = event.data().dyn_into::<js_sys::ArrayBuffer>() {
                let uint8_array = Uint8Array::new(&array_buffer);
                let mut data = vec![0u8; uint8_array.length() as usize];
                uint8_array.copy_to(&mut data);
                let message = Message {
                    data,
                    message_type: MessageType::Binary,
                };
                let _ = sender.send(message);
            }
        }) as Box<dyn FnMut(MessageEvent)>);

        ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
        self._onmessage = Some(onmessage);

        // Setup close handler
        let onclose = Closure::wrap(Box::new(move |_event: CloseEvent| {
            *state.lock().unwrap() = ConnectionState::Disconnected;
        }) as Box<dyn FnMut(CloseEvent)>);

        ws.set_onclose(Some(onclose.as_ref().unchecked_ref()));
        self._onclose = Some(onclose);

        // Setup error handler
        let onerror = Closure::wrap(Box::new(move |_event: ErrorEvent| {
            *state.lock().unwrap() = ConnectionState::Failed;
        }) as Box<dyn FnMut(ErrorEvent)>);

        ws.set_onerror(Some(onerror.as_ref().unchecked_ref()));
        self._onerror = Some(onerror);

        Ok(())
    }

    /// Get the current ready state of the WebSocket
    pub fn get_ready_state(&self) -> Result<WebSocketReadyState, TransportError> {
        let ws = self.ws.as_ref().ok_or_else(|| {
            TransportError::InvalidState("WebSocket not initialized".to_string())
        })?;
        Ok(ws.ready_state())
    }

    /// Get the protocol of the WebSocket
    pub fn get_protocol(&self) -> Result<String, TransportError> {
        let ws = self.ws.as_ref().ok_or_else(|| {
            TransportError::InvalidState("WebSocket not initialized".to_string())
        })?;
        ws.protocol().map_err(|_| TransportError::ProtocolError("Failed to get protocol".to_string()))
    }

    /// Get the URL of the WebSocket
    pub fn get_url(&self) -> Result<String, TransportError> {
        let ws = self.ws.as_ref().ok_or_else(|| {
            TransportError::InvalidState("WebSocket not initialized".to_string())
        })?;
        Ok(ws.url())
    }
}

#[cfg(target_arch = "wasm32")]
#[async_trait]
impl Transport for WasmWebSocketConnection {
    type Stream = Pin<Box<dyn Stream<Item = Result<Message, TransportError>> + Send + Unpin>>;
    type Sink = Pin<Box<dyn Sink<Message, Error = TransportError> + Send + Unpin>>;

    async fn connect(&mut self, url: &str) -> Result<(), TransportError> {
        *self.state.lock().unwrap() = ConnectionState::Connecting;

        // Create WebSocket
        let ws = WebSocket::new(url)
            .map_err(|_| TransportError::ConnectionFailed("Failed to create WebSocket".to_string()))?;

        // Check if connection was successful
        if ws.ready_state() == WebSocketReadyState::Connecting {
            // Wait for connection to be established
            // In a real implementation, we would wait for the onopen event
            // For now, we'll simulate a successful connection
            *self.state.lock().unwrap() = ConnectionState::Connected;
        } else {
            return Err(TransportError::ConnectionFailed("WebSocket creation failed".to_string()));
        }

        self.ws = Some(ws);
        self.setup_event_handlers()?;

        Ok(())
    }

    async fn disconnect(&mut self) -> Result<(), TransportError> {
        *self.state.lock().unwrap() = ConnectionState::Disconnected;

        if let Some(ws) = &self.ws {
            ws.close().map_err(|_| TransportError::ConnectionFailed("Failed to close WebSocket".to_string()))?;
        }

        // Clear event handlers
        self._onmessage = None;
        self._onclose = None;
        self._onerror = None;

        Ok(())
    }

    async fn send_message(&self, message: &Message) -> Result<(), TransportError> {
        if !matches!(self.state(), ConnectionState::Connected) {
            return Err(TransportError::NotConnected);
        }

        let ws = self.ws.as_ref().ok_or_else(|| {
            TransportError::InvalidState("WebSocket not initialized".to_string())
        })?;

        match message.message_type {
            MessageType::Text => {
                let text = String::from_utf8(message.data.clone())
                    .map_err(|_| TransportError::SendFailed("Invalid UTF-8".to_string()))?;
                ws.send_with_str(&text)
                    .map_err(|_| TransportError::SendFailed("Send failed".to_string()))?;
            }
            MessageType::Binary => {
                let array = Uint8Array::new_with_length(message.data.len() as u32);
                array.copy_from(&message.data);
                ws.send_with_array_buffer(&array.buffer())
                    .map_err(|_| TransportError::SendFailed("Send failed".to_string()))?;
            }
            _ => return Err(TransportError::NotSupported("Message type not supported".to_string())),
        }

        Ok(())
    }

    fn split(self) -> (Self::Stream, Self::Sink) {
        // Create a new channel for the split connection
        let (sender, receiver) = mpsc::unbounded_channel();

        // Create sink and stream with the new channel
        let sink = WasmWebSocketSink::new(Some(sender));
        let stream = self.create_message_stream_from_receiver(receiver);

        (
            Box::pin(stream) as Self::Stream,
            Box::pin(sink) as Self::Sink,
        )
    }

    fn state(&self) -> ConnectionState {
        *self.state.lock().unwrap()
    }

    async fn create_bidirectional_stream(&mut self) -> Result<(), TransportError> {
        // WASM WebSocket doesn't support bidirectional streams in the same way
        // This is a no-op for WASM implementation
        Ok(())
    }
}

#[cfg(target_arch = "wasm32")]
impl WasmWebSocketConnection {
    /// Create a message stream from a receiver
    fn create_message_stream_from_receiver(&self, receiver: mpsc::UnboundedReceiver<Message>) -> impl Stream<Item = Result<Message, TransportError>> {
        use futures::Stream;
        use std::pin::Pin;
        use std::task::{Context, Poll};

        // Simple wrapper to convert receiver to stream
        struct ReceiverStream(mpsc::UnboundedReceiver<Message>);

        impl Stream for ReceiverStream {
            type Item = Result<Message, TransportError>;

            fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
                match self.0.try_recv() {
                    Ok(msg) => Poll::Ready(Some(Ok(msg))),
                    Err(mpsc::error::TryRecvError::Empty) => Poll::Pending,
                    Err(mpsc::error::TryRecvError::Disconnected) => Poll::Ready(None),
                }
            }
        }

        ReceiverStream(receiver)
    }
}

/// WASM WebSocket sink for sending messages
#[cfg(target_arch = "wasm32")]
pub struct WasmWebSocketSink {
    sender: Option<mpsc::UnboundedSender<Message>>,
    pending_message: Option<Message>,
}

#[cfg(target_arch = "wasm32")]
impl WasmWebSocketSink {
    /// Create a new WASM WebSocket sink
    pub fn new(sender: Option<mpsc::UnboundedSender<Message>>) -> Self {
        Self {
            sender,
            pending_message: None,
        }
    }
}

#[cfg(target_arch = "wasm32")]
impl Sink<Message> for WasmWebSocketSink {
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
                Poll::Ready(Ok(()))
            }
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Pending => Poll::Pending,
        }
    }
}

// Non-WASM targets get a stub implementation
#[cfg(not(target_arch = "wasm32"))]
pub struct WasmWebSocketConnection;

#[cfg(not(target_arch = "wasm32"))]
impl WasmWebSocketConnection {
    pub async fn new(_config: TransportConfig) -> Result<Self, TransportError> {
        Err(TransportError::NotSupported("WASM WebSocket not available on non-WASM targets".to_string()))
    }

    pub fn capabilities(&self) -> TransportCapabilities {
        TransportCapabilities {
            websocket: false,
            webtransport: false,
            sse: false,
            binary: false,
            compression: false,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
impl Transport for WasmWebSocketConnection {
    type Stream = Pin<Box<dyn Stream<Item = Result<Message, TransportError>> + Send + Unpin>>;
    type Sink = Pin<Box<dyn Sink<Message, Error = TransportError> + Send + Unpin>>;

    async fn connect(&mut self, _url: &str) -> Result<(), TransportError> {
        Err(TransportError::NotSupported("WASM WebSocket not available on non-WASM targets".to_string()))
    }

    async fn disconnect(&mut self) -> Result<(), TransportError> {
        Err(TransportError::NotSupported("WASM WebSocket not available on non-WASM targets".to_string()))
    }

    async fn send_message(&self, _message: &Message) -> Result<(), TransportError> {
        Err(TransportError::NotSupported("WASM WebSocket not available on non-WASM targets".to_string()))
    }

    fn split(self) -> (Self::Stream, Self::Sink) {
        // Return empty stream and sink for non-WASM targets
        let (sender, receiver) = mpsc::unbounded_channel::<Message>();
        drop(sender); // Close the sender immediately

        let stream = futures::stream::empty();
        let sink = WasmWebSocketSink::new(None);

        (Box::pin(stream) as Self::Stream, Box::pin(sink) as Self::Sink)
    }

    fn state(&self) -> ConnectionState {
        ConnectionState::Disconnected
    }

    async fn create_bidirectional_stream(&mut self) -> Result<(), TransportError> {
        Err(TransportError::NotSupported("WASM WebSocket not available on non-WASM targets".to_string()))
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub struct WasmWebSocketSink;

#[cfg(not(target_arch = "wasm32"))]
impl WasmWebSocketSink {
    pub fn new(_sender: Option<mpsc::UnboundedSender<Message>>) -> Self {
        Self
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Sink<Message> for WasmWebSocketSink {
    type Error = TransportError;

    fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Err(TransportError::NotSupported("WASM WebSocket not available on non-WASM targets".to_string())))
    }

    fn start_send(self: Pin<&mut Self>, _item: Message) -> Result<(), Self::Error> {
        Err(TransportError::NotSupported("WASM WebSocket not available on non-WASM targets".to_string()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Err(TransportError::NotSupported("WASM WebSocket not available on non-WASM targets".to_string())))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Err(TransportError::NotSupported("WASM WebSocket not available on non-WASM targets".to_string())))
    }
}
