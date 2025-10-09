#!/bin/bash

# Development launcher for MCP Proxy Tauri App

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd "$SCRIPT_DIR"

echo "========================================="
echo "MCP Proxy Tauri App - Development Mode"
echo "========================================="

# Check prerequisites
check_command() {
    if ! command -v "$1" &> /dev/null; then
        echo "❌ $1 is not installed. Please install it first."
        return 1
    else
        echo "✅ $1 is installed"
        return 0
    fi
}

echo ""
echo "Checking prerequisites..."
check_command "rustc" || exit 1
check_command "cargo" || exit 1
check_command "node" || exit 1
check_command "npm" || exit 1

# Check if Tauri CLI is installed locally
if ! npm list @tauri-apps/cli &> /dev/null; then
    echo "📦 Installing Tauri CLI locally..."
    # Install locally instead of globally
    npm install --save-dev @tauri-apps/cli@next
fi

# Check if trunk is installed for Yew
if ! command -v trunk &> /dev/null; then
    echo "📦 Installing trunk for Yew..."
    cargo install trunk --locked
fi

# Install npm dependencies
if [ ! -d "node_modules" ]; then
    echo ""
    echo "📦 Installing npm dependencies..."
    npm install
fi

# Create placeholder icons if they don't exist
ICONS_DIR="src-tauri/icons"
if [ ! -f "$ICONS_DIR/icon.png" ]; then
    echo ""
    echo "🎨 Creating placeholder icons..."
    mkdir -p "$ICONS_DIR"
    
    # Create a simple 32x32 base64 PNG placeholder
    echo "iVBORw0KGgoAAAANSUhEUgAAACAAAAAgCAYAAABzenr0AAAABHNCSVQICAgIfAhkiAAAAAlwSFlzAAAA7AAAAOwBeShxvQAAABl0RVh0U29mdHdhcmUAd3d3Lmlua3NjYXBlLm9yZ5vuPBoAAABMSURBVFiF7dYxAQAgDMCwgX/PwJMNCLqzs3d3B/xqvgMaE4CECUDCBCBhApAwAUiYACRMABImAAkTgIQJQMIEIGECkDABSJgAJEwADyYXAxyj1RkGAAAAAElFTkSuQmCC" | base64 -d > "$ICONS_DIR/32x32.png" 2>/dev/null || \
    echo "iVBORw0KGgoAAAANSUhEUgAAACAAAAAgCAYAAABzenr0AAAABHNCSVQICAgIfAhkiAAAAAlwSFlzAAAA7AAAAOwBeShxvQAAABl0RVh0U29mdHdhcmUAd3d3Lmlua3NjYXBlLm9yZ5vuPBoAAABMSURBVFiF7dYxAQAgDMCwgX/PwJMNCLqzs3d3B/xqvgMaE4CECUDCBCBhApAwAUiYACRMABImAAkTgIQJQMIEIGECkDABSJgAJEwADyYXAxyj1RkGAAAAAElFTkSuQmCC" | base64 -D > "$ICONS_DIR/32x32.png"
    
    cp "$ICONS_DIR/32x32.png" "$ICONS_DIR/128x128.png"
    cp "$ICONS_DIR/32x32.png" "$ICONS_DIR/128x128@2x.png"
    cp "$ICONS_DIR/32x32.png" "$ICONS_DIR/icon.png"
    cp "$ICONS_DIR/32x32.png" "$ICONS_DIR/icon.ico"
    cp "$ICONS_DIR/32x32.png" "$ICONS_DIR/icon.icns"
    echo "✅ Icons created"
fi

# Check if Yew UI exists and prepare it
YEW_DIR="../yew-ui"
if [ -d "$YEW_DIR" ]; then
    echo ""
    echo "🔧 Preparing Yew UI for Tauri..."
    
    # Create index.html if it doesn't exist
    if [ ! -f "$YEW_DIR/index.html" ]; then
        cat > "$YEW_DIR/index.html" << 'EOF'
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>MCP Proxy Manager</title>
    <link data-trunk rel="css" href="style.css">
    <link data-trunk rel="rust" data-wasm-opt="z" />
    <script type="module">
        // Make Tauri API available globally for Yew
        import { invoke } from "@tauri-apps/api/core";
        window.__TAURI_INVOKE__ = invoke;
    </script>
</head>
<body>
    <div id="app"></div>
</body>
</html>
EOF
        echo "✅ Created index.html for Yew UI"
    fi
    
    # Create basic style.css if it doesn't exist
    if [ ! -f "$YEW_DIR/style.css" ]; then
        cat > "$YEW_DIR/style.css" << 'EOF'
* {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
}

body {
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Oxygen, Ubuntu, sans-serif;
    background: #f5f5f5;
    color: #333;
}

#app {
    min-height: 100vh;
}
EOF
        echo "✅ Created style.css for Yew UI"
    fi
else
    echo "⚠️  Yew UI directory not found at $YEW_DIR"
    echo "   The app will start but may not have a UI"
fi

# Set environment variables for development
export RUST_LOG=debug
export RUST_BACKTRACE=1

# Start the development server
echo ""
echo "🚀 Starting Tauri development server..."
echo "   Proxy API: http://localhost:3001"
echo "   Dev UI: http://localhost:1420"
echo ""
echo "Press Ctrl+C to stop"
echo "========================================="
echo ""

# Run Tauri in development mode using npx
npx tauri dev