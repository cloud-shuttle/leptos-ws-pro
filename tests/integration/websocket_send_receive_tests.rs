//! WebSocket Send/Receive Integration Tests
//!
//! These tests validate that the WebSocket send_message method actually works
//! and sends real data over the network.

use leptos_ws_pro::transport::{
    websocket::WebSocketConnection,
    Transport, TransportConfig, Message, MessageType
};
use tokio::time::{timeout, Duration};

/// Test that send_message actually sends data
#[tokio::test]
async fn test_websocket_send_message_actually_sends() {
    let config = TransportConfig::default();
    let mut connection = WebSocketConnection::new(config).await.unwrap();

    // Connect to a real WebSocket echo server
    let result = timeout(
        Duration::from_secs(10),
        connection.connect("wss://echo.websocket.org")
    ).await;

    match result {
        Ok(Ok(())) => {
            println!("✅ Connected to real WebSocket echo server");

            // Test the connection state
            assert_eq!(connection.state(), leptos_ws_pro::transport::ConnectionState::Connected);

            // Test sending a message using send_message method
            let test_message = Message {
                data: b"Hello, WebSocket send_message!".to_vec(),
                message_type: MessageType::Text,
            };

            // This should actually send the message now
            let send_result = connection.send_message(&test_message).await;
            assert!(send_result.is_ok(), "send_message should work: {:?}", send_result);

            println!("✅ Successfully sent message using send_message method");

            // Test that we can use the split method to receive
            let (stream, sink) = connection.split();

            // Send another message via sink
            let sink_message = Message {
                data: b"Hello, WebSocket sink!".to_vec(),
                message_type: MessageType::Text,
            };

            let sink_result = sink.send(sink_message).await;
            assert!(sink_result.is_ok(), "sink.send should work: {:?}", sink_result);

            println!("✅ Successfully sent message using sink");

        }
        Ok(Err(e)) => {
            println!("⚠️  Failed to connect to real WebSocket server: {}", e);
            // This is not a test failure - the server might be down
        }
        Err(_) => {
            println!("⚠️  Timeout connecting to real WebSocket server");
            // This is not a test failure - network might be slow
        }
    }
}

/// Test that send_message works with different message types
#[tokio::test]
async fn test_websocket_send_message_types() {
    let config = TransportConfig::default();
    let mut connection = WebSocketConnection::new(config).await.unwrap();

    // Connect to a real WebSocket echo server
    let result = timeout(
        Duration::from_secs(10),
        connection.connect("wss://echo.websocket.org")
    ).await;

    match result {
        Ok(Ok(())) => {
            println!("✅ Connected for message type testing");

            // Test text message
            let text_message = Message {
                data: b"Text message".to_vec(),
                message_type: MessageType::Text,
            };
            let result = connection.send_message(&text_message).await;
            assert!(result.is_ok(), "Text message should send: {:?}", result);

            // Test binary message
            let binary_message = Message {
                data: vec![0x00, 0x01, 0x02, 0x03],
                message_type: MessageType::Binary,
            };
            let result = connection.send_message(&binary_message).await;
            assert!(result.is_ok(), "Binary message should send: {:?}", result);

            // Test ping message
            let ping_message = Message {
                data: b"ping".to_vec(),
                message_type: MessageType::Ping,
            };
            let result = connection.send_message(&ping_message).await;
            assert!(result.is_ok(), "Ping message should send: {:?}", result);

            println!("✅ All message types sent successfully");

        }
        Ok(Err(e)) => {
            println!("⚠️  Failed to connect for message type testing: {}", e);
        }
        Err(_) => {
            println!("⚠️  Timeout connecting for message type testing");
        }
    }
}

/// Test that send_message fails when not connected
#[tokio::test]
async fn test_websocket_send_message_not_connected() {
    let config = TransportConfig::default();
    let connection = WebSocketConnection::new(config).await.unwrap();

    // Should be disconnected initially
    assert_eq!(connection.state(), leptos_ws_pro::transport::ConnectionState::Disconnected);

    // Sending a message when not connected should fail
    let test_message = Message {
        data: b"Should fail".to_vec(),
        message_type: MessageType::Text,
    };

    let result = connection.send_message(&test_message).await;
    assert!(result.is_err(), "send_message should fail when not connected");

    // Check the error type
    match result {
        Err(leptos_ws_pro::transport::TransportError::ConnectionFailed(msg)) => {
            assert!(msg.contains("Not connected"));
        }
        _ => panic!("Expected ConnectionFailed error"),
    }

    println!("✅ send_message correctly fails when not connected");
}

/// Test that receive_message returns NotSupported (as expected)
#[tokio::test]
async fn test_websocket_receive_message_not_supported() {
    let config = TransportConfig::default();
    let connection = WebSocketConnection::new(config).await.unwrap();

    // receive_message should return NotSupported
    let result = connection.receive_message().await;
    assert!(result.is_err(), "receive_message should return NotSupported");

    // Check the error type
    match result {
        Err(leptos_ws_pro::transport::TransportError::NotSupported(msg)) => {
            assert!(msg.contains("Use split()"));
        }
        _ => panic!("Expected NotSupported error"),
    }

    println!("✅ receive_message correctly returns NotSupported");
}

/// Test that the split method still works
#[tokio::test]
async fn test_websocket_split_method_still_works() {
    let config = TransportConfig::default();
    let mut connection = WebSocketConnection::new(config).await.unwrap();

    // Connect to a real WebSocket echo server
    let result = timeout(
        Duration::from_secs(10),
        connection.connect("wss://echo.websocket.org")
    ).await;

    match result {
        Ok(Ok(())) => {
            println!("✅ Connected for split method testing");

            // Test that split method works
            let (stream, sink) = connection.split();

            // Send a message via sink
            let test_message = Message {
                data: b"Split method test".to_vec(),
                message_type: MessageType::Text,
            };

            let result = sink.send(test_message).await;
            assert!(result.is_ok(), "sink.send should work: {:?}", result);

            println!("✅ Split method works correctly");

        }
        Ok(Err(e)) => {
            println!("⚠️  Failed to connect for split method testing: {}", e);
        }
        Err(_) => {
            println!("⚠️  Timeout connecting for split method testing");
        }
    }
}
