use crate::state::AppState;
use mcp_proxy_core::Result;
use std::sync::Arc;
use warp::Filter;

pub mod api;
pub mod ws;

#[cfg(test)]
mod api_tests;

#[cfg(test)]
mod ws_tests;

pub async fn start_server(state: Arc<AppState>) -> Result<()> {
    let config = state.config.read().await;
    let addr = format!("{}:{}", config.web_ui.host, config.web_ui.port);
    let api_key = config.web_ui.api_key.clone();
    drop(config);

    tracing::info!("Starting web UI server on {}", addr);

    // Create routes
    let routes = create_routes(state.clone(), api_key);

    // Parse address
    let addr: std::net::SocketAddr = addr.parse().map_err(|e| {
        mcp_proxy_core::ProxyError::Config(mcp_proxy_core::error::ConfigError::Parse(format!(
            "Invalid web UI address: {}",
            e
        )))
    })?;

    // Start server with graceful shutdown
    let (_, server) = warp::serve(routes).bind_with_graceful_shutdown(addr, async move {
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
    let api_routes = api::routes(state.clone());

    // WebSocket route
    let ws_route = ws::route(state.clone());

    // Health check
    let health = warp::path("health").and(warp::get()).map(|| {
        warp::reply::json(&serde_json::json!({
            "status": "healthy",
            "service": "mcp-proxy-web-ui"
        }))
    });

    // Check if Yew UI is built
    let yew_dist_path = std::path::Path::new("yew-dist");
    let use_yew_ui = yew_dist_path.exists() && yew_dist_path.is_dir();

    if use_yew_ui {
        tracing::info!("Using compiled Yew UI from yew-dist/");
    } else {
        tracing::info!("Using legacy web UI from web-ui/");
    }

    // Static files - always use directory approach
    let static_dir = if use_yew_ui { "yew-dist" } else { "web-ui" };
    let static_files = warp::fs::dir(static_dir);

    // Combine all routes
    let routes = api_routes.or(ws_route).or(health).or(static_files);

    // Apply CORS to all routes
    let routes_with_cors = routes.with(
        warp::cors()
            .allow_any_origin()
            .allow_methods(vec!["GET", "POST", "PUT", "DELETE"])
            .allow_headers(vec!["Content-Type", "Authorization"]),
    );

    // Add API key authentication if configured
    if let Some(key) = api_key {
        routes_with_cors.and(api_key_auth(key)).boxed()
    } else {
        routes_with_cors.boxed()
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
        Self {
            kind: AuthErrorKind::MissingApiKey,
        }
    }

    fn invalid_api_key() -> Self {
        Self {
            kind: AuthErrorKind::InvalidApiKey,
        }
    }
}

impl warp::reject::Reject for AuthError {}
