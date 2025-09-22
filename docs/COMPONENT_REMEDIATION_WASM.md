# WASM Support Remediation Plan

## Current Status: ❌ **NOT IMPLEMENTED**

### Problem

No WASM (WebAssembly) support despite having `web-sys` dependencies. Cannot run in browser environments.

### Impact

- Library unusable in browser
- No client-side WebSocket support
- Missing key platform support

## Implementation Plan

### Phase 1: WASM WebSocket Client (Week 1)

#### 1.1 Conditional Compilation Setup

```rust
// src/transport/websocket/wasm.rs
#[cfg(target_arch = "wasm32")]
pub mod wasm_websocket {
    use web_sys::{WebSocket, MessageEvent, CloseEvent, ErrorEvent};
    use wasm_bindgen::prelude::*;
    use wasm_bindgen_futures::JsFuture;

    pub struct WasmWebSocket {
        ws: WebSocket,
        state: ConnectionState,
        message_receiver: mpsc::UnboundedReceiver<Message>,
    }

    impl WasmWebSocket {
        pub fn new(url: &str) -> Result<Self, TransportError> {
            let ws = WebSocket::new(url)
                .map_err(|_| TransportError::ConnectionFailed("Failed to create WebSocket".into()))?;

            Ok(Self {
                ws,
                state: ConnectionState::Disconnected,
                message_receiver: mpsc::unbounded_channel().1,
            })
        }
    }
}
```

#### 1.2 Event Handling

```rust
#[cfg(target_arch = "wasm32")]
impl WasmWebSocket {
    fn setup_event_handlers(&self) -> Result<(), TransportError> {
        let onmessage = Closure::wrap(Box::new(|event: MessageEvent| {
            if let Ok(data) = event.data().dyn_into::<js_sys::JsString>() {
                let message = Message {
                    data: data.as_string().unwrap().into_bytes(),
                    message_type: MessageType::Text,
                };
                // Send to message receiver
            }
        }) as Box<dyn FnMut(_)>);

        self.ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
        onmessage.forget(); // Prevent cleanup

        Ok(())
    }

    fn setup_error_handler(&self) -> Result<(), TransportError> {
        let onerror = Closure::wrap(Box::new(|_event: ErrorEvent| {
            // Handle WebSocket errors
        }) as Box<dyn FnMut(_)>);

        self.ws.set_onerror(Some(onerror.as_ref().unchecked_ref()));
        onerror.forget();

        Ok(())
    }
}
```

### Phase 2: Transport Trait Implementation (Week 1-2)

#### 2.1 WASM Transport Implementation

```rust
#[cfg(target_arch = "wasm32")]
impl Transport for WasmWebSocket {
    async fn connect(&mut self, url: &str) -> Result<(), TransportError> {
        self.ws = WebSocket::new(url)
            .map_err(|_| TransportError::ConnectionFailed("Connection failed".into()))?;

        self.setup_event_handlers()?;
        self.setup_error_handler()?;

        self.state = ConnectionState::Connected;
        Ok(())
    }

    async fn send(&mut self, message: Message) -> Result<(), TransportError> {
        match message.message_type {
            MessageType::Text => {
                let text = String::from_utf8(message.data)
                    .map_err(|_| TransportError::SendFailed("Invalid UTF-8".into()))?;
                self.ws.send_with_str(&text)
                    .map_err(|_| TransportError::SendFailed("Send failed".into()))?;
            }
            MessageType::Binary => {
                let array = js_sys::Uint8Array::new_with_length(message.data.len() as u32);
                array.copy_from(&message.data);
                self.ws.send_with_array_buffer(&array.buffer())
                    .map_err(|_| TransportError::SendFailed("Send failed".into()))?;
            }
            _ => return Err(TransportError::NotSupported("Message type not supported".into())),
        }
        Ok(())
    }

    async fn receive(&mut self) -> Result<Message, TransportError> {
        self.message_receiver.recv().await
            .ok_or_else(|| TransportError::ReceiveFailed("Channel closed".into()))
    }

    fn state(&self) -> ConnectionState {
        self.state
    }
}
```

