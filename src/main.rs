use tokio::signal;
use tracing::{info, error};

mod config;
mod error;
mod transport;
mod state;
mod server;
mod proxy;
mod web;

use error::Result;
use state::AppState;
use server::ServerManager;
use proxy::ProxyServer;

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

    // Initialize application state
    let (state, shutdown_rx) = AppState::new(config);

    // Start server manager
    let server_manager = ServerManager::new(state.clone(), shutdown_rx.resubscribe());
    let manager_handle = tokio::spawn(async move {
        if let Err(e) = server_manager.run().await {
            error!("Server manager error: {}", e);
        }
    });

    // Start proxy server
    let proxy_server = ProxyServer::new(state.clone());
    let proxy_handle = tokio::spawn(async move {
        if let Err(e) = proxy_server.run().await {
            error!("Proxy server error: {}", e);
        }
    });

    // Start web UI if enabled
    let web_handle = if state.config.read().await.web_ui.enabled {
        let web_state = state.clone();
        Some(tokio::spawn(async move {
            if let Err(e) = web::start_server(web_state).await {
                error!("Web UI server error: {}", e);
            }
        }))
    } else {
        None
    };

    // Wait for shutdown signal
    shutdown_signal().await;

    info!("Shutting down MCP Rust Proxy Server");

    // Graceful shutdown
    state.shutdown().await;

    // Wait for tasks to complete
    if let Some(web_handle) = web_handle {
        let _ = tokio::join!(manager_handle, proxy_handle, web_handle);
    } else {
        let _ = tokio::join!(manager_handle, proxy_handle);
    }

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