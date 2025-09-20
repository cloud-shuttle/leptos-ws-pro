//! WebTransport Stream
//!
//! Advanced stream implementation for WebTransport

use crate::transport::TransportError;
use std::time::{Duration, Instant};

use super::config::{CongestionControl, OrderingMode, ReliabilityMode, StreamConfig};

/// WebTransport stream with advanced features
#[derive(Debug, Clone)]
pub struct AdvancedWebTransportStream {
    stream_id: u32,
    reliability: ReliabilityMode,
    ordering: OrderingMode,
    congestion_control: CongestionControl,
    is_active: bool,
    can_send: bool,
    can_receive: bool,
    send_latency: Duration,
    delivery_guaranteed: bool,
    max_retransmissions: u32,
    retransmission_count: u32,
    average_send_rate: f64,
    last_used: Instant,
}

impl AdvancedWebTransportStream {
    pub fn new(stream_id: u32, config: StreamConfig) -> Self {
        Self {
            stream_id,
            reliability: config.reliability,
            ordering: config.ordering,
            congestion_control: config.congestion_control,
            is_active: true,
            can_send: true,
            can_receive: true,
            send_latency: Duration::from_millis(10),
            delivery_guaranteed: matches!(config.reliability, ReliabilityMode::Reliable),
            max_retransmissions: match config.reliability {
                ReliabilityMode::PartiallyReliable {
                    max_retransmissions,
                } => max_retransmissions,
                _ => 0,
            },
            retransmission_count: 0,
            average_send_rate: 0.0,
            last_used: Instant::now(),
        }
    }

    pub fn stream_id(&self) -> u32 {
        self.stream_id
    }

    pub fn reliability_mode(&self) -> ReliabilityMode {
        self.reliability
    }

    pub fn ordering_mode(&self) -> OrderingMode {
        self.ordering
    }

    pub fn is_active(&self) -> bool {
        self.is_active
    }

    pub fn can_send(&self) -> bool {
        self.can_send
    }

    pub fn can_receive(&self) -> bool {
        self.can_receive
    }

    pub async fn send_data<T: serde::Serialize>(
        &mut self,
        _data: &T,
    ) -> Result<(), TransportError> {
        // Simulate sending data
        self.last_used = Instant::now();
        Ok(())
    }

    pub async fn send_latency(&self) -> Duration {
        self.send_latency
    }

    pub fn is_delivery_guaranteed(&self) -> bool {
        self.delivery_guaranteed
    }

    pub async fn acknowledgment_received(&self) -> bool {
        // Simulate acknowledgment
        true
    }

    pub fn max_retransmissions(&self) -> u32 {
        self.max_retransmissions
    }

    pub async fn retransmission_count(&self) -> u32 {
        self.retransmission_count
    }

    pub async fn average_send_rate(&self) -> f64 {
        self.average_send_rate
    }
}
