// API adapter that can switch between HTTP and Tauri modes
use crate::types::{ApiResponse, Metric, MetricsResponse, Server, ServersResponse};
use serde_json::Value;

#[cfg(feature = "tauri")]
use crate::tauri_api::api as tauri_api;

#[cfg(not(feature = "tauri"))]
use gloo_net::http::Request;

pub enum ApiMode {
    Http,
    Tauri,
}

impl ApiMode {
    pub fn detect() -> Self {
        // Check if we're running in Tauri context
        #[cfg(feature = "tauri")]
        {
            ApiMode::Tauri
        }
        #[cfg(not(feature = "tauri"))]
        {
            ApiMode::Http
        }
    }
}

pub struct ApiClient {
    mode: ApiMode,
}

impl ApiClient {
    pub fn new() -> Self {
        Self {
            mode: ApiMode::detect(),
        }
    }

    pub async fn fetch_servers(&self) -> Result<ServersResponse, String> {
        match self.mode {
            ApiMode::Tauri => {
                #[cfg(feature = "tauri")]
                {
                    let servers = tauri_api::get_servers().await?;
                    Ok(ServersResponse { servers })
                }
                #[cfg(not(feature = "tauri"))]
                {
                    Err("Tauri mode not available".to_string())
                }
            }
            ApiMode::Http => {
                #[cfg(not(feature = "tauri"))]
                {
                    Request::get("/api/servers")
                        .send()
                        .await
                        .map_err(|e| e.to_string())?
                        .json()
                        .await
                        .map_err(|e| e.to_string())
                }
                #[cfg(feature = "tauri")]
                {
                    Err("HTTP mode not available in Tauri build".to_string())
                }
            }
        }
    }

    pub async fn fetch_metrics(&self) -> Result<MetricsResponse, String> {
        match self.mode {
            ApiMode::Tauri => {
                #[cfg(feature = "tauri")]
                {
                    let metrics = tauri_api::get_metrics().await?;
                    Ok(MetricsResponse { metrics })
                }
                #[cfg(not(feature = "tauri"))]
                {
                    Err("Tauri mode not available".to_string())
                }
            }
            ApiMode::Http => {
                #[cfg(not(feature = "tauri"))]
                {
                    Request::get("/api/metrics")
                        .send()
                        .await
                        .map_err(|e| e.to_string())?
                        .json()
                        .await
                        .map_err(|e| e.to_string())
                }
                #[cfg(feature = "tauri")]
                {
                    Err("HTTP mode not available in Tauri build".to_string())
                }
            }
        }
    }

    pub async fn server_action(&self, name: &str, action: &str) -> Result<ApiResponse, String> {
        match self.mode {
            ApiMode::Tauri => {
                #[cfg(feature = "tauri")]
                {
                    tauri_api::server_action(name, action).await
                }
                #[cfg(not(feature = "tauri"))]
                {
                    Err("Tauri mode not available".to_string())
                }
            }
            ApiMode::Http => {
                #[cfg(not(feature = "tauri"))]
                {
                    Request::post(&format!("/api/servers/{}/{}", name, action))
                        .header("Content-Type", "application/json")
                        .send()
                        .await
                        .map_err(|e| e.to_string())?
                        .json()
                        .await
                        .map_err(|e| e.to_string())
                }
                #[cfg(feature = "tauri")]
                {
                    Err("HTTP mode not available in Tauri build".to_string())
                }
            }
        }
    }

    pub async fn get_logs(
        &self,
        server: &str,
        lines: Option<usize>,
        log_type: Option<String>,
    ) -> Result<Vec<String>, String> {
        match self.mode {
            ApiMode::Tauri => {
                #[cfg(feature = "tauri")]
                {
                    tauri_api::get_logs(server, lines, log_type).await
                }
                #[cfg(not(feature = "tauri"))]
                {
                    Err("Tauri mode not available".to_string())
                }
            }
            ApiMode::Http => {
                #[cfg(not(feature = "tauri"))]
                {
                    let mut url = format!("/api/logs/{}", server);
                    let mut params = vec![];
                    if let Some(lines) = lines {
                        params.push(format!("lines={}", lines));
                    }
                    if let Some(log_type) = log_type {
                        params.push(format!("type={}", log_type));
                    }
                    if !params.is_empty() {
                        url.push_str(&format!("?{}", params.join("&")));
                    }

                    Request::get(&url)
                        .send()
                        .await
                        .map_err(|e| e.to_string())?
                        .json()
                        .await
                        .map_err(|e| e.to_string())
                }
                #[cfg(feature = "tauri")]
                {
                    Err("HTTP mode not available in Tauri build".to_string())
                }
            }
        }
    }
}
