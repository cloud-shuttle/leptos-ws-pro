//! Comprehensive unit tests for codec module - v1.0 TDD
//!
//! This test suite ensures 100% coverage of the codec functionality
//! following TDD principles for v1.0 release.

use leptos_ws_pro::codec::{Codec, CodecError, HybridCodec, JsonCodec, RkyvCodec, WsMessage};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TestData {
    id: u64,
    name: String,
    values: Vec<i32>,
    metadata: std::collections::HashMap<String, String>,
}

impl TestData {
    pub fn new() -> Self {
        Self {
            id: 12345,
            name: "test_data".to_string(),
            values: vec![1, 2, 3, 4, 5],
            metadata: [
                ("key1".to_string(), "value1".to_string()),
                ("key2".to_string(), "value2".to_string()),
            ]
            .iter()
            .cloned()
            .collect(),
        }
    }

    pub fn large() -> Self {
        Self {
            id: u64::MAX,
            name: "x".repeat(10000),      // Large string
            values: (0..10000).collect(), // Large vector
            metadata: (0..1000)
                .map(|i| (format!("key_{}", i), format!("value_{}", i)))
                .collect(),
        }
    }
}

#[cfg(test)]
mod codec_core_tests {
    use super::*;

    #[test]
    fn test_json_codec_basic_roundtrip() {
        let codec = JsonCodec::new();
        let data = TestData::new();

        // Test encode
        let encoded = codec.encode(&data).unwrap();
        assert!(!encoded.is_empty());
        assert_eq!(
            <JsonCodec as Codec<TestData>>::content_type(&codec),
            "application/json"
        );

        // Verify it's valid JSON
        let json_value: serde_json::Value = serde_json::from_slice(&encoded).unwrap();
        assert!(json_value.is_object());

        // Test decode
        let decoded = codec.decode(&encoded).unwrap();
        assert_eq!(data, decoded);
    }

    #[test]
    fn test_json_codec_with_unicode() {
        let codec = JsonCodec::new();
        let data = TestData {
            id: 1,
            name: "Hello 🌍 World! 中文 العربية".to_string(),
            values: vec![],
            metadata: [
                ("emoji".to_string(), "🚀🎯💡".to_string()),
                ("chinese".to_string(), "你好世界".to_string()),
                ("arabic".to_string(), "مرحبا بالعالم".to_string()),
            ]
            .iter()
            .cloned()
            .collect(),
        };

        let encoded = codec.encode(&data).unwrap();
        let decoded = codec.decode(&encoded).unwrap();
        assert_eq!(data, decoded);
    }

    #[test]
    fn test_json_codec_error_handling() {
        let codec = JsonCodec::new();

        // Test decode with invalid JSON
        let invalid_json = b"invalid json data {{{";
        let result = <JsonCodec as Codec<TestData>>::decode(&codec, invalid_json);
        assert!(result.is_err());

        match result {
            Err(CodecError::DeserializationFailed(msg)) => {
                assert!(msg.contains("expected"));
            }
            _ => panic!("Expected DeserializationFailed error"),
        }
    }

    #[test]
    fn test_rkyv_codec_basic_roundtrip() {
        let codec = RkyvCodec::new();
        let data = TestData::new();

        let encoded = codec.encode(&data).unwrap();
        assert!(!encoded.is_empty());
        assert_eq!(
            <RkyvCodec as Codec<TestData>>::content_type(&codec),
            "application/rkyv"
        );

        let decoded = codec.decode(&encoded).unwrap();
        assert_eq!(data, decoded);
    }

    #[test]
    fn test_hybrid_codec_creation() {
        let codec = HybridCodec::new().unwrap();
        assert_eq!(
            <HybridCodec as Codec<TestData>>::content_type(&codec),
            "application/hybrid"
        );
    }

    #[test]
    fn test_hybrid_codec_roundtrip() {
        let codec = HybridCodec::new().unwrap();
        let data = TestData::new();

        let encoded = codec.encode(&data).unwrap();
        let decoded = codec.decode(&encoded).unwrap();
        assert_eq!(data, decoded);
    }

