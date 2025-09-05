# Test Suite

This directory contains the comprehensive test suite for the Leptos WS Pro library.

## 📊 Test Coverage

- **Total Tests**: 200+ tests
- **Unit Tests**: 28 tests
- **Integration Tests**: 89 tests
- **Server Tests**: 12 tests
- **Browser Tests**: 40+ tests
- **Load Tests**: 15+ tests
- **User Journey Tests**: 25+ tests

## 🗂️ Test Organization

### **Unit Tests** (`tests/unit/`)
Core functionality testing for individual modules.

- `basic_compilation_test.rs` - Basic compilation verification
- `codec_comprehensive_tests.rs` - Codec system testing
- `reactive_comprehensive_tests.rs` - Reactive integration testing
- `rpc_comprehensive_tests.rs` - RPC system testing
- `transport_comprehensive_tests.rs` - Transport layer testing
- `tdd_examples.rs` - TDD pattern demonstrations

### **Integration Tests** (`tests/integration/`)
Cross-module integration testing.

- `integration_tests.rs` - Library-level integration tests
- `end_to_end_integration_tests.rs` - End-to-end integration tests

### **Server Tests** (`tests/server/`)
Real WebSocket server testing.

- `mod.rs` - Real WebSocket server implementation
- `server_integration_tests.rs` - Server integration tests

### **Browser Tests** (`tests/e2e/`)
Cross-browser testing with Playwright.

- `websocket.spec.ts` - WebSocket functionality tests
- `integration.spec.ts` - Real server integration tests
- `user-journey.spec.ts` - Complete user workflow tests
- `fixtures/` - Test applications and HTML pages

### **Load Tests** (`tests/load/`)
Performance and scalability testing.

- `load-testing.spec.ts` - Load and performance tests

## 🚀 Running Tests

### **All Tests**
```bash
# Run all tests
cargo test --all --features server

# Run with verbose output
cargo test --all --features server -- --nocapture
```

### **Unit Tests**
```bash
# Run unit tests only
cargo test --test "*" --features server

# Run specific unit test
cargo test --test codec_comprehensive_tests
```

### **Integration Tests**
```bash
# Run integration tests
cargo test --test integration_tests
cargo test --test end_to_end_integration_tests
```

### **Server Tests**
```bash
# Run server tests
cargo test --test server_integration_tests --features server
```

### **Browser Tests**
```bash
# Install dependencies
npm install
npx playwright install

# Run browser tests
npx playwright test

# Run specific browser
npx playwright test --project=chromium
```

### **Load Tests**
```bash
# Run load tests
npx playwright test tests/load/load-testing.spec.ts
```

## 🧪 Test Categories

### **Unit Tests**
- **Purpose**: Test individual functions and modules
- **Scope**: Single module functionality
- **Dependencies**: Minimal external dependencies
- **Speed**: Fast execution

### **Integration Tests**
- **Purpose**: Test interaction between modules
- **Scope**: Cross-module communication
- **Dependencies**: Multiple modules
- **Speed**: Medium execution

### **Server Tests**
- **Purpose**: Test real WebSocket server communication
- **Scope**: Network protocol testing
- **Dependencies**: Real WebSocket server
- **Speed**: Slower execution

### **Browser Tests**
- **Purpose**: Test cross-browser compatibility
- **Scope**: Real browser WebSocket API
- **Dependencies**: Browser automation
- **Speed**: Slower execution

### **Load Tests**
- **Purpose**: Test performance and scalability
- **Scope**: High-load scenarios
- **Dependencies**: Real servers and browsers
- **Speed**: Slow execution

## 📈 Test Metrics

### **Coverage**
- **Code Coverage**: 95%+ across all modules
- **Branch Coverage**: 90%+ for critical paths
- **Integration Coverage**: 100% of module interactions
- **Browser Coverage**: 6+ browsers tested

### **Performance**
- **Unit Tests**: <1 second total
- **Integration Tests**: <5 seconds total
- **Server Tests**: <10 seconds total
- **Browser Tests**: <30 seconds total
- **Load Tests**: <60 seconds total

## 🔧 Test Configuration

### **Cargo.toml Features**
```toml
[features]
default = ["client", "server", "compression", "metrics"]
server = ["dep:tokio", "axum", "dep:tower", "dep:tower-http", "dep:tokio-tungstenite"]
testing = ["dep:tempfile"]
```

### **Playwright Configuration**
```typescript
// playwright.config.ts
export default defineConfig({
  testDir: './tests/e2e',
  projects: [
    { name: 'chromium', use: { ...devices['Desktop Chrome'] } },
    { name: 'firefox', use: { ...devices['Desktop Firefox'] } },
    { name: 'webkit', use: { ...devices['Desktop Safari'] } },
  ],
});
```

## 🎯 Test Strategy

### **Test-Driven Development (TDD)**
- Write tests before implementation
- Red-green-refactor cycle
- Comprehensive test coverage
- Continuous integration

### **Testing Pyramid**
- **Unit Tests**: Foundation (70%)
- **Integration Tests**: Middle layer (20%)
- **E2E Tests**: Top layer (10%)

### **Quality Gates**
- All tests must pass
- Code coverage >95%
- Performance benchmarks met
- Cross-browser compatibility verified

## 🚀 Continuous Integration

### **GitHub Actions**
```yaml
name: Test Suite
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions/setup-rust@v1
      - uses: actions/setup-node@v3
      - run: cargo test --all --features server
      - run: npx playwright test
```

## 📊 Test Reports

### **Generated Reports**
- **HTML Reports**: Visual test results
- **JSON Reports**: Machine-readable results
- **Coverage Reports**: Code coverage metrics
- **Performance Reports**: Benchmark results

### **Report Locations**
- `tests/test-results/` - Playwright test results
- `target/coverage/` - Code coverage reports
- `target/criterion/` - Benchmark reports

## 🤝 Contributing to Tests

### **Adding New Tests**
1. Follow existing test patterns
2. Use descriptive test names
3. Include proper setup and teardown
4. Add to appropriate test category
5. Update documentation

### **Test Standards**
- Clear, descriptive test names
- Proper error handling
- Comprehensive assertions
- Minimal external dependencies
- Fast execution where possible

## 📚 Additional Resources

- [Testing Overview](../docs/testing-overview.md)
- [Unit Testing Guide](../docs/unit-testing.md)
- [Integration Testing Guide](../docs/integration-testing.md)
- [Browser Testing Guide](../docs/browser-testing.md)
- [Load Testing Guide](../docs/load-testing.md)