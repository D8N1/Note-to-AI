#!/bin/bash

echo "Running note-to-ai test suite..."
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR" || exit 1

# Set test environment
export RUST_BACKTRACE=1
export NOTE_TO_AI_TEST_MODE=true


# Path to the correct Cargo.toml
MANIFEST_PATH="$SCRIPT_DIR/../Cargo.toml"

# Run unit tests
echo "Running unit tests..."
cargo test --lib --manifest-path "$MANIFEST_PATH"

# Run integration tests
echo "Running integration tests..."
cargo test --test integration_tests --manifest-path "$MANIFEST_PATH"

# Run property-based tests
echo "Running property-based tests..."
cargo test --test property_tests --manifest-path "$MANIFEST_PATH"

# Run performance tests
echo "Running performance tests..."
cargo test --test performance_tests --manifest-path "$MANIFEST_PATH"

# Run benchmarks
echo "Running benchmarks..."
cargo bench

# Run with coverage (if grcov is installed)
if command -v grcov &> /dev/null; then
    echo "Running coverage analysis..."
    CARGO_INCREMENTAL=0 RUSTFLAGS='-Cinstrument-coverage' LLVM_PROFILE_FILE='cargo-test-%p-%m.profraw' cargo test
    grcov . --binary-path ./target/debug/ -s . -t html --branch --ignore-not-existing -o ./coverage/
    echo "Coverage report generated in ./coverage/"
fi

echo "Test suite completed!" 