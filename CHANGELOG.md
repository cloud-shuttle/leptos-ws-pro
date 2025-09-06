# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0-alpha] - 2025-09-06

### 🎉 Major Milestone: Real WebSocket Implementation Complete!

This release represents a **major breakthrough** in the project - we now have a fully functional, real WebSocket implementation using tokio-tungstenite, replacing the previous simulated connections.

### Added

- **Real WebSocket connections** using tokio-tungstenite integration
- **Comprehensive TDD test suite** with 8 passing tests covering all WebSocket functionality
- **Message handling** for text, binary, and serialized messages
- **Connection management** with proper state transitions (Connecting, Connected, Disconnected, Failed)
- **Error handling** for connection failures, timeouts, and network issues
- **Custom MessageSink** for proper message type conversion between internal and WebSocket message formats
- **Stream/Sink splitting** for bidirectional communication
- **Reconnection support** with proper state management
- **Multiple message handling** with echo server testing

### Changed

- **Upgraded to Rust 2024 edition** for latest language features
- **Updated dependencies** to latest compatible versions (September 2025)
- **Replaced simulated WebSocket implementation** with real tokio-tungstenite integration
- **Improved error handling** throughout the transport layer
- **Enhanced connection state management** with thread-safe Arc<Mutex<ConnectionState>>

### Fixed

- **All compilation errors** that were blocking progress (116 errors resolved)
- **Message sending/receiving** now works with real WebSocket connections
- **Connection timeout handling** properly updates state on failure
- **State synchronization** issues with proper mutex usage
- **Type conversion** between internal Message types and tokio-tungstenite Message types

### Technical Details

- **Test Coverage**: 8/8 WebSocket tests passing (100% success rate)
- **Real Network Communication**: Actual WebSocket servers and clients
- **Bidirectional Message Flow**: Verified with echo server testing
- **Error Propagation**: Proper error handling throughout the stack
- **Thread Safety**: Arc<Mutex<ConnectionState>> for safe concurrent access

### 🚀 What's Next

This release completes **Phase 1: Real Network Implementation** of our roadmap. The foundation is now solid for:

- WebTransport implementation with HTTP/3 support
- Server-Sent Events (SSE) with event stream parsing
- Adaptive transport with automatic capability detection
- Integration with existing RPC system

## [0.7.8] - 2024-03-25

### Changed

- Now support leptos 0.7.8
- Changed codee to 0.3

## [0.7.7] - 2024-03-02

### Changed

- Now support leptos 0.7.7

## [0.7.0-rc1] - 2024-11-16

### Changed

- Now support rc of leptos

### Fixed

- Fixed Issues with Reconnects

## [0.7.0-beta5] - 2024-09-28

### Changed

- Now support beta5 of leptos

### Fixed

- Fixed Issues with Hydration

## [0.7.0-beta4.1] - 2024-09-02

### Changed

- Use [leptos-use](https://leptos-use.rs/) instead of own client websocket implementation
