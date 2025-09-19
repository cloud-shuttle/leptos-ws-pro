//! RPC Test Server for Integration Testing
//!
//! This module provides an RPC server that can handle JSON-RPC requests
//! and return appropriate responses for testing the RPC system.

use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{accept_async, WebSocketStream, MaybeTlsStream};
use futures_util::{SinkExt, StreamExt};
use tungstenite::Message as WsMessage;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// RPC method handler function type
type RpcMethodHandler = Box<dyn Fn(Value) -> Result<Value, String> + Send + Sync>;

/// RPC Test Server
pub struct RpcServer {
    listener: TcpListener,
    methods: Arc<Mutex<HashMap<String, RpcMethodHandler>>>,
    port: u16,
}

/// JSON-RPC Request structure
#[derive(serde::Deserialize, Debug)]
struct RpcRequest {
    id: Value,
    method: String,
    params: Value,
}

/// JSON-RPC Response structure
#[derive(serde::Serialize, Debug)]
struct RpcResponse {
    id: Value,
    result: Option<Value>,
    error: Option<RpcError>,
}

/// JSON-RPC Error structure
#[derive(serde::Serialize, Debug)]
struct RpcError {
    code: i32,
    message: String,
    data: Option<Value>,
}

impl RpcServer {
    /// Create a new RPC server
    pub async fn new(port: u16) -> Result<Self, Box<dyn std::error::Error>> {
        let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).await?;

        let mut server = Self {
            listener,
            methods: Arc::new(Mutex::new(HashMap::new())),
            port,
        };

        // Register default RPC methods
        server.register_method("echo", Self::echo_handler).await;
        server.register_method("add", Self::add_handler).await;
        server.register_method("get_time", Self::get_time_handler).await;
        server.register_method("error_test", Self::error_handler).await;

        Ok(server)
    }

    /// Register an RPC method
    async fn register_method<F>(&self, name: &str, handler: F)
    where
        F: Fn(Value) -> Result<Value, String> + Send + Sync + 'static,
    {
        self.methods.lock().await.insert(name.to_string(), Box::new(handler));
    }

    /// Start the RPC server
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("RPC server starting on port {}", self.port);

        while let Ok((stream, addr)) = self.listener.accept().await {
            println!("New RPC connection from: {}", addr);

            let ws_stream = accept_async(stream).await?;
            let methods = self.methods.clone();

            tokio::spawn(async move {
                Self::handle_rpc_client(ws_stream, methods).await;
            });
        }

        Ok(())
    }

    /// Handle an RPC client connection
    async fn handle_rpc_client(
        mut ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
        methods: Arc<Mutex<HashMap<String, RpcMethodHandler>>>,
    ) {
        while let Some(msg) = ws_stream.next().await {
            match msg {
                Ok(WsMessage::Text(text)) => {
                    // Parse JSON-RPC request
                    match serde_json::from_str::<RpcRequest>(&text) {
                        Ok(request) => {
                            println!("Received RPC request: {} with params: {:?}", request.method, request.params);

                            let response = Self::handle_rpc_request(request, &methods).await;

                            // Send response
                            match serde_json::to_string(&response) {
                                Ok(response_json) => {
                                    if let Err(e) = ws_stream.send(WsMessage::Text(response_json)).await {
                                        eprintln!("Error sending RPC response: {}", e);
                                        break;
                                    }
                                }
                                Err(e) => {
                                    eprintln!("Error serializing RPC response: {}", e);
                                    break;
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Error parsing RPC request: {}", e);

                            // Send error response
                            let error_response = RpcResponse {
                                id: json!(null),
                                result: None,
                                error: Some(RpcError {
                                    code: -32700,
                                    message: "Parse error".to_string(),
                                    data: Some(json!(e.to_string())),
                                }),
                            };

                            if let Ok(response_json) = serde_json::to_string(&error_response) {
                                let _ = ws_stream.send(WsMessage::Text(response_json)).await;
                            }
                        }
                    }
                }
                Ok(WsMessage::Close(_)) => {
                    println!("RPC client closed connection");
                    break;
                }
                Err(e) => {
                    eprintln!("RPC WebSocket error: {}", e);
                    break;
                }
                _ => {}
            }
        }
    }

    /// Handle an RPC request
    async fn handle_rpc_request(
        request: RpcRequest,
        methods: &Arc<Mutex<HashMap<String, RpcMethodHandler>>>,
    ) -> RpcResponse {
        let methods = methods.lock().await;

        if let Some(handler) = methods.get(&request.method) {
            match handler(request.params) {
                Ok(result) => RpcResponse {
                    id: request.id,
                    result: Some(result),
                    error: None,
                },
                Err(error) => RpcResponse {
                    id: request.id,
                    result: None,
                    error: Some(RpcError {
                        code: -32603,
                        message: "Internal error".to_string(),
                        data: Some(json!(error)),
                    }),
                },
            }
        } else {
            RpcResponse {
                id: request.id,
                result: None,
                error: Some(RpcError {
                    code: -32601,
                    message: "Method not found".to_string(),
                    data: None,
                }),
            }
        }
    }

    /// Echo handler - returns the same parameters
    fn echo_handler(params: Value) -> Result<Value, String> {
        Ok(params)
    }

    /// Add handler - adds two numbers
    fn add_handler(params: Value) -> Result<Value, String> {
        if let (Some(a), Some(b)) = (
            params.get("a").and_then(|v| v.as_i64()),
            params.get("b").and_then(|v| v.as_i64()),
        ) {
            Ok(json!({ "result": a + b }))
        } else {
            Err("Invalid parameters: expected 'a' and 'b' as integers".to_string())
        }
    }

    /// Get time handler - returns current timestamp
    fn get_time_handler(_params: Value) -> Result<Value, String> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Ok(json!({ "timestamp": timestamp }))
    }

    /// Error handler - always returns an error for testing
    fn error_handler(_params: Value) -> Result<Value, String> {
        Err("This is a test error".to_string())
    }

    /// Get the server URL
    pub fn url(&self) -> String {
        format!("ws://127.0.0.1:{}", self.port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rpc_server_creation() {
        let server = RpcServer::new(8082).await;
        assert!(server.is_ok());
    }

    #[tokio::test]
    async fn test_rpc_server_url() {
        let server = RpcServer::new(8083).await.unwrap();
        assert_eq!(server.url(), "ws://127.0.0.1:8083");
    }

    #[test]
    fn test_echo_handler() {
        let params = json!({"message": "Hello, RPC!"});
        let result = RpcServer::echo_handler(params.clone());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), params);
    }

    #[test]
    fn test_add_handler() {
        let params = json!({"a": 5, "b": 3});
        let result = RpcServer::add_handler(params);
        assert!(result.is_ok());
        assert_eq!(result.unwrap()["result"], 8);
    }

    #[test]
    fn test_error_handler() {
        let params = json!({});
        let result = RpcServer::error_handler(params);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "This is a test error");
    }
}
