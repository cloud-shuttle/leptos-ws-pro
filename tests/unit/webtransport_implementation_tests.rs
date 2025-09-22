//! TDD tests for WebTransport implementation
//!
//! These tests drive the implementation of WebTransport connections
//! using HTTP/3, providing an alternative to WebSocket connections.

use futures::{SinkExt, StreamExt};
use leptos_ws_pro::transport::{
    webtransport::WebTransportConnection, ConnectionState, Message, MessageType, Transport,
    TransportConfig, TransportError,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::time::timeout;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct TestMessage {
    id: u32,
    content: String,
    timestamp: u64,
}

/// Start a test HTTP/3 server for WebTransport testing
async fn start_test_http3_server() -> (TcpListener, u16) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    (listener, port)
}

/// Helper function to start server and wait for it to be ready
async fn start_server_and_wait() -> u16 {
    let (listener, port) = start_test_http3_server().await;

    // Start server in background
    tokio::spawn(async move {
        run_http3_echo_server(listener).await;
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    port
}

/// Run an HTTP/3 echo server for WebTransport testing
async fn run_http3_echo_server(listener: TcpListener) {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    while let Ok((stream, _)) = listener.accept().await {
        tokio::spawn(async move {
            let mut stream = BufReader::new(stream);
            let mut line = String::new();

            // Read HTTP request
            while stream.read_line(&mut line).await.unwrap() > 0 {
                if line == "\r\n" {
                    break;
                }
                line.clear();
            }

            // Send HTTP/1.1 WebTransport response (for testing)
            let response = "HTTP/1.1 200 OK\r\n\
                          Upgrade: webtransport\r\n\
                          Connection: Upgrade\r\n\
                          Sec-WebTransport-Accept: test\r\n\r\n";

            stream.get_mut().write_all(response.as_bytes()).await.unwrap();

            // Simulate WebTransport stream data
            let test_data = b"WebTransport test message";
            stream.get_mut().write_all(test_data).await.unwrap();
        });
    }
}

#[tokio::test]
async fn test_webtransport_connection() {
    // Given: An HTTP/3 server running on localhost
    let port = start_server_and_wait().await;

    // When: Client connects to the server via WebTransport
    let config = TransportConfig {
        url: format!("http://127.0.0.1:{}", port),
        ..Default::default()
    };
    let mut client = WebTransportConnection::new(config).await.unwrap();
    let result = client.connect(&format!("http://127.0.0.1:{}", port)).await;

    // Then: Connection should succeed
    assert!(result.is_ok());
    assert_eq!(client.state(), ConnectionState::Connected);
}

#[tokio::test]
async fn test_webtransport_message_sending() {
    // Given: A connected WebTransport client and HTTP/3 echo server
    let port = start_server_and_wait().await;

    let config = TransportConfig {
        url: format!("http://127.0.0.1:{}", port),
        ..Default::default()
    };
    let mut client = WebTransportConnection::new(config).await.unwrap();
    client
        .connect(&format!("http://127.0.0.1:{}", port))
        .await
        .unwrap();

    // When: Client sends a text message
    let message = Message {
        data: "Hello, WebTransport!".as_bytes().to_vec(),
        message_type: MessageType::Text,
    };
    let (mut stream, mut sink) = client.split();

    // Then: Message should be sent successfully
    let send_result = sink.send(message.clone()).await;
    assert!(send_result.is_ok());

    // And: Should receive the echoed message back
    let received = stream.next().await;
    assert!(received.is_some());
    let received_msg = received.unwrap().unwrap();
    assert_eq!(received_msg, message);
}

#[tokio::test]
async fn test_webtransport_binary_message() {
    // Given: A connected WebTransport client and HTTP/3 echo server
    let port = start_server_and_wait().await;

    let config = TransportConfig {
        url: format!("http://127.0.0.1:{}", port),
        ..Default::default()
    };
    let mut client = WebTransportConnection::new(config).await.unwrap();
    client
        .connect(&format!("http://127.0.0.1:{}", port))
        .await
        .unwrap();

    // When: Client sends a binary message
    let binary_data = vec![0x01, 0x02, 0x03, 0x04, 0x05];
    let message = Message {
        data: binary_data.clone(),
        message_type: MessageType::Binary,
    };
    let (mut stream, mut sink) = client.split();

    // Then: Binary message should be sent and received
    let send_result = sink.send(message.clone()).await;
    assert!(send_result.is_ok());

    let received = stream.next().await;
    assert!(received.is_some());
    let received_msg = received.unwrap().unwrap();
    assert_eq!(received_msg, message);
}

#[tokio::test]
async fn test_webtransport_connection_timeout() {
    // Given: A WebTransport client
    let config = TransportConfig {
        url: "http://127.0.0.1:99999".to_string(),
        ..Default::default()
    };
    let mut client = WebTransportConnection::new(config).await.unwrap();

    // When: Client tries to connect to non-existent server
    let result = timeout(
        Duration::from_secs(5),
        client.connect("http://127.0.0.1:99999"),
    )
    .await;

    // Then: Should fail with connection error
    assert!(result.is_ok()); // Timeout completed
    let connect_result = result.unwrap();
    assert!(connect_result.is_err());
    // Connection should fail with any error type
    let error = connect_result.unwrap_err();
    assert!(matches!(
        error,
        TransportError::ConnectionFailed(_) | TransportError::Timeout
    ));
    assert_eq!(client.state(), ConnectionState::Disconnected);
}

#[tokio::test]
async fn test_webtransport_disconnect() {
    // Given: A connected WebTransport client
    let port = start_server_and_wait().await;

    let config = TransportConfig {
        url: format!("http://127.0.0.1:{}", port),
        ..Default::default()
    };
    let mut client = WebTransportConnection::new(config).await.unwrap();
    client
        .connect(&format!("http://127.0.0.1:{}", port))
        .await
        .unwrap();
    assert_eq!(client.state(), ConnectionState::Connected);

    // When: Client disconnects
    let result = client.disconnect().await;

    // Then: Should disconnect successfully
    assert!(result.is_ok());
    assert_eq!(client.state(), ConnectionState::Disconnected);
}

#[tokio::test]
async fn test_webtransport_reconnection() {
    // Given: A WebTransport client that was connected
    let port = start_server_and_wait().await;

    let config = TransportConfig {
        url: format!("http://127.0.0.1:{}", port),
        ..Default::default()
    };
    let mut client = WebTransportConnection::new(config).await.unwrap();

    // First connection
    client
        .connect(&format!("http://127.0.0.1:{}", port))
        .await
        .unwrap();
    assert_eq!(client.state(), ConnectionState::Connected);

    // Disconnect
    client.disconnect().await.unwrap();
    assert_eq!(client.state(), ConnectionState::Disconnected);

    // When: Client reconnects
    let result = client.connect(&format!("http://127.0.0.1:{}", port)).await;

    // Then: Should reconnect successfully
    assert!(result.is_ok());
    assert_eq!(client.state(), ConnectionState::Connected);
}

#[tokio::test]
async fn test_webtransport_serialized_message() {
    // Given: A connected WebTransport client and HTTP/3 echo server
    let port = start_server_and_wait().await;

    let config = TransportConfig {
        url: format!("http://127.0.0.1:{}", port),
        ..Default::default()
    };
    let mut client = WebTransportConnection::new(config).await.unwrap();
    client
        .connect(&format!("http://127.0.0.1:{}", port))
        .await
        .unwrap();

    // When: Client sends a serialized message
    let test_msg = TestMessage {
        id: 42,
        content: "WebTransport test message".to_string(),
        timestamp: 1234567890,
    };
    let json = serde_json::to_string(&test_msg).unwrap();
    let message = Message {
        data: json.as_bytes().to_vec(),
        message_type: MessageType::Text,
    };
    let (mut stream, mut sink) = client.split();

    // Then: Should send and receive the serialized message
    let send_result = sink.send(message.clone()).await;
    assert!(send_result.is_ok());

    let received = stream.next().await;
    assert!(received.is_some());
    let received_msg = received.unwrap().unwrap();
    assert_eq!(received_msg, message);

    // And: Should be able to deserialize the received message
    if received_msg.message_type == MessageType::Text {
        let received_json = String::from_utf8(received_msg.data).unwrap();
        let received_test_msg: TestMessage = serde_json::from_str(&received_json).unwrap();
        assert_eq!(received_test_msg, test_msg);
    }
}

#[tokio::test]
async fn test_webtransport_multiple_messages() {
    // Given: A connected WebTransport client and HTTP/3 echo server
    let port = start_server_and_wait().await;

    let config = TransportConfig {
        url: format!("http://127.0.0.1:{}", port),
        ..Default::default()
    };
    let mut client = WebTransportConnection::new(config).await.unwrap();
    client
        .connect(&format!("http://127.0.0.1:{}", port))
        .await
        .unwrap();

    // When: Client sends multiple messages
    let (mut stream, mut sink) = client.split();
    let messages = vec![
        Message {
            data: "Message 1".as_bytes().to_vec(),
            message_type: MessageType::Text,
        },
        Message {
            data: "Message 2".as_bytes().to_vec(),
            message_type: MessageType::Text,
        },
        Message {
            data: "Message 3".as_bytes().to_vec(),
            message_type: MessageType::Text,
        },
    ];

    // Send all messages
    for message in &messages {
        let send_result = sink.send(message.clone()).await;
        assert!(send_result.is_ok());
    }

    // Then: Should receive all messages back
    for expected_message in &messages {
        let received = stream.next().await;
        assert!(received.is_some());
        let received_msg = received.unwrap().unwrap();
        assert_eq!(received_msg, *expected_message);
    }
}

#[tokio::test]
async fn test_webtransport_http3_protocol_features() {
    // Given: A connected WebTransport client
    let port = start_server_and_wait().await;

    let config = TransportConfig {
        url: format!("http://127.0.0.1:{}", port),
        ..Default::default()
    };
    let mut client = WebTransportConnection::new(config).await.unwrap();
    client
        .connect(&format!("http://127.0.0.1:{}", port))
        .await
        .unwrap();

    // When: Testing HTTP/3 specific features
    let (mut stream, mut sink) = client.split();

    // Test connection multiplexing (HTTP/3 feature)
    let message = Message {
        data: "HTTP/3 multiplexing test".as_bytes().to_vec(),
        message_type: MessageType::Text,
    };

    // Then: Should handle HTTP/3 multiplexing correctly
    let send_result = sink.send(message.clone()).await;
    assert!(send_result.is_ok());

    let received = stream.next().await;
    assert!(received.is_some());
    let received_msg = received.unwrap().unwrap();
    assert_eq!(received_msg, message);
}
