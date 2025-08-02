use std::sync::Arc;
use warp::Filter;
use crate::error::Result;
use crate::state::AppState;

pub mod api;
pub mod ws;

pub async fn start_server(state: Arc<AppState>) -> Result<()> {
    let config = state.config.read().await;
    let addr = format!("{}:{}", config.web_ui.host, config.web_ui.port);
    let api_key = config.web_ui.api_key.clone();
    drop(config);
    
    tracing::info!("Starting web UI server on {}", addr);
    
    // Create routes
    let routes = create_routes(state.clone(), api_key);
    
    // Parse address
    let addr: std::net::SocketAddr = addr.parse()
        .map_err(|e| crate::error::ProxyError::Config(
            crate::error::ConfigError::Parse(format!("Invalid web UI address: {}", e))
        ))?;
    
    // Start server with graceful shutdown
    let (_, server) = warp::serve(routes)
        .bind_with_graceful_shutdown(addr, async move {
            let _ = state.shutdown_tx.subscribe().recv().await;
        });
    
    server.await;
    
    tracing::info!("Web UI server stopped");
    Ok(())
}

fn create_routes(
    state: Arc<AppState>,
    api_key: Option<String>,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    // API routes
    let api_routes = api::routes(state.clone())
        .with(warp::cors()
            .allow_any_origin()
            .allow_methods(vec!["GET", "POST", "PUT", "DELETE"])
            .allow_headers(vec!["Content-Type", "Authorization"]));
    
    // WebSocket route
    let ws_route = ws::route(state.clone());
    
    // Static files
    let static_files = warp::path("static")
        .and(warp::fs::dir("web-ui"));
    
    // Index route
    let index = warp::path::end()
        .and(warp::get())
        .and(warp::fs::file("web-ui/index.html"));
    
    // Health check
    let health = warp::path("health")
        .and(warp::get())
        .map(|| warp::reply::json(&serde_json::json!({
            "status": "healthy",
            "service": "mcp-proxy-web-ui"
        })));
    
    // Combine all routes
    let routes = api_routes
        .or(ws_route)
        .or(static_files)
        .or(index)
        .or(health);
    
    // Add API key authentication if configured
    if let Some(key) = api_key {
        routes.and(api_key_auth(key)).boxed()
    } else {
        routes.boxed()
    }
}

fn api_key_auth(api_key: String) -> impl Filter<Extract = (), Error = warp::Rejection> + Clone {
    warp::header::optional::<String>("authorization")
        .and_then(move |auth_header: Option<String>| {
            let api_key = api_key.clone();
            async move {
                if let Some(header) = auth_header {
                    if header == format!("Bearer {}", api_key) {
                        Ok(())
                    } else {
                        Err(warp::reject::custom(AuthError::invalid_api_key()))
                    }
                } else {
                    Err(warp::reject::custom(AuthError::missing_api_key()))
                }
            }
        })
        .untuple_one()
}

#[derive(Debug)]
struct AuthError {
    kind: AuthErrorKind,
}

#[derive(Debug)]
enum AuthErrorKind {
    MissingApiKey,
    InvalidApiKey,
}

impl AuthError {
    fn missing_api_key() -> Self {
        Self { kind: AuthErrorKind::MissingApiKey }
    }
    
    fn invalid_api_key() -> Self {
        Self { kind: AuthErrorKind::InvalidApiKey }
    }
}

impl warp::reject::Reject for AuthError {}