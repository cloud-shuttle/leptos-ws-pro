//! WebSocket Echo Server for Integration Testing
//!
//! This module provides a simple WebSocket echo server that can be used
//! for integration testing of the RPC and transport systems.

use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{accept_async, WebSocketStream, MaybeTlsStream};
use futures_util::{SinkExt, StreamExt};
use tungstenite::Message as WsMessage;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::HashMap;

/// WebSocket Echo Server
pub struct EchoServer {
    listener: TcpListener,
    clients: Arc<Mutex<HashMap<usize, tokio::task::JoinHandle<()>>>>,
    next_client_id: Arc<Mutex<usize>>,
    port: u16,
}

impl EchoServer {
    /// Create a new echo server
    pub async fn new(port: u16) -> Result<Self, Box<dyn std::error::Error>> {
        let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).await?;

        Ok(Self {
            listener,
            clients: Arc::new(Mutex::new(HashMap::new())),
            next_client_id: Arc::new(Mutex::new(0)),
            port,
        })
    }

    /// Start the echo server
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Echo server starting on port {}", self.port);

        while let Ok((stream, addr)) = self.listener.accept().await {
            println!("New connection from: {}", addr);

            let ws_stream = accept_async(stream).await?;
            let clients = self.clients.clone();
            let next_id = self.next_client_id.clone();

            let client_id = {
                let mut id = next_id.lock().await;
                *id += 1;
                *id
            };

            let handle = tokio::spawn(async move {
                Self::handle_client(ws_stream, client_id).await;
            });

            clients.lock().await.insert(client_id, handle);
        }

        Ok(())
    }

    /// Handle a WebSocket client connection
    async fn handle_client(
        mut ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
        client_id: usize,
    ) {
        println!("Handling client {}", client_id);

        while let Some(msg) = ws_stream.next().await {
            match msg {
                Ok(WsMessage::Text(text)) => {
                    println!("Client {} sent text: {}", client_id, text);

                    // Echo back the message
                    if let Err(e) = ws_stream.send(WsMessage::Text(text)).await {
                        eprintln!("Error sending text message to client {}: {}", client_id, e);
                        break;
                    }
                }
                Ok(WsMessage::Binary(data)) => {
                    println!("Client {} sent binary data: {} bytes", client_id, data.len());

                    // Echo back binary data
                    if let Err(e) = ws_stream.send(WsMessage::Binary(data)).await {
                        eprintln!("Error sending binary message to client {}: {}", client_id, e);
                        break;
                    }
                }
                Ok(WsMessage::Ping(data)) => {
                    println!("Client {} sent ping", client_id);
                    if let Err(e) = ws_stream.send(WsMessage::Pong(data)).await {
                        eprintln!("Error sending pong to client {}: {}", client_id, e);
                        break;
                    }
                }
                Ok(WsMessage::Close(_)) => {
                    println!("Client {} closed connection", client_id);
                    break;
                }
                Err(e) => {
                    eprintln!("WebSocket error for client {}: {}", client_id, e);
                    break;
                }
                _ => {}
            }
        }

        println!("Client {} disconnected", client_id);
    }

    /// Get the server URL
    pub fn url(&self) -> String {
        format!("ws://127.0.0.1:{}", self.port)
    }

    /// Stop the server and clean up clients
    pub async fn stop(&self) {
        let mut clients = self.clients.lock().await;
        for (client_id, handle) in clients.drain() {
            println!("Stopping client {}", client_id);
            handle.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_echo_server_creation() {
        let server = EchoServer::new(8080).await;
        assert!(server.is_ok());
    }

    #[tokio::test]
    async fn test_echo_server_url() {
        let server = EchoServer::new(8081).await.unwrap();
        assert_eq!(server.url(), "ws://127.0.0.1:8081");
    }
}
