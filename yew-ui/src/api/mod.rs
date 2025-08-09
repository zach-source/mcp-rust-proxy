pub mod log_stream;
pub mod websocket;

use crate::types::{ApiResponse, MetricsResponse, ServersResponse};
use gloo_net::http::Request;

const API_BASE: &str = "/api";

pub async fn fetch_servers() -> Result<ServersResponse, gloo_net::Error> {
    Request::get(&format!("{}/servers", API_BASE))
        .send()
        .await?
        .json()
        .await
}

pub async fn fetch_metrics() -> Result<MetricsResponse, gloo_net::Error> {
    Request::get(&format!("{}/metrics", API_BASE))
        .send()
        .await?
        .json()
        .await
}

pub async fn server_action(
    server_name: &str,
    action: &str,
) -> Result<ApiResponse, gloo_net::Error> {
    let url = format!("{}/servers/{}/{}", API_BASE, server_name, action);
    web_sys::console::log_1(&format!("Sending POST request to: {}", url).into());

    let response = Request::post(&url)
        .header("Content-Type", "application/json")
        .send()
        .await?;

    let status = response.status();
    web_sys::console::log_1(&format!("Response status: {}", status).into());

    response.json().await
}

pub async fn toggle_server_disable(server_name: &str) -> Result<ApiResponse, gloo_net::Error> {
    let url = format!("{}/servers/{}/toggle-disable", API_BASE, server_name);
    web_sys::console::log_1(&format!("Toggling disable state for: {}", server_name).into());

    let response = Request::post(&url)
        .header("Content-Type", "application/json")
        .send()
        .await?;

    response.json().await
}
