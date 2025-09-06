//! Codec module for encoding and decoding WebSocket messages
//!
//! This module provides a simple JSON-based codec system for WebSocket messages.
//! Future versions will include zero-copy serialization with rkyv and compression.

use serde::{Deserialize as SerdeDeserialize, Serialize as SerdeSerialize};
use thiserror::Error;

/// Trait for encoding and decoding messages
pub trait Codec<T>: Send + Sync
where
    T: Send + Sync,
{
    /// Encode a message to bytes
    fn encode(&self, message: &T) -> Result<Vec<u8>, CodecError>;

    /// Decode bytes to a message
    fn decode(&self, data: &[u8]) -> Result<T, CodecError>;

    /// Get the content type for this codec
    fn content_type(&self) -> &'static str;
}

/// Codec errors
#[derive(Debug, Error)]
pub enum CodecError {
    #[error("Serialization failed: {0}")]
    SerializationFailed(String),

    #[error("Deserialization failed: {0}")]
    DeserializationFailed(String),

    #[error("Compression failed: {0}")]
    CompressionFailed(String),

    #[error("Decompression failed: {0}")]
    DecompressionFailed(String),
}

/// JSON codec using serde
pub struct JsonCodec;

impl JsonCodec {
    pub fn new() -> Self {
        Self
    }
}

impl<T> Codec<T> for JsonCodec
where
    T: SerdeSerialize + for<'de> SerdeDeserialize<'de> + Clone + Send + Sync,
{
    fn encode(&self, message: &T) -> Result<Vec<u8>, CodecError> {
        serde_json::to_vec(message).map_err(|e| CodecError::SerializationFailed(e.to_string()))
    }

    fn decode(&self, data: &[u8]) -> Result<T, CodecError> {
        serde_json::from_slice(data).map_err(|e| CodecError::DeserializationFailed(e.to_string()))
    }

    fn content_type(&self) -> &'static str {
        "application/json"
    }
}

/// Placeholder for rkyv codec (simplified for now)
pub struct RkyvCodec;

impl RkyvCodec {
    pub fn new() -> Self {
        Self
    }
}

impl<T> Codec<T> for RkyvCodec
where
    T: SerdeSerialize + for<'de> SerdeDeserialize<'de> + Clone + Send + Sync,
{
    fn encode(&self, message: &T) -> Result<Vec<u8>, CodecError> {
        // For now, just use JSON serialization
        serde_json::to_vec(message).map_err(|e| CodecError::SerializationFailed(e.to_string()))
    }

    fn decode(&self, data: &[u8]) -> Result<T, CodecError> {
        // For now, just use JSON deserialization
        serde_json::from_slice(data).map_err(|e| CodecError::DeserializationFailed(e.to_string()))
    }

    fn content_type(&self) -> &'static str {
        "application/rkyv"
    }
}

/// Hybrid codec that tries rkyv first, falls back to JSON
pub struct HybridCodec {
    rkyv_codec: RkyvCodec,
    json_codec: JsonCodec,
}

impl HybridCodec {
    pub fn new() -> Result<Self, CodecError> {
        Ok(Self {
            rkyv_codec: RkyvCodec::new(),
            json_codec: JsonCodec::new(),
        })
    }
}

impl<T> Codec<T> for HybridCodec
where
    T: SerdeSerialize + for<'de> SerdeDeserialize<'de> + Clone + Send + Sync,
{
    fn encode(&self, message: &T) -> Result<Vec<u8>, CodecError> {
        // Try rkyv first for performance
        match self.rkyv_codec.encode(message) {
            Ok(data) => Ok(data),
            Err(_) => {
                // Fall back to JSON
                self.json_codec.encode(message)
            }
        }
    }

    fn decode(&self, data: &[u8]) -> Result<T, CodecError> {
        // Try JSON first (simpler for now)
        match self.json_codec.decode(data) {
            Ok(result) => Ok(result),
            Err(_) => {
                // Fall back to rkyv
                match self.rkyv_codec.decode(data) {
                    Ok(result) => Ok(result),
                    Err(_e) => {
                        // If both fail, return the JSON error
                        self.json_codec.decode(data)
                    }
                }
            }
        }
    }

    fn content_type(&self) -> &'static str {
        "application/hybrid"
    }
}

/// Wrapper for WebSocket messages with type information
#[derive(Debug, Clone, SerdeSerialize, SerdeDeserialize)]
pub struct WsMessage<T> {
    pub data: T,
}

impl<T> WsMessage<T> {
    pub fn new(data: T) -> Self {
        Self { data }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestMessage {
        id: u32,
        content: String,
    }

    #[test]
    fn test_json_codec_basic() {
        let codec = JsonCodec::new();
        let message = TestMessage {
            id: 42,
            content: "Hello, World!".to_string(),
        };

        let encoded = codec.encode(&message).unwrap();
        let decoded = codec.decode(&encoded).unwrap();

        assert_eq!(message, decoded);
    }

    #[test]
    fn test_rkyv_codec_basic() {
        let codec = RkyvCodec::new();
        let message = TestMessage {
            id: 42,
            content: "Hello, World!".to_string(),
        };

        let encoded = codec.encode(&message).unwrap();
        let decoded = codec.decode(&encoded).unwrap();

        assert_eq!(message, decoded);
    }

    #[test]
    fn test_hybrid_codec_basic() {
        let codec = HybridCodec::new().unwrap();
        let message = TestMessage {
            id: 42,
            content: "Hello, World!".to_string(),
        };

        let encoded = codec.encode(&message).unwrap();
        let decoded = codec.decode(&encoded).unwrap();

        assert_eq!(message, decoded);
    }

    #[test]
    fn test_ws_message_wrapper() {
        let test_data = TestMessage {
            id: 42,
            content: "test".to_string(),
        };

        let ws_message = WsMessage::new(test_data.clone());
        assert_eq!(ws_message.data, test_data);
    }

    #[test]
    fn test_ws_message_serialization() {
        let test_data = TestMessage {
            id: 42,
            content: "test".to_string(),
        };

        let ws_message = WsMessage::new(test_data.clone());

        // Test JSON serialization
        let json_encoded = serde_json::to_string(&ws_message).unwrap();
        let json_decoded: WsMessage<TestMessage> = serde_json::from_str(&json_encoded).unwrap();
        assert_eq!(ws_message.data, json_decoded.data);
    }
}
