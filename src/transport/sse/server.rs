//! SSE Server Implementation
//!
//! Server-side Server-Sent Events handling for broadcasting events to clients

use crate::transport::{ConnectionState, Message, MessageType, TransportError};
use axum::{
    extract::State,
    response::{sse::Event, Sse},
    routing::get,
    Router,
};
use futures::stream::{self, Stream, StreamExt};
use std::{
    collections::HashMap,
    convert::Infallible,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::broadcast;

/// SSE Server for broadcasting events to connected clients
pub struct SseServer {
    state: Arc<Mutex<ConnectionState>>,
    clients: Arc<Mutex<HashMap<String, broadcast::Sender<Message>>>>,
    event_broadcaster: broadcast::Sender<Message>,
    heartbeat_interval: Duration,
}

impl SseServer {
    /// Create a new SSE server
    pub fn new(heartbeat_interval: Duration) -> Self {
        let (event_broadcaster, _) = broadcast::channel(1000);

        Self {
            state: Arc::new(Mutex::new(ConnectionState::Disconnected)),
            clients: Arc::new(Mutex::new(HashMap::new())),
            event_broadcaster,
            heartbeat_interval,
        }
    }

    /// Start the server
    pub async fn start(&mut self, _port: u16) -> Result<(), TransportError> {
        *self.state.lock().unwrap() = ConnectionState::Connected;

        // Start heartbeat task
        self.start_heartbeat_task().await;

        Ok(())
    }

    /// Stop the server
    pub async fn stop(&mut self) -> Result<(), TransportError> {
        *self.state.lock().unwrap() = ConnectionState::Disconnected;
        Ok(())
    }

    /// Broadcast a message to all connected clients
    pub async fn broadcast(&self, message: Message) -> Result<(), TransportError> {
        let _ = self.event_broadcaster.send(message);
        Ok(())
    }

    /// Get the number of connected clients
    pub fn client_count(&self) -> usize {
        self.clients.lock().unwrap().len()
    }

    /// Start heartbeat task to keep connections alive
    async fn start_heartbeat_task(&self) {
        let broadcaster = self.event_broadcaster.clone();
        let interval = self.heartbeat_interval;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(interval);
            loop {
                interval.tick().await;
                let heartbeat = Message {
                    data: b"ping".to_vec(),
                    message_type: MessageType::Text,
                };
                let _ = broadcaster.send(heartbeat);
            }
        });
    }

    /// Create SSE stream for a client
    pub fn create_client_stream(&self, client_id: String) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
        let receiver = self.event_broadcaster.subscribe();
        let stream = stream::unfold(receiver, |mut rx| async move {
            match rx.recv().await {
                Ok(message) => Some((Ok(Event::default().data(String::from_utf8_lossy(&message.data))), rx)),
                Err(_) => None,
            }
        })
        .chain(stream::once(async { Ok(Event::default().data("connection_closed")) }));

        // Register client
        {
            let mut clients = self.clients.lock().unwrap();
            clients.insert(client_id, self.event_broadcaster.clone());
        }

        Sse::new(stream)
            .keep_alive(
                axum::response::sse::KeepAlive::new()
                    .interval(Duration::from_secs(15))
                    .text("keep-alive"),
            )
    }

    /// Remove a client
    pub fn remove_client(&self, client_id: &str) {
        let mut clients = self.clients.lock().unwrap();
        clients.remove(client_id);
    }

    /// Get server state
    pub fn state(&self) -> ConnectionState {
        *self.state.lock().unwrap()
    }
}

/// SSE handler for Axum integration
pub async fn sse_handler(
    State(server): State<Arc<SseServer>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let client_id = uuid::Uuid::new_v4().to_string();
    server.create_client_stream(client_id)
}

/// Create Axum router for SSE endpoints
pub fn create_sse_router(server: Arc<SseServer>) -> Router {
    Router::new()
        .route("/events", get(sse_handler))
        .with_state(server)
}

/// SSE Event builder for server-side event creation
pub struct SseEventBuilder {
    event_type: Option<String>,
    data: String,
    id: Option<String>,
    retry: Option<u64>,
}

impl SseEventBuilder {
    pub fn new() -> Self {
        Self {
            event_type: None,
            data: String::new(),
            id: None,
            retry: None,
        }
    }

    pub fn event_type(mut self, event_type: &str) -> Self {
        self.event_type = Some(event_type.to_string());
        self
    }

    pub fn data(mut self, data: &str) -> Self {
        self.data = data.to_string();
        self
    }

    pub fn id(mut self, id: &str) -> Self {
        self.id = Some(id.to_string());
        self
    }

    pub fn retry(mut self, retry: u64) -> Self {
        self.retry = Some(retry);
        self
    }

    pub fn build(self) -> String {
        let mut event = String::new();

        if let Some(event_type) = self.event_type {
            event.push_str(&format!("event: {}\n", event_type));
        }

        if let Some(id) = self.id {
            event.push_str(&format!("id: {}\n", id));
        }

        if let Some(retry) = self.retry {
            event.push_str(&format!("retry: {}\n", retry));
        }

        // Handle multi-line data
        for line in self.data.lines() {
            event.push_str(&format!("data: {}\n", line));
        }

        event.push('\n');
        event
    }
}

impl Default for SseEventBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sse_server_creation() {
        let server = SseServer::new(Duration::from_secs(30));
        assert_eq!(server.state(), ConnectionState::Disconnected);
        assert_eq!(server.client_count(), 0);
    }

    #[tokio::test]
    async fn test_sse_server_start_stop() {
        let mut server = SseServer::new(Duration::from_secs(30));

        server.start(8080).await.unwrap();
        assert_eq!(server.state(), ConnectionState::Connected);

        server.stop().await.unwrap();
        assert_eq!(server.state(), ConnectionState::Disconnected);
    }

    #[test]
    fn test_sse_event_builder() {
        let event = SseEventBuilder::new()
            .event_type("message")
            .data("Hello World")
            .id("123")
            .retry(5000)
            .build();

        assert!(event.contains("event: message"));
        assert!(event.contains("data: Hello World"));
        assert!(event.contains("id: 123"));
        assert!(event.contains("retry: 5000"));
    }

    #[test]
    fn test_sse_event_builder_multiline() {
        let event = SseEventBuilder::new()
            .event_type("message")
            .data("Line 1\nLine 2")
            .build();

        assert!(event.contains("event: message"));
        assert!(event.contains("data: Line 1"));
        assert!(event.contains("data: Line 2"));
    }
}
