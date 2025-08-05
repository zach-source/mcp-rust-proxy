use crate::types::LogData;
use wasm_bindgen::prelude::*;
use web_sys::{EventSource, MessageEvent};
use yew::prelude::*;

pub struct LogStreamService {
    event_source: Option<EventSource>,
    callback: Option<Callback<String>>,
}

impl LogStreamService {
    pub fn new() -> Self {
        Self {
            event_source: None,
            callback: None,
        }
    }

    pub fn start_streaming(
        &mut self,
        server_name: &str,
        callback: Callback<String>,
    ) -> Result<(), JsValue> {
        // Close existing stream if any
        self.stop_streaming();

        let url = format!("/api/logs/{}/stream", server_name);

        match EventSource::new(&url) {
            Ok(event_source) => {
                let callback_clone = callback.clone();

                let onmessage_callback = Closure::wrap(Box::new(move |e: MessageEvent| {
                    if let Ok(data) = e.data().dyn_into::<js_sys::JsString>() {
                        let message: String = data.into();
                        callback_clone.emit(message);
                    }
                }) as Box<dyn FnMut(_)>);

                event_source.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
                onmessage_callback.forget();

                let onerror_callback = Closure::wrap(Box::new(move |e: web_sys::Event| {
                    web_sys::console::error_1(&JsValue::from_str(&format!("SSE error: {:?}", e)));
                }) as Box<dyn FnMut(_)>);

                event_source.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
                onerror_callback.forget();

                self.event_source = Some(event_source);
                self.callback = Some(callback);
                Ok(())
            }
            Err(e) => {
                web_sys::console::error_1(&JsValue::from_str(&format!(
                    "Failed to create EventSource: {:?}",
                    e
                )));
                Err(e)
            }
        }
    }

    pub fn stop_streaming(&mut self) {
        if let Some(event_source) = self.event_source.take() {
            event_source.close();
        }
        self.callback = None;
    }
}

impl Drop for LogStreamService {
    fn drop(&mut self) {
        self.stop_streaming();
    }
}

pub fn parse_log_line(line: &str) -> Option<LogData> {
    // Parse the combined log format: [timestamp] [STDOUT/STDERR] message
    if line.starts_with('[') {
        let parts: Vec<&str> = line.splitn(3, ']').collect();
        if parts.len() >= 3 {
            let timestamp = parts[0].trim_start_matches('[');
            let level = parts[1].trim_start_matches(" [").trim();
            let message = parts[2].trim_start_matches(' ');

            return Some(LogData {
                server: String::new(), // Will be set by the caller
                timestamp: Some(timestamp.to_string()),
                level: Some(level.to_string()),
                message: Some(message.to_string()),
            });
        }
    }

    // Fallback: treat the whole line as message
    Some(LogData {
        server: String::new(),
        timestamp: None,
        level: None,
        message: Some(line.to_string()),
    })
}
