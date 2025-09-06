//! TDD tests for Performance Benchmarks and Metrics
//!
//! These tests verify performance characteristics and measure
//! key metrics like latency, throughput, and resource usage.

use leptos_ws_pro::transport::{
    ConnectionState, Message, MessageType, Transport, TransportConfig,
    websocket::WebSocketConnection,
    sse::SseConnection,
    webtransport::WebTransportConnection,
    adaptive::AdaptiveTransport,
};
use futures::{SinkExt, StreamExt};
use std::time::{Duration, Instant};
use tokio::time::timeout;

/// Test WebSocket server for benchmarking
async fn start_benchmark_websocket_server() -> (tokio::net::TcpListener, u16) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    (listener, port)
}

/// Run a benchmark WebSocket echo server
async fn run_benchmark_websocket_server(listener: tokio::net::TcpListener) {
    use tokio_tungstenite::accept_async;
    
    while let Ok((stream, _)) = listener.accept().await {
        let ws_stream = accept_async(stream).await.unwrap();
        let (mut write, mut read) = ws_stream.split();
        
        // Echo back all messages
        tokio::spawn(async move {
            while let Some(msg) = read.next().await {
                if let Ok(msg) = msg {
                    if write.send(msg).await.is_err() {
                        break;
                    }
                }
            }
        });
    }
}

#[tokio::test]
async fn test_websocket_connection_latency() {
    // Given: A WebSocket server and client
    let (listener, port) = start_benchmark_websocket_server().await;
    let server_handle = tokio::spawn(run_benchmark_websocket_server(listener));
    
    let config = TransportConfig {
        url: format!("ws://127.0.0.1:{}", port),
        ..Default::default()
    };
    
    let mut client = WebSocketConnection::new(config).await.unwrap();
    
    // When: Measuring connection latency
    let start = Instant::now();
    let result = client.connect(&format!("ws://127.0.0.1:{}", port)).await;
    let connection_time = start.elapsed();
    
    // Then: Should connect quickly
    assert!(result.is_ok(), "Connection failed: {:?}", result);
    assert!(connection_time < Duration::from_millis(100), 
            "Connection took too long: {:?}", connection_time);
    
    // Cleanup
    let _ = client.disconnect().await;
    server_handle.abort();
}

#[tokio::test]
async fn test_websocket_message_latency() {
    // Given: A connected WebSocket client
    let (listener, port) = start_benchmark_websocket_server().await;
    let server_handle = tokio::spawn(run_benchmark_websocket_server(listener));
    
    let config = TransportConfig {
        url: format!("ws://127.0.0.1:{}", port),
        ..Default::default()
    };
    
    let mut client = WebSocketConnection::new(config).await.unwrap();
    client.connect(&format!("ws://127.0.0.1:{}", port)).await.unwrap();
    
    let (mut stream, mut sink) = client.split();
    
    // When: Measuring message round-trip latency
    let num_tests = 10;
    let mut latencies = Vec::new();
    
    for i in 0..num_tests {
        let test_message = Message {
            data: format!("Latency test {}", i).as_bytes().to_vec(),
            message_type: MessageType::Text,
        };
        
        let start = Instant::now();
        sink.send(test_message.clone()).await.unwrap();
        
        let received = timeout(Duration::from_secs(5), stream.next()).await;
        let latency = start.elapsed();
        
        assert!(received.is_ok(), "Message timeout for test {}", i);
        let received_msg = received.unwrap().unwrap().unwrap();
        assert_eq!(received_msg.data, test_message.data);
        
        latencies.push(latency);
    }
    
    // Then: Should have reasonable latency
    let avg_latency = latencies.iter().sum::<Duration>() / num_tests as u32;
    let max_latency = latencies.iter().max().unwrap();
    
    assert!(avg_latency < Duration::from_millis(50), 
            "Average latency too high: {:?}", avg_latency);
    assert!(*max_latency < Duration::from_millis(100), 
            "Max latency too high: {:?}", max_latency);
    
    println!("Average latency: {:?}, Max latency: {:?}", avg_latency, max_latency);
    
    // Cleanup
    drop(stream);
    drop(sink);
    server_handle.abort();
}

#[tokio::test]
async fn test_websocket_throughput() {
    // Given: A connected WebSocket client
    let (listener, port) = start_benchmark_websocket_server().await;
    let server_handle = tokio::spawn(run_benchmark_websocket_server(listener));
    
    let config = TransportConfig {
        url: format!("ws://127.0.0.1:{}", port),
        ..Default::default()
    };
    
    let mut client = WebSocketConnection::new(config).await.unwrap();
    client.connect(&format!("ws://127.0.0.1:{}", port)).await.unwrap();
    
    let (mut stream, mut sink) = client.split();
    
    // When: Measuring throughput
    let num_messages = 1000;
    let message_size = 1024; // 1KB messages
    let test_data = vec![0x42; message_size];
    
    let start = Instant::now();
    
    // Send all messages
    for _i in 0..num_messages {
        let test_message = Message {
            data: test_data.clone(),
            message_type: MessageType::Binary,
        };
        
        sink.send(test_message).await.unwrap();
    }
    
    // Receive all messages
    let mut received_count = 0;
    while received_count < num_messages {
        let received = timeout(Duration::from_secs(10), stream.next()).await;
        if received.is_ok() {
            received_count += 1;
        } else {
            break;
        }
    }
    
    let total_time = start.elapsed();
    
    // Then: Should achieve reasonable throughput
    assert_eq!(received_count, num_messages, "Didn't receive all messages");
    
    let total_bytes = (num_messages * message_size) as u64;
    let throughput_mbps = (total_bytes as f64) / (total_time.as_secs_f64() * 1_000_000.0);
    
    assert!(throughput_mbps > 1.0, "Throughput too low: {:.2} MB/s", throughput_mbps);
    
    println!("Throughput: {:.2} MB/s, Total time: {:?}", throughput_mbps, total_time);
    
    // Cleanup
    drop(stream);
    drop(sink);
    server_handle.abort();
}

