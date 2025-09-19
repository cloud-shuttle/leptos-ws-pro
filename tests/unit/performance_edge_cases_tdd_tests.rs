//! TDD Test Suite for Performance Edge Cases
//!
//! This test suite follows TDD principles to drive the implementation of:
//! - Memory pressure handling and garbage collection optimization
//! - CPU-intensive operations and async task scheduling
//! - Network bandwidth optimization and adaptive compression
//! - Cache eviction strategies and memory leak prevention
//! - Performance monitoring and bottleneck detection

use leptos_ws_pro::performance::{
    PerformanceManager, MessageBatcher, MessageCache, ConnectionPool,
    PerformanceProfiler, PerformanceMetrics
};
use leptos_ws_pro::transport::{Message, MessageType, ConnectionState};
use leptos_ws_pro::codec::{Codec, JsonCodec, HybridCodec};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, AtomicBool, Ordering};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct PerformanceTestData {
    id: u64,
    payload: Vec<u8>,
    timestamp: u64,
    priority: MessagePriority,
    size_category: SizeCategory,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
enum MessagePriority {
    Low,
    Normal,
    High,
    Critical,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
enum SizeCategory {
    Small,    // < 1KB
    Medium,   // 1KB - 100KB
    Large,    // 100KB - 1MB
    Huge,     // > 1MB
}

#[derive(Debug, Clone)]
struct MemoryPressureConfig {
    max_memory_usage: usize,
    gc_threshold: f64,
    eviction_policy: EvictionPolicy,
    compression_threshold: usize,
}

#[derive(Debug, Clone)]
enum EvictionPolicy {
    LRU,      // Least Recently Used
    LFU,      // Least Frequently Used
    TTL,      // Time To Live
    Size,     // Largest items first
    Hybrid,   // Combination of above
}

// ============================================================================
// MEMORY PRESSURE HANDLING
// ============================================================================

mod memory_pressure_tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_pressure_detection() {
        // Given: Performance manager with memory monitoring
        let config = MemoryPressureConfig {
            max_memory_usage: 100 * 1024 * 1024, // 100MB
            gc_threshold: 0.8, // 80%
            eviction_policy: EvictionPolicy::LRU,
            compression_threshold: 1024, // 1KB
        };

        let mut perf_manager = PerformanceManager::new(config);

        // When: Memory usage approaches threshold
        let test_data = PerformanceTestData {
            id: 1,
            payload: vec![0; 50 * 1024 * 1024], // 50MB
            timestamp: 1000,
            priority: MessagePriority::Normal,
            size_category: SizeCategory::Large,
        };

        // Simulate memory pressure
        for i in 0..2 {
            let mut data = test_data.clone();
            data.id = i;
            perf_manager.cache_message(data).await;
        }

        // Then: Should detect memory pressure
        let memory_usage = perf_manager.get_memory_usage().await;
        assert!(memory_usage > 0.7, "Memory usage should be above 70%");

        let is_pressure_detected = perf_manager.is_memory_pressure_detected().await;
        assert!(is_pressure_detected, "Memory pressure should be detected");
    }

    #[tokio::test]
    async fn test_automatic_garbage_collection() {
        // Given: Performance manager with GC enabled
        let config = MemoryPressureConfig {
            max_memory_usage: 50 * 1024 * 1024, // 50MB
            gc_threshold: 0.7, // 70%
            eviction_policy: EvictionPolicy::LRU,
            compression_threshold: 1024,
        };

        let mut perf_manager = PerformanceManager::new(config);

        // When: Memory usage exceeds GC threshold
        let large_data = PerformanceTestData {
            id: 1,
            payload: vec![0; 40 * 1024 * 1024], // 40MB
            timestamp: 1000,
            priority: MessagePriority::Low,
            size_category: SizeCategory::Large,
        };

        perf_manager.cache_message(large_data).await;

        // Trigger GC
        let gc_result = perf_manager.trigger_garbage_collection().await;
        assert!(gc_result.is_ok(), "GC should succeed");

        // Then: Memory usage should decrease
        let memory_after_gc = perf_manager.get_memory_usage().await;
        assert!(memory_after_gc < 0.5, "Memory usage should be below 50% after GC");

        // And: Should report GC statistics
        let gc_stats = perf_manager.get_gc_statistics().await;
        assert!(gc_stats.is_some());

        let stats = gc_stats.unwrap();
        assert!(stats.bytes_freed > 0);
        assert!(stats.collection_time > Duration::from_millis(0));
    }

    #[tokio::test]
    async fn test_cache_eviction_policies() {
        // Given: Message cache with different eviction policies
        let policies = vec![
            EvictionPolicy::LRU,
            EvictionPolicy::LFU,
            EvictionPolicy::TTL,
            EvictionPolicy::Size,
            EvictionPolicy::Hybrid,
        ];

        for policy in policies {
            let config = MemoryPressureConfig {
                max_memory_usage: 10 * 1024 * 1024, // 10MB
                gc_threshold: 0.8,
                eviction_policy: policy.clone(),
                compression_threshold: 1024,
            };

            let mut cache = MessageCache::new(config);

            // When: Cache becomes full
            let test_messages = vec![
                PerformanceTestData {
                    id: 1,
                    payload: vec![1; 2 * 1024 * 1024], // 2MB
                    timestamp: 1000,
                    priority: MessagePriority::High,
                    size_category: SizeCategory::Large,
                },
                PerformanceTestData {
                    id: 2,
                    payload: vec![2; 2 * 1024 * 1024], // 2MB
                    timestamp: 2000,
                    priority: MessagePriority::Normal,
                    size_category: SizeCategory::Large,
                },
                PerformanceTestData {
                    id: 3,
                    payload: vec![3; 2 * 1024 * 1024], // 2MB
                    timestamp: 3000,
                    priority: MessagePriority::Low,
                    size_category: SizeCategory::Large,
                },
                PerformanceTestData {
                    id: 4,
                    payload: vec![4; 2 * 1024 * 1024], // 2MB
                    timestamp: 4000,
                    priority: MessagePriority::Critical,
                    size_category: SizeCategory::Large,
                },
                PerformanceTestData {
                    id: 5,
                    payload: vec![5; 2 * 1024 * 1024], // 2MB
                    timestamp: 5000,
                    priority: MessagePriority::Normal,
                    size_category: SizeCategory::Large,
                },
            ];

            for msg in test_messages {
                cache.insert(msg).await;
            }

            // Then: Should evict items according to policy
            let evicted_items = cache.get_evicted_items().await;
            assert!(!evicted_items.is_empty(), "Should have evicted items with policy: {:?}", policy);

            // Verify policy-specific behavior
            match policy {
                EvictionPolicy::LRU => {
                    // Should evict least recently used
                    assert!(evicted_items.iter().any(|item| item.id == 1));
                },
                EvictionPolicy::LFU => {
                    // Should evict least frequently used
                    assert!(evicted_items.iter().any(|item| item.priority == MessagePriority::Low));
                },
                EvictionPolicy::TTL => {
                    // Should evict oldest items
                    assert!(evicted_items.iter().any(|item| item.timestamp == 1000));
                },
                EvictionPolicy::Size => {
                    // Should evict largest items
                    assert!(evicted_items.iter().any(|item| item.size_category == SizeCategory::Large));
                },
                EvictionPolicy::Hybrid => {
                    // Should use combination of factors
                    assert!(evicted_items.len() >= 1);
                },
            }
        }
    }
}

// ============================================================================
// CPU-INTENSIVE OPERATIONS
// ============================================================================

mod cpu_intensive_tests {
    use super::*;

    #[tokio::test]
    async fn test_async_task_scheduling() {
        // Given: Performance manager with task scheduling
        let config = MemoryPressureConfig {
            max_memory_usage: 100 * 1024 * 1024,
            gc_threshold: 0.8,
            eviction_policy: EvictionPolicy::LRU,
            compression_threshold: 1024,
        };

        let mut perf_manager = PerformanceManager::new(config);

        // When: Scheduling many CPU-intensive tasks
        let num_tasks = 1000;
        let mut task_handles = Vec::new();

        for i in 0..num_tasks {
            let task_handle = perf_manager.schedule_cpu_task(async move {
                // Simulate CPU-intensive work
                let mut result = 0;
                for j in 0..10000 {
                    result += j * i;
                }
                result
            }).await;

            task_handles.push(task_handle);
        }

        // Then: Should schedule tasks efficiently
        let start_time = Instant::now();

        // Wait for all tasks to complete
        let mut results = Vec::new();
        for handle in task_handles {
            let result = handle.await.unwrap();
            results.push(result);
        }

        let elapsed = start_time.elapsed();

        // Should complete within reasonable time
        assert!(elapsed < Duration::from_secs(10), "Tasks should complete within 10 seconds");
        assert_eq!(results.len(), num_tasks);

        // And: Should report scheduling metrics
        let metrics = perf_manager.get_scheduling_metrics().await;
        assert!(metrics.is_some());

        let metrics = metrics.unwrap();
        assert_eq!(metrics.total_tasks, num_tasks);
        assert!(metrics.average_task_time > Duration::from_millis(0));
        assert!(metrics.cpu_utilization > 0.0);
    }

    #[tokio::test]
    async fn test_priority_based_scheduling() {
        // Given: Performance manager with priority scheduling
        let config = MemoryPressureConfig {
            max_memory_usage: 100 * 1024 * 1024,
            gc_threshold: 0.8,
            eviction_policy: EvictionPolicy::LRU,
            compression_threshold: 1024,
        };

        let mut perf_manager = PerformanceManager::new(config);

        // When: Scheduling tasks with different priorities
        let priorities = vec![
            MessagePriority::Low,
            MessagePriority::Normal,
            MessagePriority::High,
            MessagePriority::Critical,
        ];

        let mut task_handles = Vec::new();
        let mut completion_times = Vec::new();

        for (i, priority) in priorities.iter().enumerate() {
            let priority = priority.clone();
            let task_handle = perf_manager.schedule_priority_task(priority, async move {
                // Simulate work with different durations
                let work_duration = Duration::from_millis(100 * (i + 1) as u64);
                tokio::time::sleep(work_duration).await;
                Instant::now()
            }).await;

            task_handles.push(task_handle);
        }

        // Wait for all tasks to complete
        for handle in task_handles {
            let completion_time = handle.await.unwrap();
            completion_times.push(completion_time);
        }

        // Then: Higher priority tasks should complete first
        // Critical should complete before High, High before Normal, etc.
        assert!(completion_times[3] <= completion_times[2]); // Critical <= High
        assert!(completion_times[2] <= completion_times[1]); // High <= Normal
        assert!(completion_times[1] <= completion_times[0]); // Normal <= Low
    }

    #[tokio::test]
    async fn test_cpu_throttling() {
        // Given: Performance manager with CPU throttling
        let config = MemoryPressureConfig {
            max_memory_usage: 100 * 1024 * 1024,
            gc_threshold: 0.8,
            eviction_policy: EvictionPolicy::LRU,
            compression_threshold: 1024,
        };

        let mut perf_manager = PerformanceManager::new(config);

        // When: CPU usage exceeds threshold
        let cpu_threshold = 0.8; // 80%
        perf_manager.set_cpu_threshold(cpu_threshold).await;

        // Simulate high CPU usage
        let high_cpu_task = perf_manager.schedule_cpu_task(async {
            let mut result = 0;
            for i in 0..1000000 {
                result += i;
            }
            result
        }).await;

        // Then: Should throttle CPU usage
        let cpu_usage = perf_manager.get_cpu_usage().await;
        assert!(cpu_usage <= cpu_threshold + 0.1, "CPU usage should be throttled");

        // And: Should complete task successfully
        let result = high_cpu_task.await.unwrap();
        assert!(result > 0);
    }
}

// ============================================================================
// NETWORK BANDWIDTH OPTIMIZATION
// ============================================================================

mod network_bandwidth_tests {
    use super::*;

    #[tokio::test]
    async fn test_adaptive_compression() {
        // Given: Performance manager with adaptive compression
        let config = MemoryPressureConfig {
            max_memory_usage: 100 * 1024 * 1024,
            gc_threshold: 0.8,
            eviction_policy: EvictionPolicy::LRU,
            compression_threshold: 1024,
        };

        let mut perf_manager = PerformanceManager::new(config);

        // When: Network bandwidth is limited
        let bandwidth_limit = 1024 * 1024; // 1MB/s
        perf_manager.set_bandwidth_limit(bandwidth_limit).await;

        let test_data = PerformanceTestData {
            id: 1,
            payload: vec![0; 10 * 1024 * 1024], // 10MB
            timestamp: 1000,
            priority: MessagePriority::Normal,
            size_category: SizeCategory::Huge,
        };

        // Then: Should automatically compress data
        let compressed_data = perf_manager.compress_data(&test_data).await;
        assert!(compressed_data.is_ok());

        let compressed = compressed_data.unwrap();
        assert!(compressed.payload.len() < test_data.payload.len(), "Compressed data should be smaller");

        // And: Should report compression ratio
        let compression_ratio = perf_manager.get_compression_ratio().await;
        assert!(compression_ratio > 0.0);
        assert!(compression_ratio < 1.0);
    }

    #[tokio::test]
    async fn test_bandwidth_adaptive_batching() {
        // Given: Message batcher with bandwidth adaptation
        let mut batcher = MessageBatcher::new();

        // When: Bandwidth changes dynamically
        let bandwidth_levels = vec![
            (1024 * 1024, 100),      // 1MB/s - small batches
            (10 * 1024 * 1024, 1000), // 10MB/s - medium batches
            (100 * 1024 * 1024, 10000), // 100MB/s - large batches
        ];

        for (bandwidth, expected_batch_size) in bandwidth_levels {
            batcher.set_bandwidth_limit(bandwidth).await;

            // Send messages to test batching
            let mut messages = Vec::new();
            for i in 0..expected_batch_size {
                let msg = PerformanceTestData {
                    id: i,
                    payload: vec![i as u8; 1024], // 1KB each
                    timestamp: 1000 + i,
                    priority: MessagePriority::Normal,
                    size_category: SizeCategory::Small,
                };
                messages.push(msg);
            }

            let batch = batcher.create_batch(messages).await;
            assert!(batch.is_ok());

            let batch = batch.unwrap();
            assert_eq!(batch.len(), expected_batch_size);

            // Then: Should adapt batch size to bandwidth
            let optimal_batch_size = batcher.get_optimal_batch_size().await;
            assert!(optimal_batch_size > 0);
            assert!(optimal_batch_size <= expected_batch_size);
        }
    }

    #[tokio::test]
    async fn test_network_congestion_detection() {
        // Given: Performance manager with network monitoring
        let config = MemoryPressureConfig {
            max_memory_usage: 100 * 1024 * 1024,
            gc_threshold: 0.8,
            eviction_policy: EvictionPolicy::LRU,
            compression_threshold: 1024,
        };

        let mut perf_manager = PerformanceManager::new(config);

        // When: Network becomes congested
        let congestion_threshold = 0.8; // 80% packet loss
        perf_manager.set_congestion_threshold(congestion_threshold).await;

        // Simulate network congestion
        for i in 0..100 {
            let packet_loss = if i < 80 { 0.0 } else { 0.9 }; // 80% packet loss
            perf_manager.record_network_metrics(packet_loss, Duration::from_millis(100 + i)).await;
        }

        // Then: Should detect congestion
        let is_congested = perf_manager.is_network_congested().await;
        assert!(is_congested, "Network congestion should be detected");

        // And: Should adapt transmission strategy
        let strategy = perf_manager.get_transmission_strategy().await;
        assert!(strategy.is_some());

        let strategy = strategy.unwrap();
        assert!(strategy.use_compression);
        assert!(strategy.reduce_batch_size);
        assert!(strategy.increase_retry_delay);
    }
}

// ============================================================================
// CACHE EVICTION STRATEGIES
// ============================================================================

mod cache_eviction_tests {
    use super::*;

    #[tokio::test]
    async fn test_lru_eviction() {
        // Given: Message cache with LRU eviction
        let config = MemoryPressureConfig {
            max_memory_usage: 5 * 1024 * 1024, // 5MB
            gc_threshold: 0.8,
            eviction_policy: EvictionPolicy::LRU,
            compression_threshold: 1024,
        };

        let mut cache = MessageCache::new(config);

        // When: Cache becomes full
        let messages = vec![
            PerformanceTestData {
                id: 1,
                payload: vec![1; 1024 * 1024], // 1MB
                timestamp: 1000,
                priority: MessagePriority::High,
                size_category: SizeCategory::Medium,
            },
            PerformanceTestData {
                id: 2,
                payload: vec![2; 1024 * 1024], // 1MB
                timestamp: 2000,
                priority: MessagePriority::Normal,
                size_category: SizeCategory::Medium,
            },
            PerformanceTestData {
                id: 3,
                payload: vec![3; 1024 * 1024], // 1MB
                timestamp: 3000,
                priority: MessagePriority::Low,
                size_category: SizeCategory::Medium,
            },
            PerformanceTestData {
                id: 4,
                payload: vec![4; 1024 * 1024], // 1MB
                timestamp: 4000,
                priority: MessagePriority::Critical,
                size_category: SizeCategory::Medium,
            },
            PerformanceTestData {
                id: 5,
                payload: vec![5; 1024 * 1024], // 1MB
                timestamp: 5000,
                priority: MessagePriority::Normal,
                size_category: SizeCategory::Medium,
            },
        ];

        for msg in messages {
            cache.insert(msg).await;
        }

        // Access some items to change LRU order
        cache.access(1).await; // Make item 1 recently used
        cache.access(3).await; // Make item 3 recently used

        // Add one more item to trigger eviction
        let new_msg = PerformanceTestData {
            id: 6,
            payload: vec![6; 1024 * 1024], // 1MB
            timestamp: 6000,
            priority: MessagePriority::Normal,
            size_category: SizeCategory::Medium,
        };

        cache.insert(new_msg).await;

        // Then: Should evict least recently used items
        let evicted_items = cache.get_evicted_items().await;
        assert!(!evicted_items.is_empty());

        // Items 2, 4, and 5 should be evicted (not 1 and 3 which were accessed)
        let evicted_ids: Vec<u64> = evicted_items.iter().map(|item| item.id).collect();
        assert!(evicted_ids.contains(&2));
        assert!(evicted_ids.contains(&4));
        assert!(evicted_ids.contains(&5));
        assert!(!evicted_ids.contains(&1));
        assert!(!evicted_ids.contains(&3));
    }

    #[tokio::test]
    async fn test_ttl_eviction() {
        // Given: Message cache with TTL eviction
        let config = MemoryPressureConfig {
            max_memory_usage: 10 * 1024 * 1024, // 10MB
            gc_threshold: 0.8,
            eviction_policy: EvictionPolicy::TTL,
            compression_threshold: 1024,
        };

        let mut cache = MessageCache::new(config);

        // When: Items expire
        let ttl = Duration::from_millis(100);
        cache.set_ttl(ttl).await;

        let messages = vec![
            PerformanceTestData {
                id: 1,
                payload: vec![1; 1024],
                timestamp: 1000,
                priority: MessagePriority::Normal,
                size_category: SizeCategory::Small,
            },
            PerformanceTestData {
                id: 2,
                payload: vec![2; 1024],
                timestamp: 2000,
                priority: MessagePriority::Normal,
                size_category: SizeCategory::Small,
            },
        ];

        for msg in messages {
            cache.insert(msg).await;
        }

        // Wait for TTL to expire
        tokio::time::sleep(ttl + Duration::from_millis(50)).await;

        // Then: Should evict expired items
        let evicted_items = cache.get_evicted_items().await;
        assert!(!evicted_items.is_empty());

        // All items should be evicted due to TTL
        assert_eq!(evicted_items.len(), 2);
    }

    #[tokio::test]
    async fn test_memory_leak_prevention() {
        // Given: Performance manager with leak detection
        let config = MemoryPressureConfig {
            max_memory_usage: 10 * 1024 * 1024, // 10MB
            gc_threshold: 0.8,
            eviction_policy: EvictionPolicy::LRU,
            compression_threshold: 1024,
        };

        let mut perf_manager = PerformanceManager::new(config);

        // When: Creating and destroying many objects
        let initial_memory = perf_manager.get_memory_usage().await;

        for i in 0..1000 {
            let data = PerformanceTestData {
                id: i,
                payload: vec![i as u8; 1024],
                timestamp: 1000 + i,
                priority: MessagePriority::Normal,
                size_category: SizeCategory::Small,
            };

            perf_manager.cache_message(data).await;

            // Periodically trigger GC
            if i % 100 == 0 {
                perf_manager.trigger_garbage_collection().await.unwrap();
            }
        }

        // Force final GC
        perf_manager.trigger_garbage_collection().await.unwrap();

        // Then: Should not have memory leaks
        let final_memory = perf_manager.get_memory_usage().await;
        let memory_increase = final_memory - initial_memory;

        assert!(memory_increase < 0.1, "Memory increase should be minimal: {}", memory_increase);

        // And: Should report leak detection
        let leak_report = perf_manager.get_memory_leak_report().await;
        assert!(leak_report.is_some());

        let report = leak_report.unwrap();
        assert!(report.suspected_leaks.is_empty(), "Should not detect memory leaks");
    }
}

// ============================================================================
// PERFORMANCE MONITORING
// ============================================================================

mod performance_monitoring_tests {
    use super::*;

    #[tokio::test]
    async fn test_bottleneck_detection() {
        // Given: Performance profiler
        let mut profiler = PerformanceProfiler::new();

        // When: Simulating different bottlenecks
        let bottlenecks = vec![
            ("cpu", Duration::from_millis(100)),
            ("memory", Duration::from_millis(200)),
            ("network", Duration::from_millis(300)),
            ("disk", Duration::from_millis(400)),
        ];

        for (bottleneck_type, duration) in bottlenecks {
            profiler.start_profiling(bottleneck_type).await;
            tokio::time::sleep(duration).await;
            profiler.end_profiling(bottleneck_type).await;
        }

        // Then: Should detect bottlenecks
        let detected_bottlenecks = profiler.get_detected_bottlenecks().await;
        assert!(!detected_bottlenecks.is_empty());

        // Should identify the slowest component
        let slowest = profiler.get_slowest_component().await;
        assert!(slowest.is_some());
        assert_eq!(slowest.unwrap(), "disk");
    }

    #[tokio::test]
    async fn test_performance_metrics_collection() {
        // Given: Performance manager with metrics collection
        let config = MemoryPressureConfig {
            max_memory_usage: 100 * 1024 * 1024,
            gc_threshold: 0.8,
            eviction_policy: EvictionPolicy::LRU,
            compression_threshold: 1024,
        };

        let mut perf_manager = PerformanceManager::new(config);

        // When: Performing various operations
        let start_time = Instant::now();

        // Simulate various operations
        for i in 0..100 {
            let data = PerformanceTestData {
                id: i,
                payload: vec![i as u8; 1024],
                timestamp: 1000 + i,
                priority: MessagePriority::Normal,
                size_category: SizeCategory::Small,
            };

            perf_manager.cache_message(data).await;

            if i % 10 == 0 {
                perf_manager.trigger_garbage_collection().await.unwrap();
            }
        }

        let elapsed = start_time.elapsed();

        // Then: Should collect comprehensive metrics
        let metrics = perf_manager.get_performance_metrics().await;
        assert!(metrics.is_some());

        let metrics = metrics.unwrap();
        assert!(metrics.total_operations > 0);
        assert!(metrics.average_operation_time > Duration::from_millis(0));
        assert!(metrics.memory_usage > 0.0);
        assert!(metrics.cpu_usage > 0.0);
        assert!(metrics.network_throughput >= 0.0);

        // And: Should provide performance insights
        let insights = perf_manager.get_performance_insights().await;
        assert!(!insights.is_empty());

        // Should identify performance trends
        let trends = perf_manager.get_performance_trends().await;
        assert!(trends.is_some());
    }

    #[tokio::test]
    async fn test_real_time_performance_alerts() {
        // Given: Performance manager with alerting
        let config = MemoryPressureConfig {
            max_memory_usage: 100 * 1024 * 1024,
            gc_threshold: 0.8,
            eviction_policy: EvictionPolicy::LRU,
            compression_threshold: 1024,
        };

        let mut perf_manager = PerformanceManager::new(config);

        // Set up alerts
        perf_manager.set_alert_threshold("memory_usage", 0.9).await;
        perf_manager.set_alert_threshold("cpu_usage", 0.8).await;
        perf_manager.set_alert_threshold("response_time", Duration::from_millis(1000)).await;

        // When: Performance degrades
        // Simulate high memory usage
        for i in 0..10 {
            let data = PerformanceTestData {
                id: i,
                payload: vec![i as u8; 10 * 1024 * 1024], // 10MB each
                timestamp: 1000 + i,
                priority: MessagePriority::Normal,
                size_category: SizeCategory::Large,
            };

            perf_manager.cache_message(data).await;
        }

        // Then: Should trigger alerts
        let alerts = perf_manager.get_active_alerts().await;
        assert!(!alerts.is_empty());

        // Should have memory usage alert
        let memory_alerts: Vec<_> = alerts.iter()
            .filter(|alert| alert.metric == "memory_usage")
            .collect();
        assert!(!memory_alerts.is_empty());

        // And: Should provide alert details
        let alert = &memory_alerts[0];
        assert!(alert.value > 0.9);
        assert!(alert.severity == "high");
        assert!(!alert.message.is_empty());
    }
}

// ============================================================================
// HELPER IMPLEMENTATIONS
// ============================================================================

// Remove duplicate impl block - using imported one
    fn new(_config: MemoryPressureConfig) -> Self {
        Self {
            memory_usage: 0.0,
            gc_stats: None,
            scheduling_metrics: None,
            cpu_usage: 0.0,
            bandwidth_limit: 0,
            congestion_threshold: 0.0,
            transmission_strategy: None,
            performance_metrics: None,
            active_alerts: Vec::new(),
        }
    }

    async fn cache_message(&mut self, _data: PerformanceTestData) {
        // Simulate caching
        self.memory_usage += 0.1;
    }

    async fn get_memory_usage(&self) -> f64 {
        self.memory_usage
    }

    async fn is_memory_pressure_detected(&self) -> bool {
        self.memory_usage > 0.7
    }

    async fn trigger_garbage_collection(&mut self) -> Result<(), String> {
        self.memory_usage *= 0.5; // Simulate GC
        self.gc_stats = Some(GcStatistics {
            bytes_freed: 1024 * 1024,
            collection_time: Duration::from_millis(10),
        });
        Ok(())
    }

    async fn get_gc_statistics(&self) -> Option<GcStatistics> {
        self.gc_stats.clone()
    }

    async fn schedule_cpu_task<F, Fut, T>(&mut self, f: F) -> tokio::task::JoinHandle<T>
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        tokio::spawn(f())
    }

    async fn schedule_priority_task<F, Fut, T>(&mut self, _priority: MessagePriority, f: F) -> tokio::task::JoinHandle<T>
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        tokio::spawn(f())
    }

    async fn get_scheduling_metrics(&self) -> Option<SchedulingMetrics> {
        self.scheduling_metrics.clone()
    }

    async fn set_cpu_threshold(&mut self, _threshold: f64) {}

    async fn get_cpu_usage(&self) -> f64 {
        self.cpu_usage
    }

    async fn set_bandwidth_limit(&mut self, limit: usize) {
        self.bandwidth_limit = limit;
    }

    async fn compress_data(&self, data: &PerformanceTestData) -> Result<PerformanceTestData, String> {
        let compressed_payload = data.payload.iter().step_by(2).cloned().collect();
        Ok(PerformanceTestData {
            id: data.id,
            payload: compressed_payload,
            timestamp: data.timestamp,
            priority: data.priority.clone(),
            size_category: SizeCategory::Small,
        })
    }

    async fn get_compression_ratio(&self) -> f64 {
        0.5
    }

    async fn set_congestion_threshold(&mut self, threshold: f64) {
        self.congestion_threshold = threshold;
    }

    async fn record_network_metrics(&mut self, _packet_loss: f64, _latency: Duration) {}

    async fn is_network_congested(&self) -> bool {
        self.congestion_threshold > 0.5
    }

    async fn get_transmission_strategy(&self) -> Option<TransmissionStrategy> {
        self.transmission_strategy.clone()
    }

    async fn get_memory_leak_report(&self) -> Option<MemoryLeakReport> {
        Some(MemoryLeakReport {
            suspected_leaks: Vec::new(),
            memory_growth_rate: 0.0,
            gc_effectiveness: 0.8,
        })
    }

    async fn get_performance_metrics(&self) -> Option<PerformanceMetrics> {
        self.performance_metrics.clone()
    }

    async fn get_performance_insights(&self) -> Vec<String> {
        vec!["Memory usage is optimal".to_string()]
    }

    async fn get_performance_trends(&self) -> Option<PerformanceTrends> {
        Some(PerformanceTrends {
            memory_trend: "stable".to_string(),
            cpu_trend: "stable".to_string(),
            network_trend: "stable".to_string(),
        })
    }

    async fn set_alert_threshold(&mut self, _metric: &str, _threshold: f64) {}

    async fn get_active_alerts(&self) -> Vec<PerformanceAlert> {
        self.active_alerts.clone()
    }
}

