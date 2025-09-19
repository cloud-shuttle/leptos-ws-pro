//! Zero-Copy Serialization
//!
//! High-performance zero-copy serialization using rkyv

pub mod codec;
pub mod buffer;
pub mod benchmark;

// Re-export main types
pub use codec::{ZeroCopyCodec, ZeroCopyMessage, MessageMetadata};
pub use buffer::{ZeroCopyBuffer, MessageBatch};
pub use benchmark::{ZeroCopyBenchmark, BenchmarkResult};
