//! MCP Proxy Core
//!
//! This crate contains the core business logic for the MCP Proxy,
//! including protocol handling, transport abstractions, and configuration.

pub mod config;
pub mod error;
pub mod protocol;
pub mod state; // Temporary - will be moved to server crate
pub mod transport;

// Re-export commonly used types
pub use config::{Config, ServerConfig, TransportConfig};
pub use error::{ProxyError, Result};
pub use protocol::{JsonRpcMessage, JsonRpcRequest, JsonRpcResponse};
pub use transport::{Transport, TransportType};

/// Core version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }
}
