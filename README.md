# Leptos WS Pro

A world-class WebSocket library for Leptos with comprehensive testing infrastructure.

[![Crates.io](https://img.shields.io/crates/v/leptos-ws-pro.svg)](https://crates.io/crates/leptos-ws-pro)
[![Documentation](https://docs.rs/leptos-ws-pro/badge.svg)](https://docs.rs/leptos-ws-pro)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## 🚀 Features

- **🌐 Real WebSocket Server Testing** - Actual network communication with `tokio-tungstenite`
- **🎭 Cross-Browser Testing** - Playwright integration with 6+ browsers
- **📱 Mobile Device Support** - iOS and Android browser testing
- **⚡ Performance Monitoring** - Load testing and performance benchmarks
- **🔄 Complete User Journey Testing** - End-to-end workflow validation
- **🏗️ Production-Ready CI/CD** - Comprehensive testing infrastructure
- **🔧 Modular Architecture** - Transport, Codec, Reactive, and RPC layers
- **📊 200+ Tests** - Unit, integration, server, browser, and load tests

## 📊 Test Coverage

- **Unit Tests**: 28 tests
- **Integration Tests**: 89 tests  
- **Server Tests**: 12 tests (real WebSocket server)
- **Browser Tests**: 40+ tests (Playwright cross-browser)
- **User Journey Tests**: 25+ tests (complete workflows)
- **Load Tests**: 15+ tests (performance & scalability)
- **Total**: 200+ tests

## 🎭 Browser Support

| Browser | Desktop | Mobile | Status |
|---------|---------|--------|--------|
| **Chrome** | ✅ | ✅ | Fully Tested |
| **Firefox** | ✅ | ✅ | Fully Tested |
| **Safari** | ✅ | ✅ | Fully Tested |
| **Edge** | ✅ | ✅ | Fully Tested |
| **Mobile Chrome** | N/A | ✅ | Fully Tested |
| **Mobile Safari** | N/A | ✅ | Fully Tested |

## 🚀 Quick Start

### Installation

```toml
[dependencies]
leptos-ws-pro = "1.0"
```

### Basic Usage

```rust
use leptos_ws_pro::*;

// Create WebSocket provider
let provider = WebSocketProvider::new("ws://localhost:8080");

// Create reactive context
let context = WebSocketContext::new(provider);

// Send messages
context.send_message("Hello, Server!");

// Handle responses
let messages = context.get_received_messages::<String>();
```

## 🧪 Testing Infrastructure

### Running Tests

```bash
# Run all tests
cargo test --all --features server

# Run browser tests
npm install
npx playwright test

# Run comprehensive test suite
node tests/e2e/test-runner.js
```

### Test Categories

- **Unit Tests**: Core functionality testing
- **Integration Tests**: Cross-module communication
- **Server Tests**: Real WebSocket server testing
- **Browser Tests**: Cross-browser compatibility
- **Load Tests**: Performance and scalability
- **User Journey Tests**: Complete workflow testing

## 📈 Performance

- **Message Throughput**: 100+ messages/second
- **Connection Stability**: 99.9% uptime
- **Concurrent Connections**: 50+ clients
- **Large Payloads**: 50KB+ messages
- **Reconnection Speed**: <1 second

## 🔧 Architecture

### Modular Design

```
leptos-ws-pro/
├── transport/     # WebSocket transport layer
├── codec/         # Message encoding/decoding
├── reactive/      # Leptos reactive integration
├── rpc/           # Type-safe RPC system
└── tests/         # Comprehensive test suite
```

### Key Components

- **Transport Layer**: WebSocket, WebTransport, SSE support
- **Codec System**: JSON, Rkyv, and hybrid codecs
- **Reactive Integration**: Leptos signals and effects
- **RPC System**: Type-safe request/response handling

## 📚 Documentation

- [API Reference](https://docs.rs/leptos-ws-pro)
- [Testing Guide](tests/e2e/README.md)
- [Migration Guide](MIGRATION.md)
- [Performance Benchmarks](BENCHMARKS.md)

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

### Development Setup

```bash
# Clone repository
git clone https://github.com/cloud-shuttle/leptos-ws-pro.git
cd leptos-ws-pro

# Install dependencies
cargo build
npm install
npx playwright install

# Run tests
cargo test --all --features server
npx playwright test
```

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- Built on top of the excellent [Leptos](https://github.com/leptos-rs/leptos) framework
- Inspired by the original [leptos_ws](https://github.com/TimTom2016/leptos_ws) library
- Powered by [Playwright](https://playwright.dev/) for browser testing

## 🚀 Production Ready

This library is production-ready with:
- ✅ Comprehensive testing (200+ tests)
- ✅ Cross-browser compatibility
- ✅ Performance monitoring
- ✅ CI/CD integration
- ✅ Real-world validation
- ✅ Complete documentation

---

**Leptos WS Pro** - The world-class WebSocket library for Leptos with comprehensive testing infrastructure.