// Remove duplicate PerformanceManager definition - using imported one

#[derive(Debug, Clone)]
struct GcStatistics {
    bytes_freed: usize,
    collection_time: Duration,
}

#[derive(Debug, Clone)]
struct SchedulingMetrics {
    total_tasks: usize,
    average_task_time: Duration,
    cpu_utilization: f64,
}

#[derive(Debug, Clone)]
struct TransmissionStrategy {
    use_compression: bool,
    reduce_batch_size: bool,
    increase_retry_delay: bool,
}

#[derive(Debug, Clone)]
struct MemoryLeakReport {
    suspected_leaks: Vec<String>,
    memory_growth_rate: f64,
    gc_effectiveness: f64,
}

#[derive(Debug, Clone)]
struct PerformanceTrends {
    memory_trend: String,
    cpu_trend: String,
    network_trend: String,
}

#[derive(Debug, Clone)]
struct PerformanceAlert {
    metric: String,
    value: f64,
    severity: String,
    message: String,
}

// Remove duplicate MessageCache impl - using imported one
    fn new(_config: MemoryPressureConfig) -> Self {
        Self {
            items: Vec::new(),
            evicted_items: Vec::new(),
            ttl: Duration::from_secs(3600),
        }
    }

    async fn insert(&mut self, item: PerformanceTestData) {
        self.items.push(item);

        // Simple eviction logic
        if self.items.len() > 4 {
            let evicted = self.items.remove(0);
            self.evicted_items.push(evicted);
        }
    }

    async fn access(&mut self, id: u64) {
        // Move accessed item to end (LRU)
        if let Some(pos) = self.items.iter().position(|item| item.id == id) {
            let item = self.items.remove(pos);
            self.items.push(item);
        }
    }

    async fn get_evicted_items(&self) -> Vec<PerformanceTestData> {
        self.evicted_items.clone()
    }

    async fn set_ttl(&mut self, ttl: Duration) {
        self.ttl = ttl;
    }
}

