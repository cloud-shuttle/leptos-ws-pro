//! WebTransport Implementation
//!
//! Modern HTTP/3-based transport with advanced features

pub mod config;
pub mod core;
pub mod transport_impl;
pub mod sink;
pub mod stream;

// Re-export main types for backward compatibility
pub use config::{
    CongestionControl, OrderingMode, PerformanceMetrics, ReliabilityMode, StreamConfig,
};
pub use core::{WebTransportConnection, ConnectionStats};
pub use transport_impl::{TransportInfo, TransportCapabilities, ConnectionDiagnostics};
pub use sink::{WebTransportSink, AdvancedWebTransportSink, CompressedWebTransportSink, SinkFactory};
pub use stream::AdvancedWebTransportStream;

// Legacy compatibility module
pub mod connection {
    //! Legacy compatibility module
    pub use super::core::*;
    pub use super::transport_impl::*;
}
