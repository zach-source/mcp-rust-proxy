//! MCP Proxy Server
//!
//! This crate contains the server implementation for the MCP Proxy,
//! including the proxy handler, server lifecycle management, and web API.

pub mod logging;
pub mod proxy;
pub mod server;
pub mod state;
pub mod web;

use anyhow::Result;
use mcp_proxy_core::Config;
use std::sync::Arc;

// Re-export commonly used types
pub use state::AppState;

/// Server version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct ProxyState {
    pub config: Config,
    pub shared_state: Arc<AppState>,
}

impl ProxyState {
    pub async fn new(config: Config, _proxy_port: u16, _api_port: u16) -> Result<Self> {
        let (shared_state, _shutdown_rx) = AppState::new(config.clone());
        Ok(Self {
            config,
            shared_state,
        })
    }
}

pub async fn start_proxy_server(state: ProxyState, proxy_port: u16, api_port: u16) -> Result<()> {
    // Initialize servers from config
    for (name, server_config) in state.config.servers.iter() {
        // TODO: Initialize server state
        tracing::info!("Initializing server: {}", name);

        // Start servers (check disabled status from state)
        {
            tracing::info!("Starting server: {}", name);
        }
    }

    // Start web API server
    let api_state = state.shared_state.clone();
    let api_handle = tokio::spawn(async move {
        // TODO: Implement API server
        tracing::info!("Starting API server on port {}", api_port);
        Ok::<(), anyhow::Error>(())
    });

    // Start proxy server
    let proxy_state = state.shared_state.clone();
    let proxy_handle = tokio::spawn(async move {
        // TODO: Implement proxy server
        tracing::info!("Starting proxy server on port {}", proxy_port);
        Ok::<(), anyhow::Error>(())
    });

    // Wait for both servers
    let (api_result, proxy_result) = tokio::join!(api_handle, proxy_handle);
    api_result??;
    proxy_result??;

    Ok(())
}
