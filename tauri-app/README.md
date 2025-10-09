# MCP Proxy Tauri Desktop Application

A native desktop application for MCP Proxy built with Tauri v2, providing a lightweight, secure, and performant alternative to the web-based UI.

## Features

- **Native Desktop App**: Cross-platform support for Windows, macOS, and Linux
- **System Tray Integration**: Quick access and background operation
- **Native Notifications**: Real-time alerts for server status changes
- **Embedded Proxy Server**: Runs the MCP proxy server directly within the app
- **Smaller Size**: ~10MB compared to Electron alternatives
- **Better Performance**: Uses native system webview
- **Secure**: Built with Rust's memory safety guarantees

## Architecture

The Tauri app consists of:

1. **Frontend**: Reuses the existing Yew-based UI with minimal modifications
2. **Backend**: Rust-based Tauri backend that embeds the MCP proxy server
3. **IPC Bridge**: Commands and events for communication between frontend and backend

### Directory Structure

```
tauri-app/
├── src-tauri/          # Tauri backend (Rust)
│   ├── src/
│   │   ├── main.rs     # Tauri app entry point
│   │   ├── commands.rs # IPC command handlers
│   │   ├── proxy.rs    # Embedded proxy server
│   │   └── state.rs    # Application state management
│   ├── icons/          # Application icons
│   └── Cargo.toml      # Rust dependencies
├── src/                # Frontend integration
│   ├── tauri_api.rs    # Tauri API bindings
│   └── api_adapter.rs  # API adapter for HTTP/Tauri modes
├── package.json        # Node dependencies
└── build.sh           # Build script
```

## Development

### Prerequisites

1. Install Rust and Cargo
2. Install Node.js and npm
3. Install Tauri CLI: `npm install -g @tauri-apps/cli`

### Building

```bash
# Install dependencies
cd tauri-app
npm install

# Development mode
npm run dev

# Production build
npm run build

# Or use the build script
./build.sh
```

### Testing

The app can be tested in development mode with hot-reload:

```bash
npm run dev
```

## API Integration

The app provides the same API as the web version but through Tauri's IPC system:

### Commands (Frontend → Backend)
- `get_servers()` - List all MCP servers
- `server_action(name, action)` - Control servers (start/stop/restart)
- `get_metrics()` - Fetch server metrics
- `get_logs(server, lines, type)` - Retrieve server logs
- `get_config()` - Get configuration
- `update_config(config)` - Update configuration

### Events (Backend → Frontend)
- `server:status` - Server status updates
- `logs:{server}` - Real-time log streaming
- `health:check` - Health check results
- `notification` - System notifications

## Platform-Specific Features

### macOS
- Menu bar integration
- Native notifications via Notification Center
- App signing for distribution

### Windows
- System tray with context menu
- Jump list for quick actions
- MSI installer

### Linux
- System tray support (varies by DE)
- Desktop notifications via libnotify
- AppImage, .deb, and .rpm packages

## Distribution

Built applications are output to `src-tauri/target/release/bundle/`:

- **macOS**: `.app` bundle and `.dmg` installer
- **Windows**: `.exe` and `.msi` installer
- **Linux**: `.AppImage`, `.deb`, and `.rpm` packages

## Configuration

The Tauri configuration is in `src-tauri/tauri.conf.json`:

```json
{
  "productName": "MCP Proxy",
  "version": "0.1.0",
  "identifier": "com.mcp-proxy.app",
  ...
}
```

## Security

- All IPC commands are validated and sanitized
- File system access is restricted to necessary directories
- Network requests are limited to configured MCP servers
- Content Security Policy prevents XSS attacks

## Migrating from Web UI

The Tauri app maintains full compatibility with the existing web UI configuration and data:

1. Configuration files are stored in the same location
2. Log files remain in `~/.mcp-proxy/logs/`
3. Server definitions are unchanged
4. All features from the web UI are available

## Troubleshooting

### Build Issues
- Ensure all prerequisites are installed
- Run `cargo clean` and rebuild
- Check that icon files exist in `src-tauri/icons/`

### Runtime Issues
- Check the developer console for errors (DevTools in dev mode)
- Verify the proxy server is starting correctly
- Check system logs for Tauri-specific errors

## Future Enhancements

- [ ] Auto-updater integration
- [ ] Global hotkeys for quick actions
- [ ] Enhanced system tray menu with server status
- [ ] Native file associations for config files
- [ ] Deep OS integration (Windows taskbar, macOS dock badges)
- [ ] Offline mode with local caching