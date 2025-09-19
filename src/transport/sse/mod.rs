//! Server-Sent Events (SSE) Transport
//!
//! HTTP-based streaming transport with automatic reconnection

pub mod connection;
pub mod events;
pub mod config;

// Re-export main types
pub use connection::SseConnection;
pub use events::{SseEvent, HeartbeatEvent};
pub use config::{ReconnectionStrategy, HeartbeatConfig};
