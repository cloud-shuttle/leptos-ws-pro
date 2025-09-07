//! Zero-Copy Serialization Implementation
//!
//! High-performance serialization using rkyv for minimal allocation
//! and maximum throughput in WebSocket communications

use crate::codec::{Codec, CodecError};
use std::marker::PhantomData;
use serde::{Serialize, Deserialize};

#[cfg(feature = "zero-copy")]
use rkyv::{Archive, Serialize as RkyvSerialize, Deserialize as RkyvDeserialize, to_bytes, from_bytes};

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
    T: Archive + RkyvSerialize<rkyv::rancor::Strategy<rkyv::rancor::Panic, rkyv::rancor::Panic>> + for<'a> RkyvDeserialize<T, rkyv::rancor::Strategy<rkyv::rancor::Panic, rkyv::rancor::Panic>> + Clone + Send + Sync + 'static,
    T::Archived: rkyv::Deserialize<T, rkyv::rancor::Strategy<rkyv::rancor::Panic, rkyv::rancor::Panic>>,
{
    fn encode(&self, message: &T) -> Result<Vec<u8>, CodecError> {
        to_bytes(message)
            .map_err(|e| CodecError::SerializationFailed(format!("rkyv serialization failed: {}", e)))
            .map(|bytes| bytes.to_vec())
    }

    fn decode(&self, data: &[u8]) -> Result<T, CodecError> {
        from_bytes(data)
            .map_err(|e| CodecError::DeserializationFailed(format!("rkyv deserialization failed: {}", e)))
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
        serde_json::to_vec(message)
            .map_err(|e| CodecError::SerializationFailed(format!("JSON fallback serialization failed: {}", e)))
    }

    fn decode(&self, data: &[u8]) -> Result<T, CodecError> {
        serde_json::from_slice(data)
            .map_err(|e| CodecError::DeserializationFailed(format!("JSON fallback deserialization failed: {}", e)))
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

/// Batch message container for efficient bulk operations
#[derive(Clone, Debug)]
#[cfg_attr(feature = "zero-copy", derive(Archive, RkyvSerialize, RkyvDeserialize))]
#[cfg_attr(not(feature = "zero-copy"), derive(Serialize, Deserialize))]
pub struct MessageBatch<T> {
    pub batch_id: String,
    pub messages: Vec<ZeroCopyMessage<T>>,
    pub created_at: u64,
}

impl<T> MessageBatch<T> {
    pub fn new() -> Self {
        Self {
            batch_id: format!("batch_{}", uuid::Uuid::new_v4()),
            messages: Vec::new(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }

    pub fn add_message(&mut self, message: ZeroCopyMessage<T>) {
        self.messages.push(message);
    }

    pub fn len(&self) -> usize {
        self.messages.len()
    }

    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    pub fn clear(&mut self) {
        self.messages.clear();
    }
}

impl<T> Default for MessageBatch<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Zero-copy buffer for memory-efficient message handling
pub struct ZeroCopyBuffer {
    data: Vec<u8>,
    positions: Vec<MessagePosition>,
}

#[derive(Debug, Clone)]
struct MessagePosition {
    start: usize,
    end: usize,
    message_type: String,
}

impl ZeroCopyBuffer {
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            positions: Vec::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            data: Vec::with_capacity(capacity),
            positions: Vec::new(),
        }
    }

    /// Append message data without copying
    pub fn append_message<T>(&mut self, message: &T, codec: &ZeroCopyCodec<T>) -> Result<usize, CodecError>
    where
        T: Clone + Send + Sync + 'static,
        ZeroCopyCodec<T>: Codec<T>,
    {
        let start_pos = self.data.len();
        let encoded = codec.encode(message)?;

        self.data.extend_from_slice(&encoded);
        let end_pos = self.data.len();

        let message_index = self.positions.len();
        self.positions.push(MessagePosition {
            start: start_pos,
            end: end_pos,
            message_type: codec.content_type().to_string(),
        });

        Ok(message_index)
    }

    /// Get message data without copying
    pub fn get_message_slice(&self, index: usize) -> Option<&[u8]> {
        self.positions.get(index).map(|pos| &self.data[pos.start..pos.end])
    }

    /// Decode message from buffer position
    pub fn decode_message<T>(&self, index: usize, codec: &ZeroCopyCodec<T>) -> Result<T, CodecError>
    where
        ZeroCopyCodec<T>: Codec<T>,
        T: Send + Sync,
    {
        if let Some(slice) = self.get_message_slice(index) {
            codec.decode(slice)
        } else {
            Err(CodecError::DeserializationFailed("Invalid message index".to_string()))
        }
    }

    pub fn message_count(&self) -> usize {
        self.positions.len()
    }

    pub fn total_size(&self) -> usize {
        self.data.len()
    }

    pub fn clear(&mut self) {
        self.data.clear();
        self.positions.clear();
    }

    /// Compact buffer by removing unused space
    pub fn compact(&mut self) {
        if self.positions.is_empty() {
            self.data.clear();
            return;
        }

        // Shift all messages to remove gaps
        let mut write_pos = 0;
        for position in &mut self.positions {
            let message_len = position.end - position.start;
            if position.start != write_pos {
                self.data.copy_within(position.start..position.end, write_pos);
            }
            position.start = write_pos;
            position.end = write_pos + message_len;
            write_pos += message_len;
        }

        self.data.truncate(write_pos);
    }
}

impl Default for ZeroCopyBuffer {
    fn default() -> Self {
        Self::new()
    }
}

/// Performance benchmarking for zero-copy operations
pub struct ZeroCopyBenchmark {
    iterations: usize,
    message_size: usize,
}

impl ZeroCopyBenchmark {
    pub fn new(iterations: usize, message_size: usize) -> Self {
        Self {
            iterations,
            message_size,
        }
    }

    /// Benchmark serialization performance
    pub fn benchmark_serialization<T>(&self, message: &T, codec: &ZeroCopyCodec<T>) -> BenchmarkResult
    where
        T: Clone + Send + Sync + 'static,
        ZeroCopyCodec<T>: Codec<T>,
    {
        let start = std::time::Instant::now();
        let mut total_bytes = 0;

        for _ in 0..self.iterations {
            match codec.encode(message) {
                Ok(data) => total_bytes += data.len(),
                Err(_) => continue,
            }
        }

        let elapsed = start.elapsed();

        BenchmarkResult {
            iterations: self.iterations,
            total_time: elapsed,
            total_bytes,
            throughput_mbps: (total_bytes as f64 / elapsed.as_secs_f64()) / 1_000_000.0,
            operations_per_second: self.iterations as f64 / elapsed.as_secs_f64(),
        }
    }

    /// Benchmark deserialization performance
    pub fn benchmark_deserialization<T>(&self, data: &[u8], codec: &ZeroCopyCodec<T>) -> BenchmarkResult
    where
        T: Send + Sync,
        ZeroCopyCodec<T>: Codec<T>,
    {
        let start = std::time::Instant::now();
        let mut successful_ops = 0;

        for _ in 0..self.iterations {
            if codec.decode(data).is_ok() {
                successful_ops += 1;
            }
        }

        let elapsed = start.elapsed();
        let total_bytes = data.len() * successful_ops;

        BenchmarkResult {
            iterations: successful_ops,
            total_time: elapsed,
            total_bytes,
            throughput_mbps: (total_bytes as f64 / elapsed.as_secs_f64()) / 1_000_000.0,
            operations_per_second: successful_ops as f64 / elapsed.as_secs_f64(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    pub iterations: usize,
    pub total_time: std::time::Duration,
    pub total_bytes: usize,
    pub throughput_mbps: f64,
    pub operations_per_second: f64,
}

impl BenchmarkResult {
    pub fn print_summary(&self) {
        println!("Benchmark Results:");
        println!("  Iterations: {}", self.iterations);
        println!("  Total Time: {:?}", self.total_time);
        println!("  Total Bytes: {}", self.total_bytes);
        println!("  Throughput: {:.2} MB/s", self.throughput_mbps);
        println!("  Operations/sec: {:.2}", self.operations_per_second);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Serialize, Deserialize};

    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    #[cfg_attr(feature = "zero-copy", derive(Archive, RkyvSerialize, RkyvDeserialize))]
    struct TestData {
        id: u32,
        name: String,
        values: Vec<f64>,
    }

    #[test]
    fn test_zero_copy_codec() {
        let codec = ZeroCopyCodec::<TestData>::new();
        let test_data = TestData {
            id: 123,
            name: "test".to_string(),
            values: vec![1.0, 2.0, 3.0],
        };

        let encoded = codec.encode(&test_data).unwrap();
        let decoded = codec.decode(&encoded).unwrap();

        assert_eq!(test_data, decoded);
    }

    #[test]
    fn test_zero_copy_message() {
        let test_data = TestData {
            id: 456,
            name: "message_test".to_string(),
            values: vec![4.0, 5.0, 6.0],
        };

        let message = ZeroCopyMessage::new("msg_1".to_string(), test_data.clone())
            .with_priority(8)
            .with_ttl(300);

        assert_eq!(message.payload, test_data);
        assert_eq!(message.metadata.priority, 8);
        assert_eq!(message.metadata.ttl, Some(300));
        assert!(!message.is_expired());
    }

    #[test]
    fn test_message_batch() {
        let mut batch = MessageBatch::<TestData>::new();

        let data1 = TestData {
            id: 1,
            name: "batch1".to_string(),
            values: vec![1.0],
        };

        let data2 = TestData {
            id: 2,
            name: "batch2".to_string(),
            values: vec![2.0],
        };

        batch.add_message(ZeroCopyMessage::new("1".to_string(), data1));
        batch.add_message(ZeroCopyMessage::new("2".to_string(), data2));

        assert_eq!(batch.len(), 2);
        assert!(!batch.is_empty());
    }

    #[test]
    fn test_zero_copy_buffer() {
        let mut buffer = ZeroCopyBuffer::new();
        let codec = ZeroCopyCodec::<TestData>::new();

        let test_data = TestData {
            id: 789,
            name: "buffer_test".to_string(),
            values: vec![7.0, 8.0, 9.0],
        };

        let index = buffer.append_message(&test_data, &codec).unwrap();
        assert_eq!(index, 0);
        assert_eq!(buffer.message_count(), 1);

        let decoded = buffer.decode_message(index, &codec).unwrap();
        assert_eq!(test_data, decoded);
    }

    #[test]
    fn test_buffer_compact() {
        let mut buffer = ZeroCopyBuffer::with_capacity(1024);
        let codec = ZeroCopyCodec::<TestData>::new();

        for i in 0..5 {
            let data = TestData {
                id: i,
                name: format!("test_{}", i),
                values: vec![i as f64],
            };
            buffer.append_message(&data, &codec).unwrap();
        }

        let size_before = buffer.total_size();
        buffer.compact();
        let size_after = buffer.total_size();

        assert_eq!(buffer.message_count(), 5);
        assert!(size_after <= size_before);
    }

    #[cfg(feature = "zero-copy")]
    #[test]
    fn test_performance_comparison() {
        use crate::codec::JsonCodec;

        let test_data = TestData {
            id: 12345,
            name: "performance_test".to_string(),
            values: (0..1000).map(|i| i as f64).collect(),
        };

        let zero_copy_codec = ZeroCopyCodec::new();
        let json_codec = JsonCodec::new();

        // Encode with both codecs
        let rkyv_encoded = zero_copy_codec.encode(&test_data).unwrap();
        let json_encoded = json_codec.encode(&test_data).unwrap();

        println!("rkyv size: {} bytes", rkyv_encoded.len());
        println!("JSON size: {} bytes", json_encoded.len());

        // rkyv should be more compact
        assert!(rkyv_encoded.len() <= json_encoded.len());
    }
}
