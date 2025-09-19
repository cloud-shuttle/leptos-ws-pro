//! WebTransport Implementation
//!
//! Modern HTTP/3-based transport with advanced features

pub mod connection;
pub mod stream;
pub mod config;

// Re-export main types
pub use connection::WebTransportConnection;
pub use stream::AdvancedWebTransportStream;
pub use config::{ReliabilityMode, OrderingMode, CongestionControl, StreamConfig, PerformanceMetrics};
