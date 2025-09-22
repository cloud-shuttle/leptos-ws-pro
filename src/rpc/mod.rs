//! RPC (Remote Procedure Call) System
//!
//! Provides type-safe RPC communication over WebSocket connections
//! with support for request/response patterns and streaming

pub mod advanced;
pub mod client;
pub mod correlation;
pub mod types;

// Re-export main types
pub use client::{RpcClient, RpcSubscription, reset_rpc_id_counter};
pub use types::*;

// Re-export advanced RPC types
#[cfg(feature = "advanced-rpc")]
pub use advanced::*;
