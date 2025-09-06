//! Test WebSocket server for integration testing
//!
//! This module provides a real WebSocket server that can be used
//! for testing the leptos_ws library with actual network communication.

use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, RwLock};
use tokio_tungstenite::accept_async;
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Test WebSocket server for integration testing
pub struct TestWebSocketServer {
    addr: SocketAddr,
    server_handle: tokio::task::JoinHandle<()>,
    shutdown_tx: broadcast::Sender<()>,
    connected_clients: Arc<RwLock<HashMap<String, ClientInfo>>>,
}

#[derive(Debug, Clone)]
struct ClientInfo {
    id: String,
    connected_at: Instant,
    message_count: u64,
}

/// Message types for the test server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    Welcome { client_id: String },
    Echo { message: String, timestamp: u64 },
    Broadcast { from: String, message: String },
    Error { error: String },
    Heartbeat,
}

/// Client message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMessage {
    Echo { message: String },
    Broadcast { message: String },
    Heartbeat,
    GetStats,
}

impl TestWebSocketServer {
    /// Create a new test WebSocket server
    pub async fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;

        let (shutdown_tx, _) = broadcast::channel(1);
        let connected_clients = Arc::new(RwLock::new(HashMap::new()));

        let server_handle = {
            let shutdown_rx = shutdown_tx.subscribe();
            let clients = Arc::clone(&connected_clients);

            tokio::spawn(async move {
                Self::run_server(listener, shutdown_rx, clients).await;
            })
        };

        // Give the server a moment to start
        tokio::time::sleep(Duration::from_millis(100)).await;

        Ok(TestWebSocketServer {
            addr,
            server_handle,
            shutdown_tx,
            connected_clients,
        })
    }

    /// Get the server URL
    pub fn url(&self) -> String {
        format!("ws://{}", self.addr)
    }

    /// Get the number of connected clients
    pub async fn connected_clients_count(&self) -> usize {
        self.connected_clients.read().await.len()
    }

    /// Shutdown the server
    pub async fn shutdown(self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let _ = self.shutdown_tx.send(());
        self.server_handle.await?;
        Ok(())
    }

    /// Run the WebSocket server
    async fn run_server(
        listener: TcpListener,
        mut shutdown_rx: broadcast::Receiver<()>,
        connected_clients: Arc<RwLock<HashMap<String, ClientInfo>>>,
    ) {
        let mut client_counter = 0u64;

        loop {
            tokio::select! {
                // Accept new connections
                result = listener.accept() => {
                    match result {
                        Ok((stream, addr)) => {
                            client_counter += 1;
                            let client_id = format!("client_{}", client_counter);

                            // Add client to tracking
                            {
                                let mut clients = connected_clients.write().await;
                                clients.insert(client_id.clone(), ClientInfo {
                                    id: client_id.clone(),
                                    connected_at: Instant::now(),
                                    message_count: 0,
                                });
                            }

                            // Handle the connection
                            let clients = Arc::clone(&connected_clients);
                            tokio::spawn(async move {
                                if let Err(e) = Self::handle_connection(stream, client_id.clone(), clients).await {
                                    eprintln!("Error handling connection from {}: {}", addr, e);
                                }
                            });
                        }
                        Err(e) => {
                            eprintln!("Error accepting connection: {}", e);
                        }
                    }
                }

                // Shutdown signal
                _ = shutdown_rx.recv() => {
                    break;
                }
            }
        }
    }

    /// Handle a WebSocket connection
    async fn handle_connection(
        stream: TcpStream,
        client_id: String,
        connected_clients: Arc<RwLock<HashMap<String, ClientInfo>>>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let ws_stream = accept_async(stream).await?;
        let (mut ws_sender, mut ws_receiver) = ws_stream.split();

        // Send welcome message
        let welcome = ServerMessage::Welcome {
            client_id: client_id.clone(),
        };
        let welcome_json = serde_json::to_string(&welcome)?;
        ws_sender.send(tokio_tungstenite::tungstenite::Message::Text(welcome_json)).await?;

        // Handle incoming messages
        while let Some(msg) = ws_receiver.next().await {
            match msg? {
                tokio_tungstenite::tungstenite::Message::Text(text) => {
                    // Parse client message
                    if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                        let response = Self::handle_client_message(&client_msg, &client_id).await;
                        let response_json = serde_json::to_string(&response)?;
                        ws_sender.send(tokio_tungstenite::tungstenite::Message::Text(response_json)).await?;

                        // Update client stats
                        {
                            let mut clients = connected_clients.write().await;
                            if let Some(client_info) = clients.get_mut(&client_id) {
                                client_info.message_count += 1;
                            }
                        }
                    } else {
                        // Echo back the raw text
                        let echo = ServerMessage::Echo {
                            message: text.clone(),
                            timestamp: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_secs(),
                        };
                        let echo_json = serde_json::to_string(&echo)?;
                        ws_sender.send(tokio_tungstenite::tungstenite::Message::Text(echo_json)).await?;
                    }
                }
                tokio_tungstenite::tungstenite::Message::Close(_) => {
                    break;
                }
                tokio_tungstenite::tungstenite::Message::Ping(data) => {
                    ws_sender.send(tokio_tungstenite::tungstenite::Message::Pong(data)).await?;
                }
                _ => {}
            }
        }

        // Remove client from tracking
        {
            let mut clients = connected_clients.write().await;
            clients.remove(&client_id);
        }

        Ok(())
    }

    /// Handle client messages and generate responses
    async fn handle_client_message(
        client_msg: &ClientMessage,
        client_id: &str,
    ) -> ServerMessage {
        match client_msg {
            ClientMessage::Echo { message } => {
                ServerMessage::Echo {
                    message: message.clone(),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                }
            }
            ClientMessage::Broadcast { message } => {
                ServerMessage::Broadcast {
                    from: client_id.to_string(),
                    message: message.clone(),
                }
            }
            ClientMessage::Heartbeat => {
                ServerMessage::Heartbeat
            }
            ClientMessage::GetStats => {
                ServerMessage::Echo {
                    message: format!("Stats for {}: OK", client_id),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_server_creation() {
        let server = TestWebSocketServer::new().await.unwrap();
        assert!(!server.url().is_empty());
        assert!(server.url().starts_with("ws://"));

        server.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_server_url_format() {
        let server = TestWebSocketServer::new().await.unwrap();
        let url = server.url();

        assert!(url.starts_with("ws://127.0.0.1:"));
        assert!(url.contains("127.0.0.1"));

        server.shutdown().await.unwrap();
    }
}