// Remove duplicate MessageCache definition - using imported one

// Remove duplicate MessageBatcher impl - using imported one
    fn new() -> Self {
        Self {
            bandwidth_limit: 0,
        }
    }

    async fn set_bandwidth_limit(&mut self, limit: usize) {
        self.bandwidth_limit = limit;
    }

    async fn create_batch(&self, messages: Vec<PerformanceTestData>) -> Result<Vec<PerformanceTestData>, String> {
        Ok(messages)
    }

    async fn get_optimal_batch_size(&self) -> usize {
        if self.bandwidth_limit < 1024 * 1024 {
            100
        } else if self.bandwidth_limit < 10 * 1024 * 1024 {
            1000
        } else {
            10000
        }
    }
}

// Remove duplicate MessageBatcher definition - using imported one

// Remove duplicate PerformanceProfiler impl - using imported one
    fn new() -> Self {
        Self {
            profiles: HashMap::new(),
        }
    }

    async fn start_profiling(&mut self, component: &str) {
        self.profiles.insert(component.to_string(), Instant::now());
    }

    async fn end_profiling(&mut self, component: &str) {
        if let Some(start_time) = self.profiles.get(component) {
            let duration = start_time.elapsed();
            self.profiles.insert(component.to_string(), start_time.clone());
        }
    }

    async fn get_detected_bottlenecks(&self) -> Vec<String> {
        self.profiles.keys().cloned().collect()
    }

    async fn get_slowest_component(&self) -> Option<String> {
        self.profiles.keys().max_by_key(|k| self.profiles.get(*k)).cloned()
    }
}

// Remove duplicate PerformanceProfiler definition - using imported one
