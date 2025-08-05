pub mod websocket;
pub mod log_stream;

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
    Request::post(&format!("{}/servers/{}/{}", API_BASE, server_name, action))
        .header("Content-Type", "application/json")
        .send()
        .await?
        .json()
        .await
}