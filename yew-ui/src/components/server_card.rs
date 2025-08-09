use crate::types::{Server, ServerState};
use chrono::Utc;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct ServerCardProps {
    pub server: Server,
    pub on_action: Callback<(String, String)>, // (server_name, action)
    pub on_view_logs: Callback<String>,
    pub on_toggle_disable: Callback<String>,
}

#[function_component(ServerCard)]
pub fn server_card(props: &ServerCardProps) -> Html {
    let server = &props.server;
    let state_lower = match server.state {
        ServerState::Running => "running",
        ServerState::Stopped => "stopped",
        ServerState::Failed => "failed",
        ServerState::Starting => "starting",
        ServerState::Stopping => "stopping",
    };

    let is_running = matches!(server.state, ServerState::Running);
    let is_stopped = matches!(server.state, ServerState::Stopped);

    // Format health check status
    let health_status = if server.health_check_enabled {
        if let Some(health_check) = &server.last_health_check {
            let health_class = if health_check.success {
                "healthy"
            } else {
                "unhealthy"
            };
            let response_time = health_check
                .response_time_ms
                .map(|ms| format!("{}ms", ms))
                .unwrap_or_else(|| "N/A".to_string());

            html! {
                <>
                    <div>
                        <span class="health-status">
                            <span class={format!("health-indicator {}", health_class)}></span>
                            {format!("Health: {} ({})",
                                if health_check.success { "Healthy" } else { "Unhealthy" },
                                response_time
                            )}
                        </span>
                    </div>
                    <div class="time-ago">{format!("Last check: {}", format_time_ago(&health_check.timestamp))}</div>
                </>
            }
        } else {
            html! {
                <div>
                    <span class="health-status">
                        <span class="health-indicator disabled"></span>
                        {"Health: Pending"}
                    </span>
                </div>
            }
        }
    } else {
        html! {
            <div>
                <span class="health-status">
                    <span class="health-indicator disabled"></span>
                    {"Health checks disabled"}
                </span>
            </div>
        }
    };

    // Format last access time
    let last_access = if let Some(access_time) = &server.last_access_time {
        html! {
            <div class="time-ago">{format!("Last accessed: {}", format_time_ago(access_time))}</div>
        }
    } else {
        html! {
            <div class="time-ago">{"Never accessed"}</div>
        }
    };

    let on_start = {
        let name = server.name.clone();
        let callback = props.on_action.clone();
        Callback::from(move |_| callback.emit((name.clone(), "start".to_string())))
    };

    let on_stop = {
        let name = server.name.clone();
        let callback = props.on_action.clone();
        Callback::from(move |_| callback.emit((name.clone(), "stop".to_string())))
    };

    let on_restart = {
        let name = server.name.clone();
        let callback = props.on_action.clone();
        Callback::from(move |_| callback.emit((name.clone(), "restart".to_string())))
    };

    let on_logs = {
        let name = server.name.clone();
        let callback = props.on_view_logs.clone();
        Callback::from(move |_| callback.emit(name.clone()))
    };

    let on_toggle_disable = {
        let name = server.name.clone();
        let callback = props.on_toggle_disable.clone();
        Callback::from(move |_| callback.emit(name.clone()))
    };

    let card_class = if server.disabled {
        "server-card disabled"
    } else {
        "server-card"
    };

    html! {
        <div class={card_class}>
            <div class="server-header">
                <div class="server-name-wrapper">
                    <input
                        type="checkbox"
                        class="disable-checkbox"
                        checked={server.disabled}
                        onclick={on_toggle_disable}
                        title={if server.disabled { "Enable server" } else { "Disable server" }}
                    />
                    <span class="server-name">{&server.name}</span>
                </div>
                <span class={format!("server-state {}", state_lower)}>
                    {format!("{:?}", server.state).to_uppercase()}
                </span>
            </div>
            <div class="server-info">
                <div>{format!("Restarts: {}", server.restart_count)}</div>
                {health_status}
                {last_access}
            </div>
            <div class="server-actions">
                <button class="btn btn-start" disabled={!is_stopped} onclick={on_start}>
                    {"Start"}
                </button>
                <button class="btn btn-stop" disabled={!is_running} onclick={on_stop}>
                    {"Stop"}
                </button>
                <button class="btn btn-restart" disabled={!is_running} onclick={on_restart}>
                    {"Restart"}
                </button>
                <button class="btn btn-logs" onclick={on_logs}>
                    {"Logs"}
                </button>
            </div>
        </div>
    }
}

fn format_time_ago(timestamp: &chrono::DateTime<Utc>) -> String {
    let now = Utc::now();
    let diff = now.signed_duration_since(*timestamp);

    let seconds = diff.num_seconds();
    let minutes = diff.num_minutes();
    let hours = diff.num_hours();
    let days = diff.num_days();

    if days > 0 {
        format!("{}d ago", days)
    } else if hours > 0 {
        format!("{}h ago", hours)
    } else if minutes > 0 {
        format!("{}m ago", minutes)
    } else {
        format!("{}s ago", seconds)
    }
}
