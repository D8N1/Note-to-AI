# Testing Strategy for note-to-ai

## Overview

This document outlines the comprehensive testing strategy for the note-to-ai project, covering unit tests, integration tests, property-based tests, and performance benchmarks.

## Test Categories

### 1. Unit Tests
Located within each module using `#[cfg(test)]` blocks.

**Coverage:**
- ✅ `src/crypto/blake3_hasher.rs` - Complete test suite
- ✅ `src/vault/crdt.rs` - Basic CRDT tests
- ✅ `src/config/settings.rs` - Configuration serialization tests
- ⚠️ Other modules need unit test implementation

**Running:**
```bash
cargo test --lib
```

### 2. Integration Tests
Located in `tests/integration_tests.rs`.

**Coverage:**
- ✅ Vault creation and management
- ✅ Crypto integration
- ✅ Configuration loading
- ✅ Vault indexing
- ✅ Search functionality

**Running:**
```bash
cargo test --test integration_tests
```

### 3. Property-Based Tests
Located in `tests/property_tests.rs`.

**Coverage:**
- ✅ Hash consistency and uniqueness
- ✅ Input validation
- ✅ Edge cases

**Running:**
```bash
cargo test --test property_tests
```

### 4. Performance Tests
Located in `tests/performance_tests.rs`.

**Coverage:**
- ✅ Blake3 hashing performance
- ✅ Large data processing
- ✅ Memory usage

**Running:**
```bash
cargo test --test performance_tests
cargo bench
```

## Test Execution

### Quick Test Run
```bash
./scripts/test.sh
```

### Individual Test Categories
```bash
# Unit tests only
cargo test --lib

# Integration tests only
cargo test --test integration_tests

# All tests with verbose output
cargo test --verbose

# Tests with coverage (requires grcov)
CARGO_INCREMENTAL=0 RUSTFLAGS='-Cinstrument-coverage' cargo test
```

## Continuous Integration

GitHub Actions automatically runs the full test suite on:
- Push to main/develop branches
- Pull requests to main branch

**CI Pipeline:**
1. Unit tests
2. Integration tests
3. Property-based tests
4. Performance tests
5. Code formatting check
6. Clippy linting
7. Release build

## Test Coverage Goals

| Module | Unit Tests | Integration Tests | Property Tests | Performance Tests |
|--------|------------|-------------------|----------------|-------------------|
| crypto | ✅ 100% | ✅ Complete | ✅ Complete | ✅ Complete |
| vault | ⚠️ 20% | ✅ Complete | ⚠️ Partial | ⚠️ Partial |
| ai | ❌ 0% | ❌ None | ❌ None | ❌ None |
| signal | ❌ 0% | ❌ None | ❌ None | ❌ None |
| swarm | ❌ 0% | ❌ None | ❌ None | ❌ None |
| identity | ❌ 0% | ❌ None | ❌ None | ❌ None |
| audio | ❌ 0% | ❌ None | ❌ None | ❌ None |

## Adding New Tests

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_name() {
        // Test implementation
    }
}
```

### Integration Tests
```rust
#[tokio::test]
async fn test_integration_scenario() {
    // Integration test implementation
}
```

### Property Tests
```rust
proptest! {
    #[test]
    fn test_property(input: Vec<u8>) {
        // Property-based test
    }
}
```

## Test Data Management

- Use `tempfile` crate for temporary files
- Clean up resources in test teardown
- Use deterministic test data where possible
- Mock external dependencies

## Performance Benchmarks

Run benchmarks with:
```bash
cargo bench
```

Benchmark results are saved in `target/criterion/` and can be viewed in HTML format.

## Coverage Reporting

Install grcov for coverage analysis:
```bash
cargo install grcov
```

Generate coverage report:
```bash
./scripts/test.sh
```

Coverage reports are generated in `./coverage/` directory.

## Test Environment

Set environment variables for testing:
```bash
export RUST_BACKTRACE=1
export NOTE_TO_AI_TEST_MODE=true
```

## Troubleshooting

### Common Issues

1. **Tests failing due to missing dependencies**
   - Run `cargo build` first
   - Check that all dependencies are properly configured

2. **Integration tests failing**
   - Ensure temporary directories are properly cleaned up
   - Check file permissions

3. **Performance tests timing out**
   - Increase timeout limits in CI configuration
   - Check system resources

### Debug Mode
Run tests with debug output:
```bash
RUST_LOG=debug cargo test --verbose
``` 