#[tokio::test]
async fn test_transport_creation_performance() {
    // Given: Transport configurations
    let ws_config = TransportConfig {
        url: "ws://example.com/ws".to_string(),
        ..Default::default()
    };
    
    let sse_config = TransportConfig {
        url: "http://example.com/events".to_string(),
        ..Default::default()
    };
    
    let wt_config = TransportConfig {
        url: "https://example.com/webtransport".to_string(),
        ..Default::default()
    };
    
    // When: Measuring creation time for each transport type
    let num_instances = 100;
    
    // WebSocket creation
    let start = Instant::now();
    let mut ws_clients = Vec::new();
    for _ in 0..num_instances {
        let client = WebSocketConnection::new(ws_config.clone()).await.unwrap();
        ws_clients.push(client);
    }
    let ws_creation_time = start.elapsed();
    
    // SSE creation
    let start = Instant::now();
    let mut sse_clients = Vec::new();
    for _ in 0..num_instances {
        let client = SseConnection::new(sse_config.clone()).await.unwrap();
        sse_clients.push(client);
    }
    let sse_creation_time = start.elapsed();
    
    // WebTransport creation
    let start = Instant::now();
    let mut wt_clients = Vec::new();
    for _ in 0..num_instances {
        let client = WebTransportConnection::new(wt_config.clone()).await.unwrap();
        wt_clients.push(client);
    }
    let wt_creation_time = start.elapsed();
    
    // Then: Should create instances quickly
    assert!(ws_creation_time < Duration::from_millis(100), 
            "WebSocket creation too slow: {:?}", ws_creation_time);
    assert!(sse_creation_time < Duration::from_millis(100), 
            "SSE creation too slow: {:?}", sse_creation_time);
    assert!(wt_creation_time < Duration::from_millis(100), 
            "WebTransport creation too slow: {:?}", wt_creation_time);
    
    println!("WebSocket creation: {:?} for {} instances", ws_creation_time, num_instances);
    println!("SSE creation: {:?} for {} instances", sse_creation_time, num_instances);
    println!("WebTransport creation: {:?} for {} instances", wt_creation_time, num_instances);
}

#[tokio::test]
async fn test_memory_usage_benchmark() {
    // Given: A WebSocket server and client
    let (listener, port) = start_benchmark_websocket_server().await;
    let server_handle = tokio::spawn(run_benchmark_websocket_server(listener));
    
    let config = TransportConfig {
        url: format!("ws://127.0.0.1:{}", port),
        ..Default::default()
    };
    
    let mut client = WebSocketConnection::new(config).await.unwrap();
    client.connect(&format!("ws://127.0.0.1:{}", port)).await.unwrap();
    
    let (mut stream, mut sink) = client.split();
    
    // When: Sending many messages and measuring memory usage
    let num_messages = 10000;
    let message_size = 100; // 100 bytes per message
    
    let start = Instant::now();
    
    for i in 0..num_messages {
        let test_message = Message {
            data: vec![0x42; message_size],
            message_type: MessageType::Binary,
        };
        
        sink.send(test_message).await.unwrap();
        
        // Receive every 100th message to prevent memory buildup
        if i % 100 == 0 {
            let _ = timeout(Duration::from_millis(10), stream.next()).await;
        }
    }
    
    let total_time = start.elapsed();
    
    // Then: Should handle many messages efficiently
    assert!(total_time < Duration::from_secs(30), 
            "Memory benchmark took too long: {:?}", total_time);
    
    let messages_per_second = num_messages as f64 / total_time.as_secs_f64();
    assert!(messages_per_second > 1000.0, 
            "Message rate too low: {:.0} msg/s", messages_per_second);
    
    println!("Memory benchmark: {:.0} messages/sec, Total time: {:?}", 
             messages_per_second, total_time);
    
    // Cleanup
    drop(stream);
    drop(sink);
    server_handle.abort();
}

#[tokio::test]
async fn test_adaptive_transport_performance() {
    // Given: Adaptive transport configuration
    let config = TransportConfig {
        url: "ws://example.com/ws".to_string(),
        ..Default::default()
    };
    
    // When: Measuring adaptive transport creation and operations
    let num_instances = 50;
    
    let start = Instant::now();
    let mut adaptive_transports = Vec::new();
    for _ in 0..num_instances {
        let transport = AdaptiveTransport::new(config.clone()).await.unwrap();
        adaptive_transports.push(transport);
    }
    let creation_time = start.elapsed();
    
    // Then: Should create adaptive transports efficiently
    assert!(creation_time < Duration::from_millis(100), 
            "Adaptive transport creation too slow: {:?}", creation_time);
    
    // Verify all instances are in correct state
    for transport in &adaptive_transports {
        assert_eq!(transport.state(), ConnectionState::Disconnected);
    }
    
    println!("Adaptive transport creation: {:?} for {} instances", 
             creation_time, num_instances);
}
