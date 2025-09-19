//! RPC (Remote Procedure Call) System
//!
//! Provides type-safe RPC communication over WebSocket connections
//! with support for request/response patterns and streaming

pub mod advanced;
pub mod correlation;
pub mod types;
pub mod client;

// Re-export main types
pub use types::*;
pub use client::{RpcClient, RpcSubscription};

// Re-export advanced RPC types
#[cfg(feature = "advanced-rpc")]
pub use advanced::*;
