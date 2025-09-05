# Examples

This directory contains practical examples demonstrating how to use the Leptos WS Pro library.

## 📚 Example Categories

### **Basic Examples**
- [Simple WebSocket Connection](basic/websocket-connection.rs) - Basic WebSocket setup
- [Message Sending](basic/message-sending.rs) - Send and receive messages
- [Connection Management](basic/connection-management.rs) - Handle connection lifecycle

### **Advanced Examples**
- [RPC Client](advanced/rpc-client.rs) - Type-safe RPC communication
- [Reactive Integration](advanced/reactive-integration.rs) - Leptos reactive patterns
- [Error Handling](advanced/error-handling.rs) - Comprehensive error handling
- [Performance Monitoring](advanced/performance-monitoring.rs) - Monitor connection quality

### **Real-World Examples**
- [Chat Application](real-world/chat-app.rs) - Complete chat application
- [Real-time Dashboard](real-world/dashboard.rs) - Live data dashboard
- [Collaborative Editor](real-world/collaborative-editor.rs) - Real-time collaboration
- [Gaming Application](real-world/gaming-app.rs) - Multiplayer game

### **Integration Examples**
- [Axum Integration](integration/axum-server.rs) - Axum WebSocket server
- [Leptos SSR](integration/leptos-ssr.rs) - Server-side rendering
- [Mobile App](integration/mobile-app.rs) - Mobile application

## 🚀 Quick Start Examples

### **Basic WebSocket Connection**
```rust
use leptos_ws_pro::*;

#[component]
pub fn App() -> impl IntoView {
    let provider = WebSocketProvider::new("ws://localhost:8080");
    let context = WebSocketContext::new(provider);
    
    view! {
        <div>
            <button on:click=move |_| {
                context.send_message("Hello, Server!");
            }>
                "Send Message"
            </button>
        </div>
    }
}
```

### **RPC Communication**
```rust
use leptos_ws_pro::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct GetUserRequest {
    user_id: u32,
}

#[derive(Serialize, Deserialize)]
struct User {
    id: u32,
    name: String,
    email: String,
}

#[component]
pub fn UserProfile() -> impl IntoView {
    let provider = WebSocketProvider::new("ws://localhost:8080");
    let context = WebSocketContext::new(provider);
    let rpc_client: RpcClient<GetUserRequest> = RpcClient::new(context);
    
    let get_user = move |user_id: u32| {
        let request = GetUserRequest { user_id };
        rpc_client.call("get_user", request);
    };
    
    view! {
        <div>
            <button on:click=move |_| get_user(123)>
                "Get User"
            </button>
        </div>
    }
}
```

## 📖 Example Structure

Each example includes:

- **Complete working code** - Ready to run
- **Detailed comments** - Explaining each step
- **Error handling** - Proper error management
- **Best practices** - Following library conventions
- **Documentation** - Usage instructions

## 🎯 Learning Path

### **Beginner**
1. [Simple WebSocket Connection](basic/websocket-connection.rs)
2. [Message Sending](basic/message-sending.rs)
3. [Connection Management](basic/connection-management.rs)

### **Intermediate**
1. [RPC Client](advanced/rpc-client.rs)
2. [Reactive Integration](advanced/reactive-integration.rs)
3. [Error Handling](advanced/error-handling.rs)

### **Advanced**
1. [Chat Application](real-world/chat-app.rs)
2. [Real-time Dashboard](real-world/dashboard.rs)
3. [Collaborative Editor](real-world/collaborative-editor.rs)

## 🔧 Running Examples

### **Prerequisites**
```bash
# Install dependencies
cargo build

# Start WebSocket server (if needed)
cargo run --example server
```

### **Run Examples**
```bash
# Run specific example
cargo run --example websocket-connection

# Run with features
cargo run --example chat-app --features server

# Run in debug mode
cargo run --example dashboard --features server -- --debug
```

## 📚 Additional Resources

- [API Reference](../docs/api-reference.md)
- [Architecture Guide](../docs/architecture.md)
- [Performance Guide](../docs/performance.md)
- [Testing Guide](../tests/README.md)

## 🤝 Contributing Examples

We welcome contributions of new examples! Please see our [Contributing Guide](../docs/contributing.md) for details.

### **Example Guidelines**
- Include complete, working code
- Add detailed comments
- Follow library conventions
- Include error handling
- Test examples thoroughly
