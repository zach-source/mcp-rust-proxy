use crate::api::websocket::WsStatus;
use crate::types::Stats;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct HeaderProps {
    pub stats: Option<Stats>,
    pub ws_status: WsStatus,
}

#[function_component(Header)]
pub fn header(props: &HeaderProps) -> Html {
    let status_class = match props.ws_status {
        WsStatus::Connected => "connected",
        _ => "disconnected",
    };

    let status_text = match props.ws_status {
        WsStatus::Connected => "Connected",
        WsStatus::Connecting => "Connecting...",
        WsStatus::Disconnected => "Disconnected",
    };

    let total_servers = props
        .stats
        .as_ref()
        .map(|s| s.total_servers)
        .unwrap_or(0);
    let running_servers = props
        .stats
        .as_ref()
        .map(|s| s.running_servers)
        .unwrap_or(0);

    html! {
        <header>
            <h1>{"MCP Proxy Server Dashboard"}</h1>
            <div class="stats">
                <span>{"Total Servers: "}<span id="total-servers">{total_servers}</span></span>
                <span>{"Running: "}<span id="running-servers">{running_servers}</span></span>
                <span>{"Status: "}<span class={format!("status-indicator {}", status_class)}>{status_text}</span></span>
            </div>
        </header>
    }
}