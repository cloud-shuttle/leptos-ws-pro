# Dependency Update Plan - September 2025

## Current vs Latest Versions

### Critical Security Updates Needed

| Crate             | Current | Latest | Security Risk | Priority |
| ----------------- | ------- | ------ | ------------- | -------- |
| leptos            | 0.8.8   | 0.11+  | Medium        | P1       |
| tokio             | 1.47    | 1.52+  | HIGH          | P0       |
| tokio-tungstenite | 0.27    | 0.34+  | HIGH          | P0       |
| gloo-net          | 0.6     | 0.10+  | HIGH          | P0       |
| reqwest           | 0.12    | 0.13+  | Medium        | P1       |
| ring              | 0.17    | 0.18+  | Low           | P2       |

## Update Strategy

### Phase 1: Critical Security (This Week)

```toml
# High-priority security updates
tokio = "1.52"
tokio-tungstenite = "0.34"
gloo-net = "0.10"
```

### Phase 2: Framework Updates (Next Week)

```toml
# Leptos ecosystem updates (breaking changes expected)
leptos = "0.11"
leptos-use = "0.18+"
```

### Phase 3: Supporting Crates (Following Week)

```toml
# Remaining dependency updates
reqwest = "0.13"
ring = "0.18"
serde = "1.0.210"
```

## Compatibility Matrix

### Rust Edition

- **Current**: "2024" (INVALID - nightly only)
- **Recommended**: "2021" (stable)
- **Action**: Downgrade to 2021 edition immediately

### MSRV (Minimum Supported Rust Version)

- **Current**: Not specified
- **Recommended**: 1.75+ (for 2021 edition features)
- **Action**: Add `rust-version = "1.75"` to Cargo.toml

## Breaking Changes Expected

### Leptos 0.8 → 0.11

- Signal API changes
- Component macro updates
- SSR context changes
- **Impact**: HIGH - Major refactoring needed

### Tokio-tungstenite 0.27 → 0.34

- Error type changes
- API surface updates
- **Impact**: MEDIUM - Moderate refactoring

## Implementation Plan

1. **Create feature branch**: `deps/security-update-2025`
2. **Update in phases**: P0 → P1 → P2
3. **Fix compilation**: Address breaking changes
4. **Run full test suite**: Ensure compatibility
5. **Update CI/CD**: Pin versions in CI

## Validation Checklist

- [ ] `cargo check` passes
- [ ] `cargo test` passes
- [ ] `cargo clippy` clean
- [ ] Security audit clean (`cargo deny`)
- [ ] Performance regression tests
- [ ] Integration tests pass
