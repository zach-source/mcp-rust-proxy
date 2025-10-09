use anyhow::Result;
use clap::Parser;
use mcp_proxy_core::Config;
use mcp_proxy_server::{start_proxy_server, ProxyState};
use std::path::PathBuf;
use tracing_subscriber;

#[derive(Parser, Debug)]
#[clap(
    name = "mcp-proxy-server",
    version,
    about = "MCP Proxy Server - Backend service for MCP Proxy"
)]
struct Args {
    /// Path to configuration file
    #[clap(short, long, value_name = "FILE")]
    config: PathBuf,

    /// Proxy server port
    #[clap(short, long, default_value = "3000")]
    port: u16,

    /// API server port  
    #[clap(short = 'a', long, default_value = "3001")]
    api_port: u16,

    /// Enable debug logging
    #[clap(short = 'd', long)]
    debug: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    let log_level = if args.debug {
        tracing::Level::DEBUG
    } else {
        tracing::Level::INFO
    };

    tracing_subscriber::fmt().with_max_level(log_level).init();

    tracing::info!(
        "Starting MCP Proxy Server on ports {} (proxy) and {} (API)",
        args.port,
        args.api_port
    );

    // Load configuration
    let config_str = tokio::fs::read_to_string(&args.config).await?;
    let config: Config = if args.config.extension().unwrap_or_default() == "json" {
        serde_json::from_str(&config_str)?
    } else if args.config.extension().unwrap_or_default() == "toml" {
        toml::from_str(&config_str)?
    } else {
        serde_yaml::from_str(&config_str)?
    };

    // Initialize proxy state
    let state = ProxyState::new(config, args.port, args.api_port).await?;

    // Start the proxy server
    start_proxy_server(state, args.port, args.api_port).await?;

    Ok(())
}
