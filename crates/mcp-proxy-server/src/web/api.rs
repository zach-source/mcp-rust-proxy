use crate::server::ServerManager;
use crate::state::AppState;
use futures::stream;
use futures::stream::{Stream, StreamExt};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, AsyncSeekExt, BufReader};
use tokio::time::{interval, Duration};
use warp::{Filter, Rejection, Reply};

pub fn routes(
    state: Arc<AppState>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    // Server endpoints
    let servers = servers_routes(state.clone());

    // Log endpoints
    let logs = logs_routes(state.clone());

    // Metrics endpoint
    let metrics = metrics_route(state.clone());

    // Config endpoint
    let config = config_routes(state);

    warp::path("api").and(servers.or(logs).or(metrics).or(config))
}

fn servers_routes(
    state: Arc<AppState>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    let list = warp::path!("servers")
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(list_servers);

    let action = warp::path!("servers" / String / String)
        .and(warp::post())
        .and(with_state(state.clone()))
        .and_then(server_action);

    let toggle_disable = warp::path!("servers" / String / "toggle-disable")
        .and(warp::post())
        .and(with_state(state.clone()))
        .and_then(toggle_server_disable);

    let status = warp::path!("servers" / String)
        .and(warp::get())
        .and(with_state(state))
        .and_then(server_status);

    list.or(toggle_disable).or(action).or(status)
}

fn logs_routes(
    state: Arc<AppState>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    let logs_tail = warp::path!("logs" / String)
        .and(warp::get())
        .and(warp::query::<std::collections::HashMap<String, String>>())
        .and(with_state(state.clone()))
        .and_then(get_server_logs);

    let logs_stream = warp::path!("logs" / String / "stream")
        .and(warp::get())
        .and(warp::query::<std::collections::HashMap<String, String>>())
        .and(with_state(state))
        .and_then(stream_server_logs);

    logs_tail.or(logs_stream)
}

fn metrics_route(
    state: Arc<AppState>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("metrics")
        .and(warp::get())
        .and(with_state(state))
        .and_then(get_metrics)
}

fn config_routes(
    state: Arc<AppState>,
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
    state: Arc<AppState>,
) -> impl Filter<Extract = (Arc<AppState>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || state.clone())
}

async fn list_servers(state: Arc<AppState>) -> Result<impl Reply, Rejection> {
    let mut servers = Vec::new();

    for entry in state.servers.iter() {
        let name = entry.key().clone();
        let info = entry.value();

        let server_state = info.state.read().await;
        let state_str = match *server_state {
            crate::state::ServerState::Starting => "starting",
            crate::state::ServerState::Running => "running",
            crate::state::ServerState::Stopping => "stopping",
            crate::state::ServerState::Stopped => "stopped",
            crate::state::ServerState::Failed => "failed",
        }
        .to_string();

        let restart_count = *info.restart_count.read().await;

        let last_health_check = info.last_health_check.read().await;
        let health_check_data = last_health_check.as_ref().map(|hc| {
            serde_json::json!({
                "timestamp": hc.timestamp.to_rfc3339(),
                "success": hc.success,
                "response_time_ms": hc.response_time_ms,
                "error": hc.error
            })
        });

        let last_access_time = info.last_access_time.read().await;
        let last_access = last_access_time.as_ref().map(|t| t.to_rfc3339());

        // Check if health checks are enabled for this server
        let config = state.config.read().await;
        let health_check_enabled = config.get_server_health_check(&name).is_some();
        drop(config);

        // Check if server is disabled
        let disabled_servers = state.disabled_servers.read().await;
        let disabled = disabled_servers.is_disabled(&name);
        drop(disabled_servers);

        servers.push(serde_json::json!({
            "name": name,
            "state": state_str,
            "restart_count": restart_count,
            "health_check_enabled": health_check_enabled,
            "last_health_check": health_check_data,
            "last_access_time": last_access,
            "disabled": disabled
        }));
    }

    Ok(warp::reply::json(&serde_json::json!({
        "servers": servers
    })))
}

