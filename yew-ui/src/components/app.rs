use crate::api::{self, log_stream::*, websocket::*};
use crate::components::{ConfigHelper, Header, LogsModal, Metrics, Modal, ServersList};
use crate::types::*;
use gloo_timers::callback::Interval;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

pub enum Msg {
    // WebSocket messages
    WsConnect,
    WsMessage(WsMessage),
    WsConnectionStatusChanged,

    // API messages
    FetchServers,
    ServersLoaded(Vec<Server>),
    FetchMetrics,
    MetricsLoaded(Vec<Metric>),

    // UI actions
    ShowActionModal(String, String), // server_name, action
    HideActionModal,
    ConfirmAction,
    ViewLogs(String),
    CloseLogs,
    ClearLogs,
    ShowConfigHelper,
    HideConfigHelper,

    // Log streaming
    LogStreamMessage(String),

    // Server action response
    ActionCompleted(Result<ApiResponse, String>),

    // Toggle server disabled state
    ToggleDisable(String),
}

pub struct App {
    servers: Vec<Server>,
    metrics: Vec<Metric>,
    stats: Option<Stats>,
    ws_service: WebSocketService,
    ws_status: WsStatus,
    _metrics_interval: Option<Interval>,
    _reconnect_interval: Option<Interval>,

    // Modal state
    show_action_modal: bool,
    pending_action: Option<(String, String)>,

    // Logs modal state
    show_logs_modal: bool,
    current_log_server: Option<String>,
    logs: Vec<LogData>,
    log_stream_service: LogStreamService,

    // Config helper state
    show_config_helper: bool,
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        // Connect WebSocket
        ctx.link().send_message(Msg::WsConnect);

        // Fetch initial data
        ctx.link().send_message(Msg::FetchServers);
        ctx.link().send_message(Msg::FetchMetrics);

        // Set up metrics refresh interval (every 5 seconds)
        let link = ctx.link().clone();
        let metrics_interval = Interval::new(5000, move || {
            link.send_message(Msg::FetchMetrics);
        });

