#!/bin/bash

echo "========================================="
echo "Testing Tauri App Setup"
echo "========================================="

cd "$(dirname "$0")"

# Check if src-tauri directory exists
if [ -d "src-tauri" ]; then
    echo "✅ src-tauri directory exists"
else
    echo "❌ src-tauri directory not found"
    exit 1
fi

# Check if main.rs exists
if [ -f "src-tauri/src/main.rs" ]; then
    echo "✅ main.rs found"
else
    echo "❌ main.rs not found"
    exit 1
fi

# Check if tauri.conf.json exists
if [ -f "src-tauri/tauri.conf.json" ]; then
    echo "✅ tauri.conf.json found"
else
    echo "❌ tauri.conf.json not found"
    exit 1
fi

# Check if package.json exists
if [ -f "package.json" ]; then
    echo "✅ package.json found"
else
    echo "❌ package.json not found"
    exit 1
fi

# Check if node_modules exists
if [ -d "node_modules" ]; then
    echo "✅ node_modules directory exists"
else
    echo "⚠️  node_modules not found - run npm install"
fi

# Check cargo build
echo ""
echo "Testing Rust compilation..."
cd src-tauri
if cargo check --quiet; then
    echo "✅ Rust code compiles successfully"
else
    echo "❌ Rust compilation failed"
    exit 1
fi

echo ""
echo "========================================="
echo "✅ All checks passed!"
echo "========================================="
echo ""
echo "Next steps:"
echo "1. Ensure trunk is installed: cargo install trunk --locked"
echo "2. Run the development server: npx tauri dev"
echo "3. Or use the dev.sh script: ./dev.sh"