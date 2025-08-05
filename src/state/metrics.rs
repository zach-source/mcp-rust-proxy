use prometheus::{Histogram, IntCounter, IntGauge, Registry};
use std::sync::Arc;
use std::time::Instant;

pub struct Metrics {
    registry: Registry,

    // Server metrics
    pub total_servers: IntGauge,
    pub running_servers: IntGauge,
    pub failed_servers: IntGauge,

    // Request metrics
    pub total_requests: IntCounter,
    pub failed_requests: IntCounter,
    pub request_duration: Histogram,

    // Connection metrics
    pub active_connections: IntGauge,
    pub connection_errors: IntCounter,

    // Health check metrics
    pub health_checks_total: IntCounter,
    pub health_checks_failed: IntCounter,
}

impl Metrics {
    pub fn new() -> Self {
        let registry = Registry::new();

        let total_servers = IntGauge::new(
            "mcp_proxy_total_servers",
            "Total number of configured servers",
        )
        .expect("metric creation failed");
        let running_servers = IntGauge::new(
            "mcp_proxy_running_servers",
            "Number of currently running servers",
        )
        .expect("metric creation failed");
        let failed_servers = IntGauge::new("mcp_proxy_failed_servers", "Number of failed servers")
            .expect("metric creation failed");

        let total_requests =
            IntCounter::new("mcp_proxy_requests_total", "Total number of proxy requests")
                .expect("metric creation failed");
        let failed_requests = IntCounter::new(
            "mcp_proxy_requests_failed",
            "Total number of failed proxy requests",
        )
        .expect("metric creation failed");
        let request_duration = Histogram::with_opts(prometheus::HistogramOpts::new(
            "mcp_proxy_request_duration_seconds",
            "Request duration in seconds",
        ))
        .expect("metric creation failed");

        let active_connections = IntGauge::new(
            "mcp_proxy_active_connections",
            "Number of active connections",
        )
        .expect("metric creation failed");
        let connection_errors = IntCounter::new(
            "mcp_proxy_connection_errors_total",
            "Total number of connection errors",
        )
        .expect("metric creation failed");

        let health_checks_total = IntCounter::new(
            "mcp_proxy_health_checks_total",
            "Total number of health checks",
        )
        .expect("metric creation failed");
        let health_checks_failed = IntCounter::new(
            "mcp_proxy_health_checks_failed",
            "Total number of failed health checks",
        )
        .expect("metric creation failed");

        // Register all metrics
        registry.register(Box::new(total_servers.clone())).unwrap();
        registry
            .register(Box::new(running_servers.clone()))
            .unwrap();
        registry.register(Box::new(failed_servers.clone())).unwrap();
        registry.register(Box::new(total_requests.clone())).unwrap();
        registry
            .register(Box::new(failed_requests.clone()))
            .unwrap();
        registry
            .register(Box::new(request_duration.clone()))
            .unwrap();
        registry
            .register(Box::new(active_connections.clone()))
            .unwrap();
        registry
            .register(Box::new(connection_errors.clone()))
            .unwrap();
        registry
            .register(Box::new(health_checks_total.clone()))
            .unwrap();
        registry
            .register(Box::new(health_checks_failed.clone()))
            .unwrap();

        Self {
            registry,
            total_servers,
            running_servers,
            failed_servers,
            total_requests,
            failed_requests,
            request_duration,
            active_connections,
            connection_errors,
            health_checks_total,
            health_checks_failed,
        }
    }

    pub fn increment_server_count(&self) {
        self.total_servers.inc();
    }

    pub fn decrement_server_count(&self) {
        self.total_servers.dec();
    }

    pub fn increment_running_servers(&self) {
        self.running_servers.inc();
    }

    pub fn decrement_running_servers(&self) {
        self.running_servers.dec();
    }

    pub fn increment_failed_servers(&self) {
        self.failed_servers.inc();
    }

    pub fn record_request(&self) {
        self.total_requests.inc();
    }

    pub fn record_failed_request(&self) {
        self.failed_requests.inc();
    }

    pub fn record_request_duration(&self, duration: std::time::Duration) {
        self.request_duration.observe(duration.as_secs_f64());
    }

    pub fn increment_active_connections(&self) {
        self.active_connections.inc();
    }

    pub fn decrement_active_connections(&self) {
        self.active_connections.dec();
    }

    pub fn record_connection_error(&self) {
        self.connection_errors.inc();
    }

    pub fn record_health_check(&self, success: bool) {
        self.health_checks_total.inc();
        if !success {
            self.health_checks_failed.inc();
        }
    }

    pub fn gather_metrics(&self) -> Vec<prometheus::proto::MetricFamily> {
        self.registry.gather()
    }
}

pub struct RequestTimer {
    start: Instant,
    metrics: Arc<Metrics>,
}

impl RequestTimer {
    pub fn new(metrics: Arc<Metrics>) -> Self {
        metrics.record_request();
        Self {
            start: Instant::now(),
            metrics,
        }
    }

    pub fn finish(self) {
        let duration = self.start.elapsed();
        self.metrics.record_request_duration(duration);
    }

    pub fn fail(self) {
        let duration = self.start.elapsed();
        self.metrics.record_request_duration(duration);
        self.metrics.record_failed_request();
    }
}
