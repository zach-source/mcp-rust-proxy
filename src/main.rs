use tokio::signal;
use tracing::{info, error};

mod config;
mod error;
mod transport;

use error::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("mcp_rust_proxy=debug".parse().unwrap())
        )
        .init();

    info!("Starting MCP Rust Proxy Server");

    // Load configuration
    let config = match config::load_from_env_or_file().await {
        Ok(cfg) => {
            info!("Configuration loaded successfully");
            cfg
        }
        Err(e) => {
            error!("Failed to load configuration: {}", e);
            return Err(e);
        }
    };

    info!("Loaded {} server configurations", config.servers.len());
    info!("Proxy will listen on {}:{}", config.proxy.host, config.proxy.port);
    if config.web_ui.enabled {
        info!("Web UI will be available on {}:{}", config.web_ui.host, config.web_ui.port);
    }

    // TODO: Initialize application state
    // TODO: Start server manager
    // TODO: Start proxy server
    // TODO: Start web UI if enabled

    // Wait for shutdown signal
    shutdown_signal().await;

    info!("Shutting down MCP Rust Proxy Server");

    // TODO: Graceful shutdown

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C signal");
        },
        _ = terminate => {
            info!("Received terminate signal");
        },
    }
}