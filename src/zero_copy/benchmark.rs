//! Zero-Copy Benchmarking
//!
//! Performance benchmarking for zero-copy operations

use std::time::{Duration, Instant};

/// Zero-copy benchmark for performance testing
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

    pub fn run_serialization_benchmark<T>(&self, data: &T) -> BenchmarkResult
    where
        T: serde::Serialize,
    {
        let start = Instant::now();

        for _ in 0..self.iterations {
            let _ = serde_json::to_vec(data);
        }

        let duration = start.elapsed();

        BenchmarkResult {
            operation: "serialization".to_string(),
            iterations: self.iterations,
            total_time: duration,
            average_time: duration / self.iterations as u32,
            throughput: self.iterations as f64 / duration.as_secs_f64(),
        }
    }

    pub fn run_deserialization_benchmark<T>(&self, data: &[u8]) -> BenchmarkResult
    where
        T: for<'de> serde::Deserialize<'de>,
    {
        let start = Instant::now();

        for _ in 0..self.iterations {
            let _: T = serde_json::from_slice(data).unwrap();
        }

        let duration = start.elapsed();

        BenchmarkResult {
            operation: "deserialization".to_string(),
            iterations: self.iterations,
            total_time: duration,
            average_time: duration / self.iterations as u32,
            throughput: self.iterations as f64 / duration.as_secs_f64(),
        }
    }
}

/// Benchmark result
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    pub operation: String,
    pub iterations: usize,
    pub total_time: Duration,
    pub average_time: Duration,
    pub throughput: f64,
}
