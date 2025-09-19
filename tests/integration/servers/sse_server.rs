//! SSE Test Server for Integration Testing
//!
//! This module provides a simple SSE server that can be used
//! for integration testing of the SSE transport system.

use axum::{
    extract::State,
    http::StatusCode,
    response::{Response, Sse},
    routing::get,
    Router,
};
use axum::response::sse::{Event, KeepAlive};
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::interval;

/// SSE Server state
#[derive(Clone)]
pub struct SseServerState {
    pub event_count: Arc<Mutex<u64>>,
    pub is_running: Arc<Mutex<bool>>,
}

impl SseServerState {
    pub fn new() -> Self {
        Self {
            event_count: Arc::new(Mutex::new(0)),
            is_running: Arc::new(Mutex::new(false)),
        }
    }
}

/// SSE Server
pub struct SseServer {
    port: u16,
    state: SseServerState,
}

impl SseServer {
    /// Create a new SSE server
    pub fn new(port: u16) -> Self {
        Self {
            port,
            state: SseServerState::new(),
        }
    }

    /// Start the SSE server
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        let state = self.state.clone();
        *state.is_running.lock().await = true;

        let app = Router::new()
            .route("/events", get(sse_handler))
            .route("/heartbeat", get(heartbeat_handler))
            .route("/test", get(test_handler))
            .with_state(state);

        let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", self.port)).await?;
        println!("SSE server starting on port {}", self.port);

        axum::serve(listener, app).await?;
        Ok(())
    }

    /// Get the server URL
    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    /// Get the events endpoint URL
    pub fn events_url(&self) -> String {
        format!("http://127.0.0.1:{}/events", self.port)
    }

    /// Stop the server
    pub async fn stop(&self) {
        *self.state.is_running.lock().await = false;
    }
}

/// SSE event handler
async fn sse_handler(State(state): State<SseServerState>) -> Sse<impl futures::Stream<Item = Result<Event, Infallible>>> {
    let stream = async_stream::stream! {
        let mut interval = interval(Duration::from_secs(1));
        let mut counter = 0;

        while *state.is_running.lock().await {
            interval.tick().await;
            counter += 1;
            *state.event_count.lock().await = counter;

            let event_data = format!("Event {}", counter);
            let event = Event::default()
                .event("message")
                .data(event_data)
                .id(counter.to_string());

            yield Ok(event);
        }
    };

    Sse::new(stream).keep_alive(KeepAlive::default())
}

/// Heartbeat handler
async fn heartbeat_handler(State(state): State<SseServerState>) -> Sse<impl futures::Stream<Item = Result<Event, Infallible>>> {
    let stream = async_stream::stream! {
        let mut interval = interval(Duration::from_secs(5));

        while *state.is_running.lock().await {
            interval.tick().await;

            let heartbeat_data = format!("ping_{}", std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs());

            let event = Event::default()
                .event("heartbeat")
                .data(heartbeat_data)
                .id("heartbeat".to_string());

            yield Ok(event);
        }
    };

    Sse::new(stream).keep_alive(KeepAlive::default())
}

/// Test handler for simple events
async fn test_handler(State(state): State<SseServerState>) -> Sse<impl futures::Stream<Item = Result<Event, Infallible>>> {
    let stream = async_stream::stream! {
        let events = vec![
            ("test", "Hello, SSE!"),
            ("test", "This is a test event"),
            ("test", "SSE is working!"),
        ];

        for (event_type, data) in events {
            let event = Event::default()
                .event(event_type)
                .data(data)
                .id("test".to_string());

            yield Ok(event);

            // Small delay between events
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    };

    Sse::new(stream).keep_alive(KeepAlive::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sse_server_creation() {
        let server = SseServer::new(8080);
        assert_eq!(server.url(), "http://127.0.0.1:8080");
        assert_eq!(server.events_url(), "http://127.0.0.1:8080/events");
    }

    #[tokio::test]
    async fn test_sse_server_state() {
        let state = SseServerState::new();
        assert_eq!(*state.event_count.lock().await, 0);
        assert!(!*state.is_running.lock().await);
    }
}
