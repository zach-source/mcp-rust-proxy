use clap::Parser;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::signal;
use tracing::{error, info};

use mcp_rust_proxy::commands;
use mcp_rust_proxy::config;
use mcp_rust_proxy::error::Result;
use mcp_rust_proxy::proxy::ProxyServer;
use mcp_rust_proxy::server::ServerManager;
use mcp_rust_proxy::state::AppState;
use mcp_rust_proxy::web;

#[derive(Parser, Debug)]
#[command(name = "mcp-rust-proxy")]
#[command(about = "A fast and efficient Model Context Protocol (MCP) proxy server", long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,

    /// Path to configuration file (YAML/JSON/TOML)
    #[arg(short, long, value_name = "FILE", global = true)]
    config: Option<PathBuf>,

    /// Enable debug logging
    #[arg(short, long, global = true)]
    debug: bool,

    /// Run in stdio mode (for use as MCP server with Claude CLI)
    #[arg(long, global = true)]
    stdio: bool,
}

#[derive(Debug, clap::Subcommand)]
enum Command {
    /// Run the proxy server (default)
    Run,
    /// Check configuration and test server connections
    Check {
        /// Test ping support for all servers
        #[arg(long)]
        ping: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command-line arguments
    let args = Args::parse();

    // Initialize tracing
    let log_level = if args.debug { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(format!("mcp_rust_proxy={log_level}").parse().unwrap()),
        )
        .init();

    // Load configuration first
    let config = match args.config {
        Some(path) => {
            info!("Loading configuration from: {}", path.display());
            match config::load_from_path(&path).await {
                Ok(cfg) => {
                    info!("Configuration loaded successfully from {}", path.display());
                    cfg
                }
                Err(e) => {
                    error!(
                        "Failed to load configuration from {}: {}",
                        path.display(),
                        e
                    );
                    return Err(e);
                }
            }
        }
        None => {
            info!("Loading configuration from default locations");
            match config::load_from_env_or_file().await {
                Ok(cfg) => {
                    info!("Configuration loaded successfully");
                    cfg
                }
                Err(e) => {
                    error!("Failed to load configuration: {}", e);
                    return Err(e);
                }
            }
        }
    };

    // Handle commands
    match args.command.unwrap_or(Command::Run) {
        Command::Check { ping } => {
            // Run config check
            return commands::run_config_check(config, ping).await;
        }
        Command::Run => {
            // Check if stdio mode is enabled
            if args.stdio {
                info!("Starting MCP Rust Proxy in stdio mode");
                info!("Loaded {} server configurations", config.servers.len());
                // In stdio mode, run the stdio server instead of HTTP
                return run_stdio_mode(config).await;
            } else {
                // Continue with normal HTTP server startup
                info!("Starting MCP Rust Proxy Server");
                info!("Loaded {} server configurations", config.servers.len());
                info!(
                    "Proxy will listen on {}:{}",
                    config.proxy.host, config.proxy.port
                );
                if config.web_ui.enabled {
                    info!(
                        "Web UI will be available on {}:{}",
                        config.web_ui.host, config.web_ui.port
                    );
                }
            }
        }
    }

    // Initialize application state
    let (state, shutdown_rx) = AppState::new(config.clone());

    // Discover and load plugins if configured
    if let Some(plugin_manager) = &state.plugin_manager {
        info!(
            "Discovering plugins from: {}",
            config.plugins.as_ref().unwrap().plugin_dir.display()
        );
        match plugin_manager.discover_plugins().await {
            Ok(count) => {
                info!("Discovered {} plugins successfully", count);
            }
            Err(e) => {
                error!("Failed to discover plugins: {}", e);
            }
        }
    }

    // Initialize context tracker if enabled
    if config.context_tracing.enabled {
        info!("Initializing context tracing framework");
        match mcp_rust_proxy::context::storage::HybridStorage::new(
            config.context_tracing.sqlite_path.clone(),
            Some(mcp_rust_proxy::context::storage::CacheConfig {
                max_entries: config.context_tracing.cache_size,
                ttl_seconds: config.context_tracing.cache_ttl_seconds,
                eviction_strategy: mcp_rust_proxy::context::storage::EvictionStrategy::TimeBasedLRU,
            }),
        )
        .await
        {
            Ok(storage) => {
                let storage: Arc<dyn mcp_rust_proxy::context::storage::StorageBackend> =
                    Arc::new(storage);
                if let Err(e) = state.initialize_context_tracker(storage.clone()).await {
                    error!("Failed to initialize context tracker: {}", e);
                } else {
                    info!("Context tracing initialized successfully");

                    // Start background retention job
                    let retention_days = config.context_tracing.retention_days;
                    let storage_for_cleanup = storage.clone();
                    tokio::spawn(async move {
                        let mut interval =
                            tokio::time::interval(tokio::time::Duration::from_secs(86400)); // Daily
                        loop {
                            interval.tick().await;
                            info!(
                                "Running context tracing retention policy ({}d)",
                                retention_days
                            );
                            match storage_for_cleanup.cleanup_old_data(retention_days).await {
                                Ok(deleted) => {
                                    info!("Retention policy deleted {} old records", deleted);
                                }
                                Err(e) => {
                                    error!("Retention policy failed: {}", e);
                                }
                            }
                        }
                    });
                    info!("Background retention job started (runs daily)");
                }
            }
            Err(e) => {
                error!("Failed to create context storage: {}", e);
            }
        }
    }

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

    // Start cache warmer for instant tool/resource availability
    let cache_warmer_state = state.clone();
    let cache_warmer_handler = Arc::new(mcp_rust_proxy::proxy::RequestHandler::new(state.clone()));
    let cache_warmer_handle = tokio::spawn(async move {
        let warmer = mcp_rust_proxy::proxy::cache_warmer::CacheWarmer::new(
            cache_warmer_state,
            cache_warmer_handler,
            60, // Refresh every 60 seconds
        );
        warmer.run().await;
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

    // Graceful shutdown with timeout
    let shutdown_timeout = tokio::time::timeout(tokio::time::Duration::from_secs(30), async {
        // Signal shutdown to all components
        state.shutdown().await;

        // Wait for tasks to complete
        if let Some(web_handle) = web_handle {
            let _ = tokio::join!(manager_handle, proxy_handle, web_handle);
        } else {
            let _ = tokio::join!(manager_handle, proxy_handle);
        }
    })
    .await;

    match shutdown_timeout {
        Ok(_) => {
            info!("Graceful shutdown completed");
        }
        Err(_) => {
            error!("Shutdown timeout exceeded, forcing exit");
            // Force exit after timeout
            std::process::exit(1);
        }
    }

    Ok(())
}

async fn run_stdio_mode(config: mcp_rust_proxy::config::Config) -> Result<()> {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    // Initialize application state
    let (state, _shutdown_rx) = AppState::new(config.clone());

    // Discover and load plugins if configured
    if let Some(plugin_manager) = &state.plugin_manager {
        info!(
            "Discovering plugins from: {}",
            config.plugins.as_ref().unwrap().plugin_dir.display()
        );
        match plugin_manager.discover_plugins().await {
            Ok(count) => {
                info!("Discovered {} plugins successfully", count);
            }
            Err(e) => {
                error!("Failed to discover plugins: {}", e);
            }
        }
    }

    // Initialize context tracker if enabled
    if config.context_tracing.enabled {
        info!("Initializing context tracing framework");
        match mcp_rust_proxy::context::storage::HybridStorage::new(
            config.context_tracing.sqlite_path.clone(),
            Some(mcp_rust_proxy::context::storage::CacheConfig {
                max_entries: config.context_tracing.cache_size,
                ttl_seconds: config.context_tracing.cache_ttl_seconds,
                eviction_strategy: mcp_rust_proxy::context::storage::EvictionStrategy::TimeBasedLRU,
            }),
        )
        .await
        {
            Ok(storage) => {
                let storage: Arc<dyn mcp_rust_proxy::context::storage::StorageBackend> =
                    Arc::new(storage);
                if let Err(e) = state.initialize_context_tracker(storage.clone()).await {
                    error!("Failed to initialize context tracker: {}", e);
                } else {
                    info!("Context tracing initialized successfully");

                    // Start background retention job
                    let retention_days = config.context_tracing.retention_days;
                    let storage_for_cleanup = storage.clone();
                    tokio::spawn(async move {
                        let mut interval =
                            tokio::time::interval(tokio::time::Duration::from_secs(86400)); // Daily
                        loop {
                            interval.tick().await;
                            info!(
                                "Running context tracing retention policy ({}d)",
                                retention_days
                            );
                            match storage_for_cleanup.cleanup_old_data(retention_days).await {
                                Ok(deleted) => {
                                    info!("Retention policy deleted {} old records", deleted);
                                }
                                Err(e) => {
                                    error!("Retention policy failed: {}", e);
                                }
                            }
                        }
                    });
                    info!("Background retention job started (runs daily)");
                }
            }
            Err(e) => {
                error!("Failed to create context storage: {}", e);
            }
        }
    }

    // Start server manager in background
    let server_manager = ServerManager::new(state.clone(), state.shutdown_tx.subscribe());
    tokio::spawn(async move {
        if let Err(e) = server_manager.run().await {
            error!("Server manager error: {}", e);
        }
    });

    // Give servers time to start
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Create proxy components
    let router = std::sync::Arc::new(mcp_rust_proxy::proxy::RequestRouter::new());
    let handler = std::sync::Arc::new(mcp_rust_proxy::proxy::RequestHandler::new(state.clone()));

    info!("Stdio mode ready - reading from stdin, writing to stdout");

    // Read from stdin, write to stdout
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let mut reader = BufReader::new(stdin);
    let mut writer = stdout;
    let mut line = String::new();

    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => {
                // EOF - client disconnected
                info!("Stdin closed, shutting down");
                break;
            }
            Ok(_) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                // Parse JSON-RPC request
                match serde_json::from_str::<serde_json::Value>(trimmed) {
                    Ok(request) => {
                        // Handle the request
                        match handler.handle_request(request, router.clone()).await {
                            Ok(response) => {
                                // Write response to stdout
                                let response_json = serde_json::to_string(&response).unwrap();
                                if let Err(e) = writer.write_all(response_json.as_bytes()).await {
                                    error!("Failed to write response: {}", e);
                                    break;
                                }
                                if let Err(e) = writer.write_all(b"\n").await {
                                    error!("Failed to write newline: {}", e);
                                    break;
                                }
                                if let Err(e) = writer.flush().await {
                                    error!("Failed to flush: {}", e);
                                    break;
                                }
                            }
                            Err(e) => {
                                error!("Error handling request: {}", e);
                                // Send error response
                                let error_response = mcp_rust_proxy::proxy::MCPResponse {
                                    jsonrpc: "2.0".to_string(),
                                    id: None,
                                    result: None,
                                    error: Some(mcp_rust_proxy::proxy::MCPError {
                                        code: -32603,
                                        message: e.to_string(),
                                        data: None,
                                    }),
                                };
                                let response_json = serde_json::to_string(&error_response).unwrap();
                                let _ = writer.write_all(response_json.as_bytes()).await;
                                let _ = writer.write_all(b"\n").await;
                                let _ = writer.flush().await;
                            }
                        }
                    }
                    Err(e) => {
                        error!("Invalid JSON: {}", e);
                        // Send parse error response
                        let error_response = mcp_rust_proxy::proxy::MCPResponse {
                            jsonrpc: "2.0".to_string(),
                            id: None,
                            result: None,
                            error: Some(mcp_rust_proxy::proxy::MCPError {
                                code: -32700,
                                message: format!("Parse error: {e}"),
                                data: None,
                            }),
                        };
                        let response_json = serde_json::to_string(&error_response).unwrap();
                        let _ = writer.write_all(response_json.as_bytes()).await;
                        let _ = writer.write_all(b"\n").await;
                        let _ = writer.flush().await;
                    }
                }
            }
            Err(e) => {
                error!("Error reading from stdin: {}", e);
                break;
            }
        }
    }

    info!("Stdio mode exiting");
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
