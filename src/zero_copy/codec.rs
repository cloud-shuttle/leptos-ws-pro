//! Zero-Copy Codec
//!
//! High-performance serialization using rkyv

use crate::codec::{Codec, CodecError};
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

#[cfg(feature = "zero-copy")]
use rkyv::{
    from_bytes, to_bytes, Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize,
};

/// Zero-copy codec using rkyv serialization
pub struct ZeroCopyCodec<T> {
    _phantom: PhantomData<T>,
}

impl<T> ZeroCopyCodec<T> {
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<T> Default for ZeroCopyCodec<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "zero-copy")]
impl<T> Codec<T> for ZeroCopyCodec<T>
where
    T: Archive
        + RkyvSerialize<rkyv::rancor::Strategy<rkyv::rancor::Panic, rkyv::rancor::Panic>>
        + for<'a> RkyvDeserialize<T, rkyv::rancor::Strategy<rkyv::rancor::Panic, rkyv::rancor::Panic>>
        + Clone
        + Send
        + Sync
        + 'static,
    T::Archived:
        rkyv::Deserialize<T, rkyv::rancor::Strategy<rkyv::rancor::Panic, rkyv::rancor::Panic>>,
{
    fn encode(&self, message: &T) -> Result<Vec<u8>, CodecError> {
        to_bytes(message)
            .map_err(|e| {
                CodecError::SerializationFailed(format!("rkyv serialization failed: {}", e))
            })
            .map(|bytes| bytes.to_vec())
    }

    fn decode(&self, data: &[u8]) -> Result<T, CodecError> {
        from_bytes(data).map_err(|e| {
            CodecError::DeserializationFailed(format!("rkyv deserialization failed: {}", e))
        })
    }

    fn content_type(&self) -> &'static str {
        "application/rkyv"
    }
}

#[cfg(not(feature = "zero-copy"))]
impl<T> Codec<T> for ZeroCopyCodec<T>
where
    T: Serialize + for<'de> Deserialize<'de> + Clone + Send + Sync + 'static,
{
    fn encode(&self, message: &T) -> Result<Vec<u8>, CodecError> {
        serde_json::to_vec(message).map_err(|e| {
            CodecError::SerializationFailed(format!("JSON fallback serialization failed: {}", e))
        })
    }

    fn decode(&self, data: &[u8]) -> Result<T, CodecError> {
        serde_json::from_slice(data).map_err(|e| {
            CodecError::DeserializationFailed(format!(
                "JSON fallback deserialization failed: {}",
                e
            ))
        })
    }

    fn content_type(&self) -> &'static str {
        "application/json"
    }
}

/// High-performance message with zero-copy deserialization support
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "zero-copy", derive(Archive, RkyvSerialize, RkyvDeserialize))]
#[cfg_attr(not(feature = "zero-copy"), derive(Serialize, Deserialize))]
pub struct ZeroCopyMessage<T> {
    pub id: String,
    pub timestamp: u64,
    pub payload: T,
    pub metadata: MessageMetadata,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "zero-copy", derive(Archive, RkyvSerialize, RkyvDeserialize))]
#[cfg_attr(not(feature = "zero-copy"), derive(Serialize, Deserialize))]
pub struct MessageMetadata {
    pub content_type: String,
    pub compression: Option<String>,
    pub priority: u8,
    pub ttl: Option<u64>,
}

impl<T> ZeroCopyMessage<T> {
    pub fn new(id: String, payload: T) -> Self {
        Self {
            id,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            payload,
            metadata: MessageMetadata {
                content_type: "application/rkyv".to_string(),
                compression: None,
                priority: 5,
                ttl: None,
            },
        }
    }

    pub fn with_priority(mut self, priority: u8) -> Self {
        self.metadata.priority = priority;
        self
    }

    pub fn with_ttl(mut self, ttl_seconds: u64) -> Self {
        self.metadata.ttl = Some(ttl_seconds);
        self
    }

    pub fn is_expired(&self) -> bool {
        if let Some(ttl) = self.metadata.ttl {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            (self.timestamp / 1000) + ttl < now
        } else {
            false
        }
    }
}
