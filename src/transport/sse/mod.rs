//! Server-Sent Events (SSE) Transport
//!
//! HTTP-based streaming transport with automatic reconnection
//!
//! This module provides a complete SSE implementation broken down into focused components:
//! - `client`: Client-side SSE connection handling
//! - `server`: Server-side SSE broadcasting
//! - `events`: Event parsing, creation, and filtering
//! - `reconnect`: Reconnection strategies and health monitoring
//! - `config`: Configuration types and settings

pub mod client;
pub mod config;
pub mod connection;
pub mod events;
pub mod reconnect;
pub mod server;

// Re-export main types for backward compatibility
pub use config::{HeartbeatConfig, ReconnectionStrategy};
pub use connection::SseConnection;
pub use events::{HeartbeatEvent, SseEvent};

// Re-export new modular types
pub use client::SseClient;
pub use reconnect::{ConnectionHealthMonitor, ExponentialBackoff, LinearBackoff, ReconnectionManager};
pub use server::{SseEventBuilder, SseServer};