async fn toggle_server_disable(
    name: String,
    state: Arc<AppState>,
) -> Result<impl Reply, Rejection> {
    let mut disabled_servers = state.disabled_servers.write().await;

    match disabled_servers.toggle(&name).await {
        Ok(disabled) => {
            // If disabling the server and it's running, stop it
            if disabled {
                if let Some(info) = state.servers.get(&name) {
                    let server_state = info.state.read().await;
                    if matches!(*server_state, crate::state::ServerState::Running) {
                        drop(server_state);
                        let manager =
                            ServerManager::new(state.clone(), state.shutdown_tx.subscribe());
                        let _ = manager.stop_server(&name).await;
                    }
                }
            }

            Ok(warp::reply::with_status(
                warp::reply::json(&serde_json::json!({
                    "status": "success",
                    "disabled": disabled,
                    "message": format!("Server {} is now {}", name, if disabled { "disabled" } else { "enabled" })
                })),
                warp::http::StatusCode::OK,
            ))
        }
        Err(e) => Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "error": e.to_string()
            })),
            warp::http::StatusCode::INTERNAL_SERVER_ERROR,
        )),
    }
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
        _ => {
            return Ok(warp::reply::with_status(
                warp::reply::json(&serde_json::json!({
                    "error": format!("Unknown action: {}", action)
                })),
                warp::http::StatusCode::BAD_REQUEST,
            ))
        }
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

