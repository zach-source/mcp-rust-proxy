use std::sync::Arc;
use warp::{Filter, Rejection, Reply};
use crate::state::AppState;
use crate::server::ServerManager;

pub fn routes(
    state: Arc<AppState>
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    // Server endpoints
    let servers = servers_routes(state.clone());
    
    // Metrics endpoint
    let metrics = metrics_route(state.clone());
    
    // Config endpoint
    let config = config_routes(state);
    
    warp::path("api")
        .and(servers.or(metrics).or(config))
}

fn servers_routes(
    state: Arc<AppState>
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    let list = warp::path!("servers")
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(list_servers);
    
    let action = warp::path!("servers" / String / String)
        .and(warp::post())
        .and(with_state(state.clone()))
        .and_then(server_action);
    
    let status = warp::path!("servers" / String)
        .and(warp::get())
        .and(with_state(state))
        .and_then(server_status);
    
    list.or(action).or(status)
}

fn metrics_route(
    state: Arc<AppState>
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("metrics")
        .and(warp::get())
        .and(with_state(state))
        .and_then(get_metrics)
}

fn config_routes(
    state: Arc<AppState>
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    let get = warp::path!("config")
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(get_config);
    
    let update = warp::path!("config")
        .and(warp::put())
        .and(warp::body::json())
        .and(with_state(state))
        .and_then(update_config);
    
    get.or(update)
}

fn with_state(
    state: Arc<AppState>
) -> impl Filter<Extract = (Arc<AppState>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || state.clone())
}

async fn list_servers(state: Arc<AppState>) -> Result<impl Reply, Rejection> {
    let servers: Vec<_> = state.servers.iter()
        .map(|entry| {
            let name = entry.key().clone();
            let info = entry.value();
            let state = info.state.try_read()
                .map(|s| format!("{:?}", *s))
                .unwrap_or_else(|_| "Unknown".to_string());
            
            serde_json::json!({
                "name": name,
                "state": state,
                "restart_count": info.restart_count.try_read()
                    .map(|guard| *guard)
                    .unwrap_or(0),
            })
        })
        .collect();
    
    Ok(warp::reply::json(&serde_json::json!({
        "servers": servers
    })))
}

async fn server_action(
    name: String,
    action: String,
    state: Arc<AppState>,
) -> Result<impl Reply, Rejection> {
    let manager = ServerManager::new(state.clone(), state.shutdown_tx.subscribe());
    
    let result = match action.as_str() {
        "start" => manager.start_server(&name).await,
        "stop" => manager.stop_server(&name).await,
        "restart" => manager.restart_server(&name).await,
        _ => return Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "error": format!("Unknown action: {}", action)
            })),
            warp::http::StatusCode::BAD_REQUEST,
        )),
    };
    
    match result {
        Ok(_) => Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "status": "success",
                "message": format!("Server {} action {} completed", name, action)
            })),
            warp::http::StatusCode::OK,
        )),
        Err(e) => Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "error": e.to_string()
            })),
            warp::http::StatusCode::INTERNAL_SERVER_ERROR,
        )),
    }
}

async fn server_status(
    name: String,
    state: Arc<AppState>,
) -> Result<impl Reply, Rejection> {
    if let Some(info) = state.servers.get(&name) {
        let server_state = info.state.read().await;
        let restart_count = info.restart_count.read().await;
        
        Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "name": name,
                "state": format!("{:?}", *server_state),
                "restart_count": *restart_count,
            })),
            warp::http::StatusCode::OK,
        ))
    } else {
        Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "error": format!("Server not found: {}", name)
            })),
            warp::http::StatusCode::NOT_FOUND,
        ))
    }
}

async fn get_metrics(state: Arc<AppState>) -> Result<impl Reply, Rejection> {
    let metrics = state.metrics.gather_metrics();
    
    // Convert to JSON format
    let json_metrics: Vec<_> = metrics.into_iter()
        .map(|family| {
            serde_json::json!({
                "name": family.get_name(),
                "help": family.get_help(),
                "type": format!("{:?}", family.get_field_type()),
                "metrics": family.get_metric().iter().map(|m| {
                    serde_json::json!({
                        "labels": m.get_label().iter().map(|l| {
                            (l.get_name(), l.get_value())
                        }).collect::<std::collections::HashMap<_, _>>(),
                        "value": match m.get_counter().get_value() as i64 {
                            v if v != 0 => v as f64,
                            _ => m.get_gauge().get_value(),
                        }
                    })
                }).collect::<Vec<_>>()
            })
        })
        .collect();
    
    Ok(warp::reply::json(&serde_json::json!({
        "metrics": json_metrics
    })))
}

async fn get_config(state: Arc<AppState>) -> Result<impl Reply, Rejection> {
    let config = state.config.read().await;
    Ok(warp::reply::json(&*config))
}

async fn update_config(
    new_config: crate::config::Config,
    state: Arc<AppState>,
) -> Result<impl Reply, Rejection> {
    match state.update_config(new_config).await {
        Ok(_) => Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "status": "success",
                "message": "Configuration updated successfully"
            })),
            warp::http::StatusCode::OK,
        )),
        Err(e) => Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "error": e.to_string()
            })),
            warp::http::StatusCode::BAD_REQUEST,
        )),
    }
}