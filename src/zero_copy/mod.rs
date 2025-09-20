//! Zero-Copy Serialization
//!
//! High-performance zero-copy serialization using rkyv

pub mod benchmark;
pub mod buffer;
pub mod codec;

// Re-export main types
pub use benchmark::{BenchmarkResult, ZeroCopyBenchmark};
pub use buffer::{MessageBatch, ZeroCopyBuffer};
pub use codec::{MessageMetadata, ZeroCopyCodec, ZeroCopyMessage};
