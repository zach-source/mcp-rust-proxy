use std::sync::Arc;
use warp::{Filter, Rejection};
use futures::{StreamExt, SinkExt};
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use crate::state::AppState;

pub fn route(
    state: Arc<AppState>
) -> impl Filter<Extract = impl warp::Reply, Error = Rejection> + Clone {
    warp::path!("api" / "ws")
        .and(warp::ws())
        .and(warp::any().map(move || state.clone()))
        .map(|ws: warp::ws::Ws, state| {
            ws.on_upgrade(move |socket| client_connected(socket, state))
        })
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
            })
        })
        .collect();
    
    let msg = warp::ws::Message::text(serde_json::json!({
        "type": "initial",
        "data": {
            "servers": servers
        }
    }).to_string());
    
    if let Err(e) = tx.send(msg) {
        tracing::error!("Failed to send initial state: {}", e);
        return;
    }
    
    // Subscribe to state changes
    let mut shutdown_rx = state.shutdown_tx.subscribe();
    
    // Start update loop
    let update_interval = tokio::time::interval(tokio::time::Duration::from_secs(1));
    tokio::pin!(update_interval);
    
    loop {
        tokio::select! {
            _ = update_interval.tick() => {
                // Send periodic updates
                let update = collect_state_update(&state).await;
                let msg = warp::ws::Message::text(serde_json::json!({
                    "type": "update",
                    "data": update
                }).to_string());
                
                if tx.send(msg).is_err() {
                    break;
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
                        // Handle incoming messages if needed
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
            })
        })
        .collect();
    
    let metrics = state.metrics.gather_metrics();
    let total_servers = metrics.iter()
        .find(|m| m.get_name() == "mcp_proxy_total_servers")
        .and_then(|m| m.get_metric().first())
        .map(|m| m.get_gauge().get_value() as i64)
        .unwrap_or(0);
    
    let running_servers = metrics.iter()
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