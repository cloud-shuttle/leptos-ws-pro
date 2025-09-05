use leptos_ws::codec::*;
use leptos_ws::transport::{Message, MessageType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct TestData {
    id: u32,
    name: String,
    values: Vec<i32>,
    metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct LargeData {
    data: Vec<u8>,
    description: String,
}

#[test]
fn test_json_codec_basic_serialization() {
    let codec = JsonCodec::new();
    let test_data = TestData {
        id: 42,
        name: "test".to_string(),
        values: vec![1, 2, 3],
        metadata: {
            let mut map = HashMap::new();
            map.insert("key1".to_string(), "value1".to_string());
            map
        },
    };

    let message = Message {
        data: serde_json::to_vec(&test_data).unwrap(),
        message_type: MessageType::Text,
    };
    let encoded = codec.encode(&message).unwrap();
    let decoded = codec.decode(&encoded).unwrap();

    assert_eq!(message, decoded);
}

#[test]
fn test_json_codec_binary_data() {
    let codec = JsonCodec::new();
    let binary_data = vec![0x00, 0x01, 0x02, 0xFF, 0xFE, 0xFD];
    let message = Message {
        data: binary_data.clone(),
        message_type: MessageType::Binary,
    };

    let encoded = codec.encode(&message).unwrap();
    let decoded = codec.decode(&encoded).unwrap();

    assert_eq!(message, decoded);
}

#[test]
fn test_json_codec_ping_pong() {
    let codec = JsonCodec::new();
    let ping_message = Message {
        data: vec![1, 2, 3, 4],
        message_type: MessageType::Ping,
    };
    let pong_message = Message {
        data: vec![1, 2, 3, 4],
        message_type: MessageType::Pong,
    };

    let encoded_ping = codec.encode(&ping_message).unwrap();
    let encoded_pong = codec.encode(&pong_message).unwrap();

    let decoded_ping = codec.decode(&encoded_ping).unwrap();
    let decoded_pong = codec.decode(&encoded_pong).unwrap();

    assert_eq!(ping_message, decoded_ping);
    assert_eq!(pong_message, decoded_pong);
}

#[test]
fn test_json_codec_close_message() {
    let codec = JsonCodec::new();
    let close_data = serde_json::to_vec(&(1000, "Normal closure")).unwrap();
    let close_message = Message {
        data: close_data,
        message_type: MessageType::Close,
    };

    let encoded = codec.encode(&close_message).unwrap();
    let decoded = codec.decode(&encoded).unwrap();

    assert_eq!(close_message, decoded);
}

#[test]
fn test_json_codec_large_data() {
    let codec = JsonCodec::new();
    let large_data = LargeData {
        data: vec![0x42; 10000], // 10KB of data
        description: "Large test data".to_string(),
    };

    let message = Message {
        data: serde_json::to_vec(&large_data).unwrap(),
        message_type: MessageType::Text,
    };
    let encoded = codec.encode(&message).unwrap();
    let decoded = codec.decode(&encoded).unwrap();

    assert_eq!(message, decoded);
}

#[test]
fn test_json_codec_unicode_text() {
    let codec = JsonCodec::new();
    let unicode_text = "Hello 世界! 🌍 测试数据 with émojis 🚀";
    let message = Message {
        data: unicode_text.as_bytes().to_vec(),
        message_type: MessageType::Text,
    };

    let encoded = codec.encode(&message).unwrap();
    let decoded = codec.decode(&encoded).unwrap();

    assert_eq!(message, decoded);
}

#[test]
fn test_json_codec_error_handling() {
    let codec = JsonCodec::new();
    
    // Test with invalid JSON
    let invalid_json = "invalid json {";
    let message = Message {
        data: invalid_json.as_bytes().to_vec(),
        message_type: MessageType::Text,
    };
    
    // This should still work as we're just passing through the data
    let encoded = codec.encode(&message).unwrap();
    let decoded = codec.decode(&encoded).unwrap();
    
    assert_eq!(message, decoded);
}

#[test]
fn test_rkyv_codec_basic_serialization() {
    let codec = RkyvCodec::new();
    let test_data = TestData {
        id: 42,
        name: "test".to_string(),
        values: vec![1, 2, 3],
        metadata: HashMap::new(),
    };

    let message = Message {
        data: serde_json::to_vec(&test_data).unwrap(),
        message_type: MessageType::Text,
    };
    let encoded = codec.encode(&message).unwrap();
    let decoded = codec.decode(&encoded).unwrap();

    assert_eq!(message, decoded);
}

#[test]
fn test_rkyv_codec_binary_data() {
    let codec = RkyvCodec::new();
    let binary_data = vec![0x00, 0x01, 0x02, 0xFF, 0xFE, 0xFD];
    let message = Message {
        data: binary_data.clone(),
        message_type: MessageType::Binary,
    };

    let encoded = codec.encode(&message).unwrap();
    let decoded = codec.decode(&encoded).unwrap();

    assert_eq!(message, decoded);
}

#[test]
fn test_rkyv_codec_performance() {
    let codec = RkyvCodec::new();
    let large_data = LargeData {
        data: vec![0x42; 50000], // 50KB of data
        description: "Performance test data".to_string(),
    };

    let message = Message {
        data: serde_json::to_vec(&large_data).unwrap(),
        message_type: MessageType::Text,
    };
    
    // Test encoding performance
    let start = std::time::Instant::now();
    let encoded = codec.encode(&message).unwrap();
    let encode_time = start.elapsed();
    
    // Test decoding performance
    let start = std::time::Instant::now();
    let decoded = codec.decode(&encoded).unwrap();
    let decode_time = start.elapsed();
    
    assert_eq!(message, decoded);
    
    // Performance assertions (these are generous thresholds)
    assert!(encode_time.as_millis() < 100, "Encoding took too long: {:?}", encode_time);
    assert!(decode_time.as_millis() < 100, "Decoding took too long: {:?}", decode_time);
}

#[test]
fn test_hybrid_codec_creation() {
    let codec = HybridCodec::new();
    assert!(codec.is_ok());
}

#[test]
fn test_hybrid_codec_automatic_selection() {
    let codec = HybridCodec::new().unwrap();
    let test_data = TestData {
        id: 42,
        name: "test".to_string(),
        values: vec![1, 2, 3],
        metadata: HashMap::new(),
    };

    let message = Message {
        data: serde_json::to_vec(&test_data).unwrap(),
        message_type: MessageType::Text,
    };
    let encoded = codec.encode(&message).unwrap();
    let decoded = codec.decode(&encoded).unwrap();

    assert_eq!(message, decoded);
}

#[test]
fn test_hybrid_codec_large_data_optimization() {
    let codec = HybridCodec::new().unwrap();
    let large_data = LargeData {
        data: vec![0x42; 100000], // 100KB of data
        description: "Large data for optimization test".to_string(),
    };

    let message = Message {
        data: serde_json::to_vec(&large_data).unwrap(),
        message_type: MessageType::Text,
    };
    let encoded = codec.encode(&message).unwrap();
    let decoded = codec.decode(&encoded).unwrap();

    assert_eq!(message, decoded);
}

#[test]
fn test_ws_message_wrapper() {
    let test_data = TestData {
        id: 42,
        name: "test".to_string(),
        values: vec![1, 2, 3],
        metadata: HashMap::new(),
    };

    let ws_message = WsMessage::new(test_data.clone());
    assert_eq!(ws_message.data, test_data);
}

#[test]
fn test_ws_message_serialization() {
    let test_data = TestData {
        id: 42,
        name: "test".to_string(),
        values: vec![1, 2, 3],
        metadata: HashMap::new(),
    };

    let ws_message = WsMessage::new(test_data.clone());
    
    // Test JSON serialization
    let json_encoded = serde_json::to_string(&ws_message).unwrap();
    let json_decoded: WsMessage<TestData> = serde_json::from_str(&json_encoded).unwrap();
    assert_eq!(ws_message.data, json_decoded.data);
}

#[test]
fn test_codec_trait_consistency() {
    let codecs: Vec<Box<dyn Codec<Message>>> = vec![
        Box::new(JsonCodec::new()),
        Box::new(RkyvCodec::new()),
        Box::new(HybridCodec::new().unwrap()),
    ];

    let test_message = Message {
        data: "Hello, World!".as_bytes().to_vec(),
        message_type: MessageType::Text,
    };

    for codec in codecs {
        let encoded = codec.encode(&test_message).unwrap();
        let decoded = codec.decode(&encoded).unwrap();
        assert_eq!(test_message, decoded);
    }
}

#[test]
fn test_codec_error_recovery() {
    let codec = JsonCodec::new();
    
    // Test with empty data
    let empty_data = vec![];
    let result: Result<Message, _> = codec.decode(&empty_data);
    assert!(result.is_err());
    
    // Test with corrupted data
    let corrupted_data = vec![0xFF, 0xFE, 0xFD, 0xFC];
    let result: Result<Message, _> = codec.decode(&corrupted_data);
    assert!(result.is_err());
}

#[test]
fn test_codec_concurrent_usage() {
    // Test concurrent encoding/decoding
    let handles: Vec<_> = (0..10)
        .map(|i| {
            std::thread::spawn(move || {
                let codec = JsonCodec::new();
                let message = Message {
                    data: format!("Message {}", i).as_bytes().to_vec(),
                    message_type: MessageType::Text,
                };
                let encoded = codec.encode(&message).unwrap();
                let decoded = codec.decode(&encoded).unwrap();
                assert_eq!(message, decoded);
            })
        })
        .collect();
    
    for handle in handles {
        handle.join().unwrap();
    }
}

#[test]
fn test_codec_memory_efficiency() {
    let codec = JsonCodec::new();
    let large_message = Message {
        data: "x".repeat(100000).as_bytes().to_vec(), // 100KB string
        message_type: MessageType::Text,
    };
    
    let encoded = codec.encode(&large_message).unwrap();
    let decoded = codec.decode(&encoded).unwrap();
    
    assert_eq!(large_message, decoded);
    
    // Verify that the encoded data is reasonable in size
    // (JSON encoding adds overhead, so we allow for more expansion)
    // The Message struct with serde adds significant overhead for large data
    assert!(encoded.len() <= large_message.data.len() * 5);
}

#[test]
fn test_codec_type_safety() {
    let codec = JsonCodec::new();
    
    // Test that we can't accidentally mix message types
    let text_message = Message {
        data: "Hello".as_bytes().to_vec(),
        message_type: MessageType::Text,
    };
    let binary_message = Message {
        data: vec![1, 2, 3],
        message_type: MessageType::Binary,
    };
    
    let text_encoded = codec.encode(&text_message).unwrap();
    let binary_encoded = codec.encode(&binary_message).unwrap();
    
    let text_decoded = codec.decode(&text_encoded).unwrap();
    let binary_decoded = codec.decode(&binary_encoded).unwrap();
    
    assert_eq!(text_message, text_decoded);
    assert_eq!(binary_message, binary_decoded);
    assert_ne!(text_decoded, binary_decoded);
}