async fn server_status(name: String, state: Arc<AppState>) -> Result<impl Reply, Rejection> {
    if let Some(info) = state.servers.get(&name) {
        let server_state = info.state.read().await;
        let restart_count = info.restart_count.read().await;

        let last_health_check = info.last_health_check.read().await;
        let health_check_data = last_health_check.as_ref().map(|hc| {
            serde_json::json!({
                "timestamp": hc.timestamp.to_rfc3339(),
                "success": hc.success,
                "response_time_ms": hc.response_time_ms,
                "error": hc.error
            })
        });

        let last_access_time = info.last_access_time.read().await;
        let last_access = last_access_time.as_ref().map(|t| t.to_rfc3339());

        // Check if health checks are enabled for this server
        let config = state.config.read().await;
        let health_check_enabled = config.get_server_health_check(&name).is_some();
        drop(config);

        Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "name": name,
                "state": format!("{:?}", *server_state),
                "restart_count": *restart_count,
                "health_check_enabled": health_check_enabled,
                "last_health_check": health_check_data,
                "last_access_time": last_access
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
    let json_metrics: Vec<_> = metrics
        .into_iter()
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
    new_config: mcp_proxy_core::Config,
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

async fn get_server_logs(
    server_name: String,
    query_params: std::collections::HashMap<String, String>,
    _state: Arc<AppState>,
) -> Result<impl Reply, Rejection> {
    // Get log directory path
    let home = match dirs::home_dir() {
        Some(dir) => dir,
        None => {
            return Ok(warp::reply::with_status(
                warp::reply::json(&serde_json::json!({
                    "error": "Could not determine home directory"
                })),
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            ));
        }
    };

    let log_file_path = home
        .join(".mcp-proxy")
        .join("logs")
        .join(&server_name)
        .join("server.log");

    // Check if log file exists
    if !log_file_path.exists() {
        return Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "error": format!("Log file not found for server: {}", server_name)
            })),
            warp::http::StatusCode::NOT_FOUND,
        ));
    }

    // Parse query parameters
    let lines = query_params
        .get("lines")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(100); // Default to last 100 lines

    let filter_type = query_params.get("type"); // Optional filter: "stdout", "stderr", or none for both

    // Read the last N lines from the file
    match read_last_lines(&log_file_path, lines, filter_type).await {
        Ok(log_lines) => Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "server": server_name,
                "lines": log_lines,
                "file_path": log_file_path.to_string_lossy(),
                "filter": filter_type.map_or("all", |v| v.as_str())
            })),
            warp::http::StatusCode::OK,
        )),
        Err(e) => {
            tracing::error!("Error reading log file {:?}: {}", log_file_path, e);
            Ok(warp::reply::with_status(
                warp::reply::json(&serde_json::json!({
                    "error": format!("Failed to read log file: {}", e)
                })),
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}

async fn read_last_lines(
    file_path: &PathBuf,
    num_lines: usize,
    filter_type: Option<&String>,
) -> Result<Vec<String>, std::io::Error> {
    let file = File::open(file_path).await?;
    let mut reader = BufReader::new(file);

    // Get file size
    let metadata = tokio::fs::metadata(file_path).await?;
    let file_size = metadata.len();

    // If file is empty, return empty vec
    if file_size == 0 {
        return Ok(Vec::new());
    }

    // For efficiency, we'll read from the end of the file
    // This is a simple implementation - for very large files,
    // we might want to implement a more sophisticated approach
    let mut all_lines = Vec::new();
    let mut line = String::new();

    // Read all lines (for now - could optimize for very large files)
    loop {
        line.clear();
        let bytes_read = reader.read_line(&mut line).await?;
        if bytes_read == 0 {
            break;
        }
        all_lines.push(line.trim_end().to_string());
    }

    // Filter lines by type if specified
    let filtered_lines: Vec<String> = if let Some(filter) = filter_type {
        let filter_upper = filter.to_uppercase();
        all_lines
            .into_iter()
            .filter(|line| {
                match filter_upper.as_str() {
                    "STDOUT" => line.contains("[STDOUT]"),
                    "STDERR" => line.contains("[STDERR]"),
                    _ => true, // If invalid filter, return all lines
                }
            })
            .collect()
    } else {
        all_lines
    };

    // Return the last N lines
    let start_index = if filtered_lines.len() > num_lines {
        filtered_lines.len() - num_lines
    } else {
        0
    };

    Ok(filtered_lines[start_index..].to_vec())
}

async fn stream_server_logs(
    server_name: String,
    query_params: std::collections::HashMap<String, String>,
    _state: Arc<AppState>,
) -> Result<impl Reply, Rejection> {
    // Get log directory path
    let home = match dirs::home_dir() {
        Some(dir) => dir,
        None => {
            return Err(warp::reject::custom(LogStreamError::HomeDirectoryNotFound));
        }
    };

    let log_file_path = home
        .join(".mcp-proxy")
        .join("logs")
        .join(&server_name)
        .join("server.log");

    // Check if log file exists
    if !log_file_path.exists() {
        return Err(warp::reject::custom(LogStreamError::LogFileNotFound(
            server_name,
        )));
    }

    let filter_type = query_params.get("type").cloned(); // Optional filter: "stdout", "stderr", or none for both

    // Create the log stream
    let log_stream = create_log_stream(log_file_path, filter_type);

    // Convert to SSE format
    let sse_stream =
        log_stream.map(|line| Ok::<_, warp::Error>(warp::sse::Event::default().data(line)));

    let reply = warp::sse::reply(sse_stream);
    Ok(warp::reply::with_header(reply, "Cache-Control", "no-cache"))
}

#[derive(Debug)]
enum LogStreamError {
    HomeDirectoryNotFound,
    LogFileNotFound(String),
}

impl warp::reject::Reject for LogStreamError {}

fn create_log_stream(
    log_file_path: PathBuf,
    filter_type: Option<String>,
) -> impl Stream<Item = String> {
    stream::unfold(
        (log_file_path, filter_type, 0u64),
        move |(path, filter, mut last_position)| async move {
            let mut ticker = interval(Duration::from_millis(500)); // Check every 500ms

            loop {
                ticker.tick().await;

                match File::open(&path).await {
                    Ok(mut file) => {
                        // Seek to the last known position
                        if file
                            .seek(std::io::SeekFrom::Start(last_position))
                            .await
                            .is_err()
                        {
                            continue;
                        }

                        let mut reader = BufReader::new(file);
                        let mut line = String::new();
                        let mut new_lines = Vec::new();

                        // Read new lines from the current position
                        while let Ok(bytes_read) = reader.read_line(&mut line).await {
                            if bytes_read == 0 {
                                break; // EOF
                            }

                            let trimmed_line = line.trim_end().to_string();

                            // Apply filter if specified
                            let should_include = if let Some(ref filter) = filter {
                                match filter.to_uppercase().as_str() {
                                    "STDOUT" => trimmed_line.contains("[STDOUT]"),
                                    "STDERR" => trimmed_line.contains("[STDERR]"),
                                    _ => true,
                                }
                            } else {
                                true
                            };

                            if should_include {
                                new_lines.push(trimmed_line);
                            }

                            last_position += bytes_read as u64;
                            line.clear();
                        }

                        if !new_lines.is_empty() {
                            let combined_lines = new_lines.join("\n");
                            return Some((combined_lines, (path, filter, last_position)));
                        }
                    }
                    Err(_) => {
                        // File might not exist or be accessible, continue waiting
                        continue;
                    }
                }
            }
        },
    )
}
