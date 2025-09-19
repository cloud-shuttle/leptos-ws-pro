//! Zero-Copy Buffer
//!
//! Memory-efficient buffer for zero-copy message handling

use crate::zero_copy::codec::ZeroCopyMessage;

/// Batch message container for efficient bulk operations
#[derive(Clone, Debug)]
#[cfg_attr(feature = "zero-copy", derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize))]
#[cfg_attr(not(feature = "zero-copy"), derive(serde::Serialize, serde::Deserialize))]
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

    pub fn append(&mut self, data: &[u8], message_type: &str) {
        let start = self.data.len();
        self.data.extend_from_slice(data);
        let end = self.data.len();

        self.positions.push(MessagePosition {
            start,
            end,
            message_type: message_type.to_string(),
        });
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn read(&self, index: usize) -> Option<&[u8]> {
        if let Some(pos) = self.positions.get(index) {
            Some(&self.data[pos.start..pos.end])
        } else {
            None
        }
    }

    pub fn message_count(&self) -> usize {
        self.positions.len()
    }

    pub fn clear(&mut self) {
        self.data.clear();
        self.positions.clear();
    }

    /// Read data with optional return type
    pub fn read_optional(&self, index: usize) -> Option<&[u8]> {
        self.read(index)
    }
}

impl Default for ZeroCopyBuffer {
    fn default() -> Self {
        Self::new()
    }
}
