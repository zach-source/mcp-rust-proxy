#!/bin/bash

# Build script for Tauri app

echo "Building MCP Proxy Tauri App..."

# Create placeholder icons if they don't exist
ICONS_DIR="src-tauri/icons"
if [ ! -f "$ICONS_DIR/icon.png" ]; then
    echo "Creating placeholder icons..."
    # Create a simple 32x32 PNG placeholder
    echo "iVBORw0KGgoAAAANSUhEUgAAACAAAAAgCAYAAABzenr0AAAABHNCSVQICAgIfAhkiAAAAAlwSFlzAAAA7AAAAOwBeShxvQAAABl0RVh0U29mdHdhcmUAd3d3Lmlua3NjYXBlLm9yZ5vuPBoAAABMSURBVFiF7dYxAQAgDMCwgX/PwJMNCLqzs3d3B/xqvgMaE4CECUDCBCBhApAwAUiYACRMABImAAkTgIQJQMIEIGECkDABSJgAJEwADyYXAxyj1RkGAAAAAElFTkSuQmCC" | base64 -d > "$ICONS_DIR/32x32.png"
    cp "$ICONS_DIR/32x32.png" "$ICONS_DIR/128x128.png"
    cp "$ICONS_DIR/32x32.png" "$ICONS_DIR/128x128@2x.png"
    cp "$ICONS_DIR/32x32.png" "$ICONS_DIR/icon.png"
    cp "$ICONS_DIR/32x32.png" "$ICONS_DIR/icon.ico"
    cp "$ICONS_DIR/32x32.png" "$ICONS_DIR/icon.icns"
fi

# Update Yew UI index.html for Tauri compatibility
YEW_INDEX="../yew-ui/index.html"
if [ -f "$YEW_INDEX" ]; then
    echo "Updating Yew UI index.html for Tauri..."
    # Add Tauri API script if not already present
    if ! grep -q "__TAURI__" "$YEW_INDEX"; then
        sed -i '' '/<\/head>/i\
    <script type="module">\
      import { invoke } from "@tauri-apps/api/core";\
      window.__TAURI_INVOKE__ = invoke;\
    </script>' "$YEW_INDEX"
    fi
fi

# Install npm dependencies
echo "Installing npm dependencies..."
npm install

# Build the Tauri app
echo "Building Tauri app..."
npm run build

echo "Build complete!"