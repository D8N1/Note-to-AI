#!/bin/bash

echo "Installing note-to-ai..."

# Install Rust if not present
if ! command -v cargo &> /dev/null; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    source ~/.cargo/env
fi

# Build the project
cargo build --release

# Create necessary directories
mkdir -p config models db logs

echo "Installation complete!"