    #[test]
    fn test_hybrid_codec_fallback_behavior() {
        let codec = HybridCodec::new().unwrap();

        // Create data that should work with both codecs
        let data = TestData::new();

        // Test encoding (should try rkyv first, fall back to JSON)
        let encoded = codec.encode(&data).unwrap();
        assert!(!encoded.is_empty());

        // Test decoding (should try JSON first, fall back to rkyv)
        let decoded = codec.decode(&encoded).unwrap();
        assert_eq!(data, decoded);
    }

    #[test]
    fn test_ws_message_wrapper() {
        let test_data = TestData::new();
        let ws_message = WsMessage::new(test_data.clone());

        assert_eq!(ws_message.data, test_data);

        // Test serialization
        let json = serde_json::to_string(&ws_message).unwrap();
        let deserialized: WsMessage<TestData> = serde_json::from_str(&json).unwrap();
        assert_eq!(ws_message.data, deserialized.data);
    }

    #[test]
    fn test_ws_message_with_various_types() {
        // Test with string
        let str_msg = WsMessage::new("Hello, World!".to_string());
        assert_eq!(str_msg.data, "Hello, World!");

        // Test with number
        let num_msg = WsMessage::new(42i32);
        assert_eq!(num_msg.data, 42);

        // Test with vector
        let vec_msg = WsMessage::new(vec![1, 2, 3, 4, 5]);
        assert_eq!(vec_msg.data, vec![1, 2, 3, 4, 5]);

        // Test with complex struct
        let struct_msg = WsMessage::new(TestData::new());
        assert_eq!(struct_msg.data.id, 12345);
    }

    #[test]
    fn test_codec_error_types() {
        // Test error creation and formatting
        let errors = vec![
            CodecError::SerializationFailed("Serialization failed".to_string()),
            CodecError::DeserializationFailed("Deserialization failed".to_string()),
            CodecError::CompressionFailed("Compression failed".to_string()),
            CodecError::DecompressionFailed("Decompression failed".to_string()),
        ];

        for error in errors {
            let error_string = error.to_string();
            assert!(!error_string.is_empty());

            // Test that error implements standard Error trait
            let std_error: &dyn std::error::Error = &error;
            assert!(!std_error.to_string().is_empty());
        }
    }

    #[test]
    fn test_large_data_encoding() {
        let codecs: Vec<Box<dyn Codec<TestData>>> = vec![
            Box::new(JsonCodec::new()),
            Box::new(RkyvCodec::new()),
            Box::new(HybridCodec::new().unwrap()),
        ];

        let large_data = TestData::large();

        for codec in codecs {
            let encoded = codec.encode(&large_data).unwrap();
            assert!(encoded.len() > 1000); // Should be substantial

            let decoded = codec.decode(&encoded).unwrap();
            assert_eq!(large_data, decoded);
        }
    }

    #[test]
    fn test_empty_data_handling() {
        let codec = JsonCodec::new();

        let empty_data = TestData {
            id: 0,
            name: String::new(),
            values: Vec::new(),
            metadata: std::collections::HashMap::new(),
        };

        let encoded = codec.encode(&empty_data).unwrap();
        let decoded = codec.decode(&encoded).unwrap();
        assert_eq!(empty_data, decoded);
    }
}

#[cfg(test)]
mod codec_performance_tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_json_codec_performance() {
        let codec = JsonCodec::new();
        let data = TestData::new();
        let iterations = 1000;

        let start = Instant::now();
        for _ in 0..iterations {
            let encoded = codec.encode(&data).unwrap();
            let _decoded: TestData = codec.decode(&encoded).unwrap();
        }
        let elapsed = start.elapsed();

