#!/bin/bash

echo "Building Yew UI separately..."

# Check if trunk is installed
if ! command -v trunk &> /dev/null; then
    echo "Trunk is not installed. Please install it with:"
    echo "  cargo install --locked trunk"
    exit 1
fi

# Check if wasm target is installed
if ! rustup target list --installed | grep -q wasm32-unknown-unknown; then
    echo "Installing wasm32-unknown-unknown target..."
    rustup target add wasm32-unknown-unknown
fi

# Build Yew UI
cd yew-ui
echo "Building Yew UI..."
trunk build --release

# Copy to yew-dist
cd ..
rm -rf yew-dist
cp -r yew-ui/dist yew-dist

echo "Yew UI built successfully! Files are in yew-dist/"
echo "Now you can run: cargo build --release"