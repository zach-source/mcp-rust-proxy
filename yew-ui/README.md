# MCP Proxy Yew UI

A Yew-based web UI for the MCP Proxy Server Dashboard.

## Prerequisites

- Rust and Cargo
- Trunk (install with `cargo install trunk`)
- wasm32-unknown-unknown target (install with `rustup target add wasm32-unknown-unknown`)

## Development

1. Start the MCP proxy server on port 3000

2. In the yew-ui directory, run:
   ```bash
   trunk serve --open
   ```

This will:
- Build the Yew application
- Serve it on http://localhost:8080
- Proxy API requests to http://localhost:3000
- Open the UI in your browser
- Watch for changes and hot-reload

## Production Build

```bash
trunk build --release
```

The built files will be in the `dist/` directory.

## Features

- Real-time server status updates via WebSocket
- Server management (start/stop/restart)
- Live log streaming
- System metrics display
- Responsive design
- Auto-reconnecting WebSocket

## Architecture

- **Yew Components**: Modular UI components
- **WebSocket Service**: Handles real-time updates
- **API Module**: REST API calls for server actions
- **Type Definitions**: Shared types matching the server API