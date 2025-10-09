# Yew UI Integration

The MCP Rust Proxy now includes an integrated Yew-based web UI that can be automatically built and bundled with the server.

## Overview

The project supports two UI modes:
1. **Legacy UI** - The original HTML/JS/CSS UI in the `web-ui/` directory
2. **Yew UI** - A modern Rust/WebAssembly UI in the `yew-ui/` directory

## Building

### Option 1: Use the build script (Recommended)
```bash
./build-with-ui.sh
```

This will build the server in release mode with the Yew UI compiled and bundled.

### Option 2: Manual build with Yew UI
```bash
BUILD_YEW_UI=1 cargo build --release
```

### Option 3: Build without Yew UI (uses legacy UI)
```bash
cargo build
```

## Requirements

To build the Yew UI, you need:
- Rust (latest stable)
- Trunk: `cargo install --locked trunk`
- wasm32-unknown-unknown target: `rustup target add wasm32-unknown-unknown`

## How it Works

1. During `cargo build`, the `build.rs` script checks if `BUILD_YEW_UI=1` or if building in release mode
2. If enabled, it runs `trunk build --release` in the `yew-ui/` directory
3. The compiled files are copied to `yew-dist/` in the project root
4. At runtime, the server checks if `yew-dist/` exists and serves it; otherwise, it falls back to the legacy UI

## Development

To work on the Yew UI:
```bash
cd yew-ui
trunk serve --open
```

This will start a development server with hot reloading at http://localhost:8080

## UI Selection Logic

The server automatically selects which UI to serve:
- If `yew-dist/` directory exists → Serve Yew UI
- Otherwise → Serve legacy UI from `web-ui/`

You'll see a log message indicating which UI is being used:
```
Using compiled Yew UI from yew-dist/
```
or
```
Using legacy web UI from web-ui/
```

## Troubleshooting

1. **Trunk not found**: Install with `cargo install --locked trunk`
2. **Build fails**: Ensure you have the wasm32 target: `rustup target add wasm32-unknown-unknown`
3. **UI not updating**: Delete the `yew-dist/` directory and rebuild

## File Structure

```
mcp-rust-proxy/
├── web-ui/          # Legacy HTML/JS/CSS UI
├── yew-ui/          # Yew source code
├── yew-dist/        # Compiled Yew UI (git ignored)
├── build.rs         # Build script that compiles Yew UI
└── src/web/mod.rs   # Web server that serves the appropriate UI
```