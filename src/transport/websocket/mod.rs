//! WebSocket transport implementations
//!
//! This module provides WebSocket functionality for both native and WASM targets.

pub mod wasm;
pub mod native;

// Re-export the WASM WebSocket implementation
pub use wasm::WasmWebSocketConnection;

// Re-export the native WebSocket implementation
pub use native::WebSocketConnection;

// For non-WASM targets, we'll use the existing native WebSocket implementation
// For WASM targets, we'll use the WASM implementation
#[cfg(target_arch = "wasm32")]
pub type WebSocketTransport = WasmWebSocketConnection;

#[cfg(not(target_arch = "wasm32"))]
pub type WebSocketTransport = WebSocketConnection;
