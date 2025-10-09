// Tauri API integration for Yew frontend
use serde::{Deserialize, Serialize};
use serde_json::Value;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;

    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "event"])]
    async fn listen(event: &str, callback: &Closure<dyn FnMut(JsValue)>) -> JsValue;

    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "event"])]
    fn emit(event: &str, payload: JsValue);
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InvokeArgs<T> {
    #[serde(flatten)]
    pub data: T,
}

pub async fn invoke_tauri<T, R>(cmd: &str, args: T) -> Result<R, String>
where
    T: Serialize,
    R: for<'de> Deserialize<'de>,
{
    let args = InvokeArgs { data: args };
    let js_args = JsValue::from_serde(&args).map_err(|e| e.to_string())?;

    let result = invoke(cmd, js_args).await;

    result
        .into_serde()
        .map_err(|e| format!("Failed to deserialize response: {}", e))
}

pub fn listen_tauri<F>(event: &str, callback: F) -> Result<(), String>
where
    F: FnMut(JsValue) + 'static,
{
    let closure = Closure::new(callback);

    spawn_local(async move {
        let _unlisten = listen(event, &closure).await;
        closure.forget(); // Keep the closure alive
    });

    Ok(())
}

pub fn emit_tauri<T>(event: &str, payload: T) -> Result<(), String>
where
    T: Serialize,
{
    let js_payload = JsValue::from_serde(&payload).map_err(|e| e.to_string())?;
    emit(event, js_payload);
    Ok(())
}

// API wrapper functions for Yew components
pub mod api {
    use super::*;
    use crate::types::{ApiResponse, Metric, Server};

    #[derive(Serialize)]
    struct ServerActionArgs {
        name: String,
        action: String,
    }

    #[derive(Serialize)]
    struct LogsArgs {
        server: String,
        lines: Option<usize>,
        log_type: Option<String>,
    }

    pub async fn get_servers() -> Result<Vec<Server>, String> {
        invoke_tauri("get_servers", ()).await
    }

    pub async fn server_action(name: &str, action: &str) -> Result<ApiResponse, String> {
        invoke_tauri(
            "server_action",
            ServerActionArgs {
                name: name.to_string(),
                action: action.to_string(),
            },
        )
        .await
    }

    pub async fn get_metrics() -> Result<Vec<Metric>, String> {
        invoke_tauri("get_metrics", ()).await
    }

    pub async fn get_logs(
        server: &str,
        lines: Option<usize>,
        log_type: Option<String>,
    ) -> Result<Vec<String>, String> {
        invoke_tauri(
            "get_logs",
            LogsArgs {
                server: server.to_string(),
                lines,
                log_type,
            },
        )
        .await
    }

    pub async fn get_config() -> Result<Value, String> {
        invoke_tauri("get_config", ()).await
    }

    pub async fn update_config(config: Value) -> Result<(), String> {
        invoke_tauri("update_config", config).await
    }
}
