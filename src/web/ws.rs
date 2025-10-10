use crate::state::AppState;
use dashmap::DashMap;
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use warp::{Filter, Rejection};

#[derive(Debug, Deserialize)]
struct WsMessage {
    #[serde(rename = "type")]
    msg_type: String,
    #[serde(default)]
    server: Option<String>,
}

#[derive(Debug, Serialize)]
#[allow(dead_code)]
struct WsResponse {
    #[serde(rename = "type")]
    msg_type: String,
    data: serde_json::Value,
}

pub fn route(
    state: Arc<AppState>,
) -> impl Filter<Extract = impl warp::Reply, Error = Rejection> + Clone {
    warp::path!("api" / "ws")
        .and(warp::ws())
        .and(warp::any().map(move || state.clone()))
        .map(|ws: warp::ws::Ws, state| ws.on_upgrade(move |socket| client_connected(socket, state)))
}

async fn client_connected(ws: warp::ws::WebSocket, state: Arc<AppState>) {
    let (mut ws_tx, mut ws_rx) = ws.split();
    let (tx, rx) = mpsc::unbounded_channel();
    let mut rx = UnboundedReceiverStream::new(rx);

    // Spawn task to forward messages from channel to websocket
    tokio::spawn(async move {
        while let Some(msg) = rx.next().await {
            if let Err(e) = ws_tx.send(msg).await {
                tracing::error!("WebSocket send error: {}", e);
                break;
            }
        }
    });

    // Send initial state
    let initial_data = collect_state_update(&state).await;
    let msg = warp::ws::Message::text(
        serde_json::json!({
            "type": "initial",
            "data": initial_data
        })
        .to_string(),
    );

    if let Err(e) = tx.send(msg) {
        tracing::error!("Failed to send initial state: {}", e);
        return;
    }

    // Track what the client is subscribed to
    let subscriptions = Arc::new(DashMap::new());

    // Track previous state to send only changes
    let mut previous_state = initial_data.clone();

    // Subscribe to state changes
    let mut shutdown_rx = state.shutdown_tx.subscribe();

    // Start update loop - reduce frequency to avoid constant UI refreshes
    let update_interval = tokio::time::interval(tokio::time::Duration::from_secs(5));
    tokio::pin!(update_interval);

    loop {
        tokio::select! {
            _ = update_interval.tick() => {
                // Send periodic updates - only if something changed
                let current_state = collect_state_update(&state).await;

                // Only send if state has changed
                if current_state != previous_state {
                    let msg = warp::ws::Message::text(serde_json::json!({
                        "type": "update",
                        "data": current_state
                    }).to_string());

                    if tx.send(msg).is_err() {
                        break;
                    }

                    previous_state = current_state;
                }
            }
            _ = shutdown_rx.recv() => {
                tracing::debug!("WebSocket connection closing due to shutdown");
                break;
            }
            msg = ws_rx.next() => {
                match msg {
                    Some(Ok(msg)) => {
                        if msg.is_close() {
                            break;
                        }
                        if let Ok(text) = msg.to_str() {
                            if let Ok(ws_msg) = serde_json::from_str::<WsMessage>(text) {
                                handle_ws_message(ws_msg, &state, &tx, &subscriptions).await;
                            }
                        }
                    }
                    Some(Err(e)) => {
                        tracing::error!("WebSocket error: {}", e);
                        break;
                    }
                    None => break,
                }
            }
        }
    }

    tracing::debug!("WebSocket client disconnected");
}

async fn collect_state_update(state: &Arc<AppState>) -> serde_json::Value {
    let mut servers = Vec::new();

    for entry in state.servers.iter() {
        let name = entry.key().clone();
        let info = entry.value();

        let state_val = info.state.read().await;
        let state_str = match *state_val {
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

        servers.push(serde_json::json!({
            "name": name,
            "state": state_str,
            "restart_count": restart_count,
            "health_check_enabled": health_check_enabled,
            "last_health_check": health_check_data,
            "last_access_time": last_access
        }));
    }

    let metrics = state.metrics.gather_metrics();
    let total_servers = metrics
        .iter()
        .find(|m| m.get_name() == "mcp_proxy_total_servers")
        .and_then(|m| m.get_metric().first())
        .map(|m| m.get_gauge().get_value() as i64)
        .unwrap_or(0);

    let running_servers = metrics
        .iter()
        .find(|m| m.get_name() == "mcp_proxy_running_servers")
        .and_then(|m| m.get_metric().first())
        .map(|m| m.get_gauge().get_value() as i64)
        .unwrap_or(0);

    serde_json::json!({
        "servers": servers,
        "stats": {
            "total_servers": total_servers,
            "running_servers": running_servers,
        }
    })
}

async fn handle_ws_message(
    msg: WsMessage,
    state: &Arc<AppState>,
    tx: &mpsc::UnboundedSender<warp::ws::Message>,
    subscriptions: &Arc<DashMap<String, bool>>,
) {
    match msg.msg_type.as_str() {
        "subscribe_logs" => {
            if let Some(server_name) = msg.server {
                let subscription_key = format!("logs_{}", server_name);
                subscriptions.insert(subscription_key.clone(), true);

                // Subscribe to log stream from server
                if let Some(server_info) = state.servers.get(&server_name) {
                    let server_info = server_info.value();
                    let mut log_rx = server_info.subscribe_logs(subscription_key.clone());

                    let tx_clone = tx.clone();
                    let server_name_clone = server_name.clone();
                    let subscriptions_clone = subscriptions.clone();

                    tokio::spawn(async move {
                        while let Some(log_entry) = log_rx.recv().await {
                            // Check if still subscribed
                            if !subscriptions_clone.contains_key(&subscription_key) {
                                break;
                            }

                            let log_msg = serde_json::json!({
                                "type": "log",
                                "data": {
                                    "server": server_name_clone,
                                    "timestamp": log_entry.timestamp.to_rfc3339(),
                                    "level": log_entry.level,
                                    "message": log_entry.message,
                                }
                            });

                            if tx_clone
                                .send(warp::ws::Message::text(log_msg.to_string()))
                                .is_err()
                            {
                                break;
                            }
                        }
                    });
                }

                // Don't send confirmation - client doesn't expect it
                // let response = WsResponse {
                //     msg_type: "subscribed".to_string(),
                //     data: serde_json::json!({ "server": server_name, "type": "logs" }),
                // };
                // let _ = tx.send(warp::ws::Message::text(serde_json::to_string(&response).unwrap()));
            }
        }
        "unsubscribe_logs" => {
            if let Some(server_name) = msg.server {
                let subscription_key = format!("logs_{}", server_name);
                subscriptions.remove(&subscription_key);

                // Unsubscribe from server logs
                if let Some(server_info) = state.servers.get(&server_name) {
                    server_info.value().unsubscribe_logs(&subscription_key);
                }

                // Don't send confirmation - client doesn't expect it
                // let response = WsResponse {
                //     msg_type: "unsubscribed".to_string(),
                //     data: serde_json::json!({ "server": server_name, "type": "logs" }),
                // };
                // let _ = tx.send(warp::ws::Message::text(serde_json::to_string(&response).unwrap()));
            }
        }
        _ => {
            tracing::debug!("Unknown WebSocket message type: {}", msg.msg_type);
        }
    }
}
