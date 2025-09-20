//! WebTransport Configuration
//!
//! Configuration types for WebTransport streams and connections

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Reliability modes for WebTransport streams
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ReliabilityMode {
    BestEffort,
    Reliable,
    PartiallyReliable { max_retransmissions: u32 },
}

/// Ordering modes for WebTransport streams
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum OrderingMode {
    Unordered,
    Ordered,
    PartiallyOrdered { max_gap: u32 },
}

/// Congestion control modes
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum CongestionControl {
    Default,
    Conservative,
    Aggressive,
    Adaptive,
}

/// Stream configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StreamConfig {
    pub stream_id: u32,
    pub reliability: ReliabilityMode,
    pub ordering: OrderingMode,
    pub congestion_control: CongestionControl,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            stream_id: 0,
            reliability: ReliabilityMode::Reliable,
            ordering: OrderingMode::Ordered,
            congestion_control: CongestionControl::Default,
        }
    }
}

/// Performance metrics for WebTransport
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub packets_sent: u64,
    pub packets_received: u64,
    pub packets_lost: u64,
    pub round_trip_time: Duration,
    pub jitter: Duration,
    pub bandwidth: f64,
    pub active_streams: u32,
    pub total_streams: u32,
    pub connection_count: u32,
    pub message_count: u64,
    pub error_count: u32,
    pub connection_attempts: u64,
    pub successful_connections: u64,
    pub failed_connections: u64,
    pub messages_sent: u64,
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            bytes_sent: 0,
            bytes_received: 0,
            packets_sent: 0,
            packets_received: 0,
            packets_lost: 0,
            round_trip_time: Duration::from_millis(50),
            jitter: Duration::from_millis(5),
            bandwidth: 0.0,
            active_streams: 0,
            total_streams: 0,
            connection_count: 0,
            message_count: 0,
            error_count: 0,
            connection_attempts: 0,
            successful_connections: 0,
            failed_connections: 0,
            messages_sent: 0,
        }
    }
}
