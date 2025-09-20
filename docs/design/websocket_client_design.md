# WebSocket Client Design

## Overview

Focused design for WebSocket client implementation supporting both native and WASM environments.

## Architecture

### Core Client Structure

```rust
pub struct WebSocketClient {
    connection: Arc<Mutex<Option<Connection>>>,
    config: WebSocketConfig,
    state: ConnectionState,
    event_handlers: EventHandlers,
}

enum Connection {
    #[cfg(not(target_arch = "wasm32"))]
    Native(tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<TcpStream>>),

    #[cfg(target_arch = "wasm32")]
    Wasm(web_sys::WebSocket),
}
```

### Configuration

```rust
#[derive(Debug, Clone)]
pub struct WebSocketConfig {
    pub url: String,
    pub protocols: Vec<String>,
    pub headers: HashMap<String, String>,
    pub connect_timeout: Duration,
    pub heartbeat_interval: Option<Duration>,
    pub max_message_size: usize,
    pub compression: bool,
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            protocols: vec![],
            headers: HashMap::new(),
            connect_timeout: Duration::from_secs(30),
            heartbeat_interval: Some(Duration::from_secs(30)),
            max_message_size: 64 * 1024 * 1024, // 64MB
            compression: true,
        }
    }
}
```

### Connection States

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting { attempt: u32, next_retry: Instant },
    Failed { error: Arc<WebSocketError>, recoverable: bool },
}
```

## Platform-Specific Implementation

### Native Implementation (Tokio + tungstenite)

```rust
#[cfg(not(target_arch = "wasm32"))]
impl WebSocketClient {
    async fn connect_native(&mut self) -> Result<(), WebSocketError> {
        let request = tungstenite::client::IntoClientRequest::into_client_request(&self.config.url)?;

        let (ws_stream, _response) = tokio_tungstenite::connect_async_with_config(
            request,
            Some(self.build_tls_config()?),
            Some(self.build_websocket_config()),
        ).await?;

        let (sink, stream) = ws_stream.split();

        // Spawn reader and writer tasks
        self.spawn_reader_task(stream);
        self.spawn_writer_task(sink);

        self.state = ConnectionState::Connected;
        Ok(())
    }
}
```

### WASM Implementation (web-sys)

```rust
#[cfg(target_arch = "wasm32")]
impl WebSocketClient {
    async fn connect_wasm(&mut self) -> Result<(), WebSocketError> {
        let ws = web_sys::WebSocket::new(&self.config.url)?;

        // Set protocols if specified
        if !self.config.protocols.is_empty() {
            let protocols = js_sys::Array::new();
            for protocol in &self.config.protocols {
                protocols.push(&JsValue::from_str(protocol));
            }
            ws.set_protocols(&protocols)?;
        }

        // Set up event handlers
        self.setup_wasm_event_handlers(&ws)?;

        *self.connection.lock().await = Some(Connection::Wasm(ws));
        self.state = ConnectionState::Connecting;

        Ok(())
    }
}
```

## Message Handling

### Message Types

```rust
#[derive(Debug, Clone)]
pub enum WebSocketMessage {
    Text(String),
    Binary(Vec<u8>),
    Ping(Vec<u8>),
    Pong(Vec<u8>),
    Close(Option<CloseFrame>),
}

#[derive(Debug, Clone)]
pub struct CloseFrame {
    pub code: u16,
    pub reason: String,
}
```

### Event Handlers

```rust
pub struct EventHandlers {
    pub on_open: Option<Box<dyn Fn() + Send + Sync>>,
    pub on_message: Option<Box<dyn Fn(WebSocketMessage) + Send + Sync>>,
    pub on_close: Option<Box<dyn Fn(CloseFrame) + Send + Sync>>,
    pub on_error: Option<Box<dyn Fn(WebSocketError) + Send + Sync>>,
}
```

## Error Handling

### Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum WebSocketError {
    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Message too large: {size} bytes (max: {max})")]
    MessageTooLarge { size: usize, max: usize },

    #[error("Connection closed: {code} - {reason}")]
    Closed { code: u16, reason: String },

    #[error("Timeout: {operation}")]
    Timeout { operation: String },

    #[cfg(not(target_arch = "wasm32"))]
    #[error("Tungstenite error: {0}")]
    Tungstenite(#[from] tokio_tungstenite::tungstenite::Error),

    #[cfg(target_arch = "wasm32")]
    #[error("WebAPI error: {0}")]
    WebApi(String),
}
```

## Reconnection Logic

### Exponential Backoff

```rust
pub struct ReconnectConfig {
    pub max_attempts: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub multiplier: f64,
    pub jitter: bool,
}

impl WebSocketClient {
    async fn attempt_reconnect(&mut self) -> Result<(), WebSocketError> {
        if let ConnectionState::Reconnecting { attempt, next_retry } = &self.state {
            if Instant::now() < *next_retry {
                return Ok(()); // Wait for retry time
            }

            if *attempt >= self.reconnect_config.max_attempts {
                self.state = ConnectionState::Failed {
                    error: Arc::new(WebSocketError::Connection("Max reconnect attempts exceeded".into())),
                    recoverable: false
                };
                return Err(WebSocketError::Connection("Reconnection failed".into()));
            }
        }

        match self.connect_internal().await {
            Ok(_) => {
                self.state = ConnectionState::Connected;
                Ok(())
            }
            Err(err) => {
                let next_delay = self.calculate_backoff_delay(attempt);
                self.state = ConnectionState::Reconnecting {
                    attempt: attempt + 1,
                    next_retry: Instant::now() + next_delay,
                };
                Err(err)
            }
        }
    }
}
```

## Testing Strategy

### Unit Tests

- Connection establishment (native + WASM)
- Message sending/receiving
- Error handling scenarios
- Reconnection logic
- Configuration validation

### Integration Tests

- Real WebSocket server connection
- Large message handling
- Connection failure recovery
- Performance under load

## Key Design Decisions

1. **Unified API**: Same interface for native and WASM
2. **Async First**: All operations are async
3. **Error Recovery**: Automatic reconnection with backoff
4. **Platform Conditional**: Compile-time platform selection
5. **Event-Driven**: Callback-based event handling

## File Size Target: <300 lines per implementation file

- `websocket/client.rs` - Core client logic
- `websocket/native.rs` - Native implementation
- `websocket/wasm.rs` - WASM implementation
- `websocket/config.rs` - Configuration types
- `websocket/errors.rs` - Error definitions