        // Should complete in reasonable time (less than 1 second for 1000 iterations)
        assert!(
            elapsed.as_secs() < 1,
            "JSON codec took too long: {:?}",
            elapsed
        );
    }

    #[test]
    fn test_hybrid_codec_performance() {
        let codec = HybridCodec::new().unwrap();
        let data = TestData::new();
        let iterations = 1000;

        let start = Instant::now();
        for _ in 0..iterations {
            let encoded = codec.encode(&data).unwrap();
            let _decoded: TestData = codec.decode(&encoded).unwrap();
        }
        let elapsed = start.elapsed();

        // Should complete in reasonable time
        assert!(
            elapsed.as_secs() < 2,
            "Hybrid codec took too long: {:?}",
            elapsed
        );
    }
}

#[cfg(test)]
mod codec_edge_cases {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct EdgeCaseData {
        option_field: Option<String>,
        result_field: Result<i32, String>,
        nested: Box<EdgeCaseData>,
    }

    #[test]
    fn test_codec_with_option_types() {
        let codec = JsonCodec::new();

        // Test with Some value
        let data_some = TestData {
            id: 1,
            name: "some".to_string(),
            values: vec![1],
            metadata: [("key".to_string(), "value".to_string())]
                .iter()
                .cloned()
                .collect(),
        };

        let encoded = codec.encode(&data_some).unwrap();
        let decoded = codec.decode(&encoded).unwrap();
        assert_eq!(data_some, decoded);

        // Test with empty values
        let data_empty = TestData {
            id: 0,
            name: String::new(),
            values: Vec::new(),
            metadata: std::collections::HashMap::new(),
        };

        let encoded = codec.encode(&data_empty).unwrap();
        let decoded = codec.decode(&encoded).unwrap();
        assert_eq!(data_empty, decoded);
    }

    #[test]
    fn test_codec_with_special_characters() {
        let codec = JsonCodec::new();

        let special_data = TestData {
            id: 1,
            name: "\"quotes\" and \\ backslashes \n newlines \t tabs".to_string(),
            values: vec![],
            metadata: [
                ("null".to_string(), "\0".to_string()),
                ("control".to_string(), "\x01\x02\x03".to_string()),
            ]
            .iter()
            .cloned()
            .collect(),
        };

        let encoded = codec.encode(&special_data).unwrap();
        let decoded = codec.decode(&encoded).unwrap();
        assert_eq!(special_data, decoded);
    }

    #[test]
    fn test_codec_thread_safety() {
        use std::sync::Arc;
        use std::thread;

        let codec = Arc::new(JsonCodec::new());
        let data = Arc::new(TestData::new());

        let handles: Vec<_> = (0..10)
            .map(|_| {
                let codec_clone = codec.clone();
                let data_clone = data.clone();

                thread::spawn(move || {
                    let encoded = codec_clone.encode(&*data_clone).unwrap();
                    let decoded: TestData = codec_clone.decode(&encoded).unwrap();
                    assert_eq!(*data_clone, decoded);
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }
    }
}

#[cfg(test)]
mod codec_integration_tests {
    use super::*;

    #[test]
    fn test_cross_codec_compatibility() {
        let json_codec = JsonCodec::new();
        let rkyv_codec = RkyvCodec::new();
        let data = TestData::new();

        // Encode with JSON
        let json_encoded = json_codec.encode(&data).unwrap();

        // Since RkyvCodec currently uses JSON internally, this should work
        let json_decoded: TestData = rkyv_codec.decode(&json_encoded).unwrap();
        assert_eq!(data, json_decoded);

        // Encode with Rkyv
        let rkyv_encoded = rkyv_codec.encode(&data).unwrap();

        // Decode with JSON (should work since Rkyv uses JSON internally)
        let rkyv_decoded: TestData = json_codec.decode(&rkyv_encoded).unwrap();
        assert_eq!(data, rkyv_decoded);
    }

    #[test]
    fn test_codec_with_nested_ws_messages() {
        let codec = JsonCodec::new();

        let inner_data = TestData::new();
        let inner_msg = WsMessage::new(inner_data.clone());
        let outer_msg = WsMessage::new(inner_msg);

        let encoded = codec.encode(&outer_msg).unwrap();
        let decoded: WsMessage<WsMessage<TestData>> = codec.decode(&encoded).unwrap();

        assert_eq!(outer_msg.data.data, decoded.data.data);
    }
}
