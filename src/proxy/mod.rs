use crate::error::{ProxyError, Result};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use warp::Filter;

pub mod cache_warmer;
pub mod handler;
pub mod prompts;
pub mod resources;
pub mod router;
pub mod server_tools;
pub mod tracing_tools;

pub use handler::RequestHandler;
pub use router::RequestRouter;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "method")]
pub enum MCPRequest {
    #[serde(rename = "list")]
    List { params: ListParams },

    #[serde(rename = "call")]
    Call { params: CallParams },

    #[serde(rename = "read")]
    Read { params: ReadParams },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ListParams {
    #[serde(rename = "tools")]
    Tools,

    #[serde(rename = "resources")]
    Resources,

    #[serde(rename = "prompts")]
    Prompts,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallParams {
    pub tool: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadParams {
    pub uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPResponse {
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<MCPError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

pub struct ProxyServer {
    state: Arc<AppState>,
    router: Arc<RequestRouter>,
    handler: Arc<RequestHandler>,
}

impl ProxyServer {
    pub fn new(state: Arc<AppState>) -> Self {
        let router = Arc::new(RequestRouter::new());
        let handler = Arc::new(RequestHandler::new(state.clone()));

        Self {
            state,
            router,
            handler,
        }
    }

    pub async fn run(self) -> Result<()> {
        tracing::info!("Starting proxy server");

        // Build initial routing maps
        self.build_routing_maps().await?;

        // Start the server based on configuration
        let config = self.state.config.read().await;
        let addr = format!("{}:{}", config.proxy.host, config.proxy.port);
        drop(config);

        tracing::info!("Proxy server listening on {}", addr);

        // Create warp routes
        let routes = self.create_routes();

        // Start server
        let (_addr, server) = warp::serve(routes).bind_with_graceful_shutdown(
            addr.parse::<std::net::SocketAddr>()
                .map_err(|e| ProxyError::Config(crate::error::ConfigError::Parse(e.to_string())))?,
            async move {
                let _ = self.state.shutdown_tx.subscribe().recv().await;
            },
        );

        server.await;

        tracing::info!("Proxy server stopped");
        Ok(())
    }

    async fn build_routing_maps(&self) -> Result<()> {
        // This will be populated when servers connect and report their capabilities
        // For now, we'll just log
        tracing::debug!("Building routing maps for proxy server");
        Ok(())
    }

    fn create_routes(
        &self,
    ) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        let state = self.state.clone();
        let router = self.router.clone();
        let handler = self.handler.clone();

        // JSON-RPC endpoint
        let rpc = warp::path::end()
            .and(warp::post())
            .and(warp::body::json())
            .and(warp::any().map(move || state.clone()))
            .and(warp::any().map(move || router.clone()))
            .and(warp::any().map(move || handler.clone()))
            .and_then(handle_rpc_request);

        // Health check endpoint
        let health = warp::path("health").and(warp::get()).map(|| {
            warp::reply::json(&serde_json::json!({
                "status": "healthy",
                "service": "mcp-proxy"
            }))
        });

        rpc.or(health)
    }
}

async fn handle_rpc_request(
    request: serde_json::Value,
    state: Arc<AppState>,
    router: Arc<RequestRouter>,
    handler: Arc<RequestHandler>,
) -> std::result::Result<impl warp::Reply, warp::Rejection> {
    tracing::debug!("Received RPC request: {:?}", request);
    let timer = crate::state::metrics::RequestTimer::new(state.metrics.clone());

    let response = match handler.handle_request(request, router).await {
        Ok(resp) => {
            timer.finish();
            resp
        }
        Err(e) => {
            timer.fail();
            MCPResponse {
                jsonrpc: "2.0".to_string(),
                id: None,
                result: None,
                error: Some(MCPError {
                    code: -32603,
                    message: e.to_string(),
                    data: None,
                }),
            }
        }
    };

    Ok(warp::reply::json(&response))
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

// #[cfg(test)]
// mod cache_tests; // TODO: Add test module
