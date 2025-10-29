//! Claude API Proxy module
//!
//! This module implements a transparent HTTPS proxy that intercepts Claude API traffic,
//! captures complete request/response data with context source attribution, and provides
//! query/feedback mechanisms for improving Claude Code context composition.
//!
//! The proxy passes authentication unchanged and maintains <100ms latency while preserving
//! security through proper TLS handling.

pub mod attribution;
pub mod capture;
pub mod config;
pub mod proxy_server;
pub mod tls_handler;

pub use config::ClaudeProxyConfig;
pub use proxy_server::ProxyServer;
pub use tls_handler::TlsHandler;
