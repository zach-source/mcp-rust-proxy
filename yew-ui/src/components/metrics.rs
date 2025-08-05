use crate::types::Metric;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct MetricsProps {
    pub metrics: Vec<Metric>,
}

#[function_component(Metrics)]
pub fn metrics(props: &MetricsProps) -> Html {
    let key_metrics = vec![
        ("Total Requests", "mcp_proxy_requests_total"),
        ("Failed Requests", "mcp_proxy_requests_failed"),
        ("Active Connections", "mcp_proxy_active_connections"),
        ("Connection Errors", "mcp_proxy_connection_errors_total"),
        ("Health Checks", "mcp_proxy_health_checks_total"),
        ("Failed Health Checks", "mcp_proxy_health_checks_failed"),
    ];

    html! {
        <section class="metrics-section">
            <h2>{"System Metrics"}</h2>
            <div class="metrics-container">
                {key_metrics.iter().map(|(name, key)| {
                    let value = props.metrics.iter()
                        .find(|m| m.name == *key)
                        .and_then(|m| m.metrics.first())
                        .map(|v| v.value.round() as u64)
                        .unwrap_or(0);

                    html! {
                        <div key={*key} class="metric-card">
                            <div class="metric-name">{name}</div>
                            <div class="metric-value">{value}</div>
                        </div>
                    }
                }).collect::<Html>()}
            </div>
        </section>
    }
}