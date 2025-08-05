# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.0.1] - 2025-01-05

### Added
- Initial release of MCP Rust Proxy
- Multi-server proxy support for aggregating multiple MCP servers
- Support for multiple transport types:
  - stdio (standard input/output)
  - HTTP/SSE (Server-Sent Events)
  - WebSocket
- File-based logging system with rotation and retention
- Real-time log streaming via Server-Sent Events
- Web UI dashboard for server management and monitoring
- Health monitoring with configurable checks and auto-restart
- Connection pooling for efficient resource management
- Configuration via YAML/JSON/TOML with environment variable substitution
- Prometheus-compatible metrics collection
- Server lifecycle management (start, stop, restart)
- Integration with Claude Code via MCP remote server
- Cross-platform support (Linux, macOS, x86_64, ARM64)

### Features
- **Performance**: Built with Rust for high performance and low resource usage
- **Reliability**: Automatic reconnection and health monitoring
- **Observability**: Comprehensive logging and metrics
- **Developer Experience**: Easy configuration and web-based management UI
- **Extensibility**: Modular architecture for adding new transports and features

### Technical Details
- Built with Tokio async runtime
- Uses DashMap for thread-safe concurrent state management
- Yew framework for WASM-based web UI
- Supports graceful shutdown of all servers and connections

[0.0.1]: https://github.com/zach-source/mcp-rust-proxy/releases/tag/v0.0.1