# AGENTS.md - Leptos WebSocket Pro Development Guide

## Build/Test Commands

- **Run all tests**: `cargo test`
- **Run single test**: `cargo test test_name` or `cargo test --test test_file_name`
- **Quick validation**: `cargo test --test quick_validation`
- **Unit tests**: `cargo test --lib` (for tests in src/)
- **Integration tests**: `cargo test --test real_websocket_tests`
- **E2E tests**: `npm test` (Playwright tests in tests/e2e/)
- **Build**: `cargo build` / `cargo build --release`
- **Check**: `cargo check` (faster than build, good for CI)
- **Format**: `cargo fmt`
- **Lint**: `cargo clippy`

## Architecture & Structure

- **Core library**: Advanced WebSocket/transport library for Leptos framework
- **Multi-transport**: WebSocket, WebTransport (HTTP/3), SSE with adaptive fallback
- **Feature-based compilation**: Features like `client`, `server`, `ssr`, `compression`, `metrics`
- **Key modules**: `transport/` (connection layer), `rpc/` (type-safe calls), `codec/` (serialization), `reactive/` (Leptos integration), `security/` (auth/rate limiting), `performance/` (optimization)
- **Test structure**: `tests/unit/`, `tests/integration/`, `tests/e2e/`, `tests/contract/`

## Code Style & Conventions

- **Imports**: Group std, external crates, then internal modules; use specific imports not globs
- **Error handling**: Use `thiserror` with context-rich errors, `#[error(transparent)]` for chaining
- **Naming**: PascalCase types, snake_case functions/fields, descriptive names like `TransportCapabilities`
- **Documentation**: Triple-slash `///` for public APIs with examples, `//!` for module docs
- **Async patterns**: Use `async-trait`, tokio runtime, proper Send/Sync bounds
- **Testing**: Arrange-Act-Assert pattern, descriptive test names, separate unit/integration tests
- **Types**: Heavy use of generics, serde derives, `Arc<Mutex<>>` for thread-safe sharing