        Self {
            servers: vec![],
            metrics: vec![],
            stats: None,
            ws_service: WebSocketService::new(),
            ws_status: WsStatus::Disconnected,
            _metrics_interval: Some(metrics_interval),
            _reconnect_interval: None,
            show_action_modal: false,
            pending_action: None,
            show_logs_modal: false,
            current_log_server: None,
            logs: vec![],
            log_stream_service: LogStreamService::new(),
            show_config_helper: false,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::WsConnect => {
                let link = ctx.link().clone();
                let callback = link.callback(Msg::WsMessage);

                if let Err(e) = self.ws_service.connect(callback) {
                    web_sys::console::error_1(&JsValue::from_str(&format!(
                        "WebSocket error: {:?}",
                        e
                    )));

                    // Set up reconnect interval if not already set
                    if self._reconnect_interval.is_none() {
                        let link = ctx.link().clone();
                        let interval = Interval::new(5000, move || {
                            link.send_message(Msg::WsConnect);
                        });
                        self._reconnect_interval = Some(interval);
                    }
                }

                ctx.link().send_message(Msg::WsConnectionStatusChanged);
                false
            }

            Msg::WsMessage(msg) => match msg {
                WsMessage::Initial { data } => {
                    self.servers = data.servers;
                    self.stats = data.stats;
                    // Update connection status when we receive initial data
                    ctx.link().send_message(Msg::WsConnectionStatusChanged);
                    true
                }
                WsMessage::Update { data } => {
                    // Smart update: only update servers that actually changed
                    let mut any_change = false;

                    // Update servers more intelligently
                    if self.servers.len() != data.servers.len() {
                        self.servers = data.servers;
                        any_change = true;
                    } else {
                        // Check each server individually
                        for (i, new_server) in data.servers.iter().enumerate() {
                            if let Some(existing) = self.servers.get(i) {
                                if existing != new_server {
                                    self.servers[i] = new_server.clone();
                                    any_change = true;
                                }
                            }
                        }
                    }

                    // Check stats
                    if self.stats != data.stats {
                        self.stats = data.stats;
                        any_change = true;
                    }

                    any_change
                }
                WsMessage::Log { data } => {
                    if Some(&data.server) == self.current_log_server.as_ref() {
                        self.logs.push(data);
                        true
                    } else {
                        false
                    }
                }
            },

            Msg::WsConnectionStatusChanged => {
                self.ws_status = self.ws_service.status();

                // Clear reconnect interval if connected
                if matches!(self.ws_status, WsStatus::Connected) {
                    self._reconnect_interval = None;
                }
                true
            }

            Msg::FetchServers => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match api::fetch_servers().await {
                        Ok(response) => link.send_message(Msg::ServersLoaded(response.servers)),
                        Err(e) => web_sys::console::error_1(&JsValue::from_str(&format!(
                            "Failed to fetch servers: {:?}",
                            e
                        ))),
                    }
                });
                false
            }

            Msg::ServersLoaded(servers) => {
                self.servers = servers;
                true
            }

            Msg::FetchMetrics => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match api::fetch_metrics().await {
                        Ok(response) => link.send_message(Msg::MetricsLoaded(response.metrics)),
                        Err(e) => web_sys::console::error_1(&JsValue::from_str(&format!(
                            "Failed to fetch metrics: {:?}",
                            e
                        ))),
                    }
                });
                false
            }

            Msg::MetricsLoaded(metrics) => {
                self.metrics = metrics;
                true
            }

            Msg::ShowActionModal(server_name, action) => {
                self.pending_action = Some((server_name, action));
                self.show_action_modal = true;
                true
            }

            Msg::HideActionModal => {
                self.show_action_modal = false;
                self.pending_action = None;
                true
            }

            Msg::ConfirmAction => {
                if let Some((server_name, action)) = self.pending_action.take() {
                    let link = ctx.link().clone();
                    spawn_local(async move {
                        let result = api::server_action(&server_name, &action)
                            .await
                            .map_err(|e| format!("{:?}", e));
                        link.send_message(Msg::ActionCompleted(result));
                    });
                }
                self.show_action_modal = false;
                true
            }

            Msg::ActionCompleted(result) => {
                match result {
                    Ok(response) => {
                        if let Some(error) = response.error {
                            web_sys::window()
                                .unwrap()
                                .alert_with_message(&format!("Error: {}", error))
                                .ok();
                        }
                    }
                    Err(e) => {
                        web_sys::window()
                            .unwrap()
                            .alert_with_message(&format!("Error: {}", e))
                            .ok();
                    }
                }
                false
            }

            Msg::ViewLogs(server_name) => {
                self.current_log_server = Some(server_name.clone());
                self.logs.clear();
                self.show_logs_modal = true;

                // Start SSE log streaming
                let link = ctx.link().clone();
                let callback = link.callback(Msg::LogStreamMessage);
                if let Err(e) = self
                    .log_stream_service
                    .start_streaming(&server_name, callback)
                {
                    web_sys::console::error_1(&e);
                }

                true
            }

            Msg::CloseLogs => {
                // Stop SSE log streaming
                self.log_stream_service.stop_streaming();
                self.current_log_server = None;
                self.show_logs_modal = false;
                self.logs.clear();
                true
            }

            Msg::ClearLogs => {
                self.logs.clear();
                true
            }

            Msg::LogStreamMessage(message) => {
                if let Some(ref server_name) = self.current_log_server {
                    if let Some(mut log_data) = parse_log_line(&message) {
                        log_data.server = server_name.clone();
                        self.logs.push(log_data);
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }

            Msg::ShowConfigHelper => {
                self.show_config_helper = true;
                true
            }

            Msg::HideConfigHelper => {
                self.show_config_helper = false;
                true
            }

            Msg::ToggleDisable(server_name) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match api::toggle_server_disable(&server_name).await {
                        Ok(_) => link.send_message(Msg::FetchServers),
                        Err(e) => web_sys::console::error_1(&JsValue::from_str(&format!(
                            "Failed to toggle disable state: {:?}",
                            e
                        ))),
                    }
                });
                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let on_action = ctx
            .link()
            .callback(|(server, action): (String, String)| Msg::ShowActionModal(server, action));

        let on_view_logs = ctx.link().callback(Msg::ViewLogs);

        let on_toggle_disable = ctx.link().callback(Msg::ToggleDisable);

        html! {
            <div class="container">
                <Header
                    stats={self.stats.clone()}
                    ws_status={self.ws_status.clone()}
                    on_config_click={ctx.link().callback(|_| Msg::ShowConfigHelper)}
                />

                <main>
                    <ServersList
                        servers={self.servers.clone()}
                        on_action={on_action}
                        on_view_logs={on_view_logs}
                        on_toggle_disable={on_toggle_disable}
                    />

                    <Metrics metrics={self.metrics.clone()} />
                </main>

                // Action confirmation modal
                <Modal show={self.show_action_modal} on_background_click={Some(ctx.link().callback(|_| Msg::HideActionModal))}>
                    <div class="modal-content">
                        <h3>{"Confirm Action"}</h3>
                        {if let Some((server, action)) = &self.pending_action {
                            html! {
                                <p>{format!("Are you sure you want to {} the server \"{}\"?", action, server)}</p>
                            }
                        } else {
                            html! {}
                        }}
                        <div class="modal-buttons">
                            <button class="btn btn-primary" onclick={ctx.link().callback(|_| Msg::ConfirmAction)}>
                                {"Confirm"}
                            </button>
                            <button class="btn btn-secondary" onclick={ctx.link().callback(|_| Msg::HideActionModal)}>
                                {"Cancel"}
                            </button>
                        </div>
                    </div>
                </Modal>

                // Logs modal
                <LogsModal
                    show={self.show_logs_modal}
                    server_name={self.current_log_server.as_ref().unwrap_or(&String::new()).clone()}
                    logs={self.logs.clone()}
                    on_close={ctx.link().callback(|_| Msg::CloseLogs)}
                    on_clear={ctx.link().callback(|_| Msg::ClearLogs)}
                />

                // Config helper modal
                <ConfigHelper
                    show={self.show_config_helper}
                    on_close={ctx.link().callback(|_| Msg::HideConfigHelper)}
                />
            </div>
        }
    }
}
