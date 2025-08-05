use crate::types::{WsCommand, WsMessage};
use gloo_timers::callback::Interval;
use serde_json;
use wasm_bindgen::{prelude::*, JsCast};
use web_sys::{MessageEvent, WebSocket};
use yew::prelude::*;

#[derive(Clone, PartialEq)]
pub enum WsStatus {
    Connected,
    Disconnected,
    Connecting,
}

pub enum WsAction {
    Connect,
    Disconnect,
    SendMessage(WsCommand),
}

pub struct WebSocketService {
    ws: Option<WebSocket>,
    _reconnect_interval: Option<Interval>,
}

impl WebSocketService {
    pub fn new() -> Self {
        Self {
            ws: None,
            _reconnect_interval: None,
        }
    }

    pub fn connect(&mut self, callback: Callback<WsMessage>) -> Result<(), JsValue> {
        let window = web_sys::window().unwrap();
        let location = window.location();
        let protocol = if location.protocol()? == "https:" {
            "wss:"
        } else {
            "ws:"
        };
        let host = location.host()?;
        let ws_url = format!("{}//{}/api/ws", protocol, host);

        web_sys::console::log_1(&format!("Connecting to WebSocket: {}", ws_url).into());

        let ws = WebSocket::new(&ws_url)?;
        ws.set_binary_type(web_sys::BinaryType::Arraybuffer);

        // Set up message handler
        let onmessage_callback = Closure::<dyn FnMut(_)>::new(move |e: MessageEvent| {
            if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
                let msg_str: String = txt.into();
                web_sys::console::log_1(&format!("WS Message received: {}", msg_str).into());
                if let Ok(msg) = serde_json::from_str::<WsMessage>(&msg_str) {
                    callback.emit(msg);
                } else {
                    web_sys::console::error_1(
                        &format!("Failed to parse WS message: {}", msg_str).into(),
                    );
                }
            }
        });
        ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
        onmessage_callback.forget();

        // Set up open handler
        let onopen_callback = Closure::<dyn FnMut()>::new(move || {
            web_sys::console::log_1(&"WebSocket connected successfully".into());
        });
        ws.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
        onopen_callback.forget();

        // Set up error handler
        let onerror_callback = Closure::<dyn FnMut(_)>::new(move |e: web_sys::ErrorEvent| {
            web_sys::console::error_1(&format!("WebSocket error: {:?}", e).into());
        });
        ws.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
        onerror_callback.forget();

        self.ws = Some(ws);
        Ok(())
    }

    pub fn send(&self, command: WsCommand) -> Result<(), JsValue> {
        if let Some(ws) = &self.ws {
            if ws.ready_state() == WebSocket::OPEN {
                let msg = serde_json::to_string(&command).unwrap();
                ws.send_with_str(&msg)?;
            }
        }
        Ok(())
    }

    pub fn close(&mut self) {
        if let Some(ws) = &self.ws {
            let _ = ws.close();
        }
        self.ws = None;
    }

    pub fn status(&self) -> WsStatus {
        match &self.ws {
            Some(ws) => match ws.ready_state() {
                WebSocket::OPEN => WsStatus::Connected,
                WebSocket::CONNECTING => WsStatus::Connecting,
                _ => WsStatus::Disconnected,
            },
            None => WsStatus::Disconnected,
        }
    }
}
