#!/bin/bash

# Install dependencies if needed
if ! command -v trunk &> /dev/null; then
    echo "Installing trunk..."
    cargo install trunk
fi

# Add wasm target if needed
rustup target add wasm32-unknown-unknown

# Build and serve
echo "Building and serving Yew UI..."
trunk serve --open