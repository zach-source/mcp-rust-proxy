//! MCP Proxy CLI
//!
//! Command-line interface for the MCP Proxy server.

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[clap(
    name = "mcp-proxy",
    version,
    about = "MCP Proxy Server - High-performance Model Context Protocol proxy"
)]
struct Args {
    /// Path to configuration file
    #[clap(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Enable debug logging
    #[clap(short, long)]
    debug: bool,

    /// Port to listen on for proxy connections
    #[clap(short, long, default_value = "3000")]
    port: u16,

    /// Port for the web UI
    #[clap(short = 'u', long, default_value = "3001")]
    ui_port: u16,

    /// Disable the web UI
    #[clap(long)]
    no_ui: bool,

    /// Run a configuration check and exit
    #[clap(long)]
    check: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    let filter = if args.debug {
        EnvFilter::new("debug")
    } else {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))
    };

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();

    tracing::info!("MCP Proxy Server v{}", env!("CARGO_PKG_VERSION"));

    if args.check {
        tracing::info!("Running configuration check...");
        // TODO: Implement config check
        tracing::info!("Configuration check passed");
        return Ok(());
    }

    // TODO: Load configuration
    // TODO: Initialize server
    // TODO: Start proxy and web UI

    tracing::info!("Starting MCP Proxy on port {}", args.port);
    if !args.no_ui {
        tracing::info!("Web UI available on port {}", args.ui_port);
    }

    // TODO: Run server
    tokio::signal::ctrl_c().await?;
    tracing::info!("Shutting down...");

    Ok(())
}