### Phase 3: Platform Abstraction (Week 2)

#### 3.1 Unified Transport Interface

```rust
// src/transport/websocket/mod.rs
pub enum WebSocketTransport {
    #[cfg(not(target_arch = "wasm32"))]
    Native(NativeWebSocket),
    #[cfg(target_arch = "wasm32")]
    Wasm(WasmWebSocket),
}

impl Transport for WebSocketTransport {
    async fn connect(&mut self, url: &str) -> Result<(), TransportError> {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            WebSocketTransport::Native(ws) => ws.connect(url).await,
            #[cfg(target_arch = "wasm32")]
            WebSocketTransport::Wasm(ws) => ws.connect(url).await,
        }
    }

    async fn send(&mut self, message: Message) -> Result<(), TransportError> {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            WebSocketTransport::Native(ws) => ws.send(message).await,
            #[cfg(target_arch = "wasm32")]
            WebSocketTransport::Wasm(ws) => ws.send(message).await,
        }
    }

    async fn receive(&mut self) -> Result<Message, TransportError> {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            WebSocketTransport::Native(ws) => ws.receive().await,
            #[cfg(target_arch = "wasm32")]
            WebSocketTransport::Wasm(ws) => ws.receive().await,
        }
    }

    fn state(&self) -> ConnectionState {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            WebSocketTransport::Native(ws) => ws.state(),
            #[cfg(target_arch = "wasm32")]
            WebSocketTransport::Wasm(ws) => ws.state(),
        }
    }
}
```

### Phase 4: WASM-Specific Features (Week 2-3)

#### 4.1 Browser API Integration

```rust
#[cfg(target_arch = "wasm32")]
impl WasmWebSocket {
    pub fn get_ready_state(&self) -> ReadyState {
        match self.ws.ready_state() {
            web_sys::ReadyState::Connecting => ReadyState::Connecting,
            web_sys::ReadyState::Open => ReadyState::Open,
            web_sys::ReadyState::Closing => ReadyState::Closing,
            web_sys::ReadyState::Closed => ReadyState::Closed,
            _ => ReadyState::Closed,
        }
    }

    pub fn get_protocol(&self) -> Option<String> {
        self.ws.protocol().ok()
    }

    pub fn get_url(&self) -> String {
        self.ws.url()
    }
}
```

#### 4.2 WASM-Specific Error Handling

```rust
#[cfg(target_arch = "wasm32")]
impl WasmWebSocket {
    fn handle_wasm_error(&mut self, error: JsValue) -> TransportError {
        let error_string = error.as_string().unwrap_or_else(|| "Unknown error".to_string());

        if error_string.contains("connection") {
            TransportError::ConnectionFailed(error_string)
        } else if error_string.contains("timeout") {
            TransportError::Timeout
        } else {
            TransportError::ProtocolError(error_string)
        }
    }
}
```

## Testing Strategy

### Unit Tests

- WASM WebSocket creation
- Event handling
- Message sending/receiving
- Error handling

### Browser Tests

- Real browser WebSocket communication
- Cross-browser compatibility
- Performance testing

### Integration Tests

- WASM + Native transport switching
- Platform detection
- Feature flag testing

## Dependencies Required

```toml
[dependencies]
web-sys = { version = "0.3", features = ["WebSocket", "MessageEvent", "CloseEvent", "ErrorEvent"] }
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
js-sys = "0.3"
```

## Success Criteria

- [ ] WASM WebSocket connects successfully
- [ ] Messages sent/received in browser
- [ ] Platform abstraction working
- [ ] Cross-browser compatibility
- [ ] All tests passing
- [ ] No compilation errors on wasm32 target

## Estimated Effort

- **Development**: 3 weeks
- **Testing**: 1 week
- **Total**: 4 weeks
