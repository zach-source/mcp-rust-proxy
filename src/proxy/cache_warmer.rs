use crate::error::Result;
use crate::state::AppState;
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;
use tracing::{debug, info, warn};

/// Background cache warmer that pre-fetches and maintains tool/resource/prompt lists
/// This ensures instant availability when remote MCP clients connect
pub struct CacheWarmer {
    state: Arc<AppState>,
    handler: Arc<super::RequestHandler>,
    refresh_interval: Duration,
}

impl CacheWarmer {
    pub fn new(
        state: Arc<AppState>,
        handler: Arc<super::RequestHandler>,
        refresh_interval_secs: u64,
    ) -> Self {
        Self {
            state,
            handler,
            refresh_interval: Duration::from_secs(refresh_interval_secs),
        }
    }

    /// Start the cache warmer background task
    pub async fn run(self) {
        info!(
            "Starting cache warmer (refresh every {}s)",
            self.refresh_interval.as_secs()
        );

        // Wait for servers to initialize before first warm
        // This ensures we capture all servers in the initial cache
        info!("Waiting 5s for servers to initialize before first cache warm");
        tokio::time::sleep(Duration::from_secs(5)).await;

        // Warm cache after servers have had time to start
        self.warm_cache_once().await;

        // Then run periodic refresh
        let mut ticker = interval(self.refresh_interval);
        loop {
            ticker.tick().await;
            debug!("Cache warmer tick - refreshing caches");
            self.warm_cache_once().await;
        }
    }

    /// Perform a single cache warming cycle
    async fn warm_cache_once(&self) {
        // Only warm cache if we have running servers
        let running_count = self
            .state
            .servers
            .iter()
            .filter(|entry| {
                let state = entry.value().state.try_read();
                matches!(
                    state,
                    Ok(guard) if matches!(*guard, crate::state::ServerState::Running)
                )
            })
            .count();

        if running_count == 0 {
            debug!("No running servers, skipping cache warm");
            return;
        }

        info!("Warming cache for {} running servers", running_count);

        // Warm tools cache
        if let Err(e) = self.warm_tools_cache().await {
            warn!("Failed to warm tools cache: {}", e);
        }

        // Warm resources cache
        if let Err(e) = self.warm_resources_cache().await {
            warn!("Failed to warm resources cache: {}", e);
        }

        // Warm prompts cache
        if let Err(e) = self.warm_prompts_cache().await {
            warn!("Failed to warm prompts cache: {}", e);
        }

        // Warm capabilities cache
        if let Err(e) = self.warm_capabilities_cache().await {
            warn!("Failed to warm capabilities cache: {}", e);
        }

        info!("Cache warming complete");
    }

    /// Warm tools/list cache
    async fn warm_tools_cache(&self) -> Result<()> {
        debug!("Warming tools cache...");

        // Create a dummy request for tools/list
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": "cache_warmer",
            "method": "tools/list"
        });

        let router = Arc::new(super::RequestRouter::new());
        match self.handler.handle_request(request, router).await {
            Ok(response) => {
                if let Some(result) = response.result {
                    if let Some(tools) = result.get("tools").and_then(|t| t.as_array()) {
                        debug!("Tools cache warmed with {} tools", tools.len());
                    }
                }
                Ok(())
            }
            Err(e) => {
                warn!("Failed to warm tools cache: {}", e);
                Err(e)
            }
        }
    }

    /// Warm resources/list cache
    async fn warm_resources_cache(&self) -> Result<()> {
        debug!("Warming resources cache...");

        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": "cache_warmer",
            "method": "resources/list"
        });

        let router = Arc::new(super::RequestRouter::new());
        match self.handler.handle_request(request, router).await {
            Ok(response) => {
                if let Some(result) = response.result {
                    if let Some(resources) = result.get("resources").and_then(|r| r.as_array()) {
                        debug!("Resources cache warmed with {} resources", resources.len());
                    }
                }
                Ok(())
            }
            Err(e) => {
                warn!("Failed to warm resources cache: {}", e);
                Err(e)
            }
        }
    }

    /// Warm prompts/list cache
    async fn warm_prompts_cache(&self) -> Result<()> {
        debug!("Warming prompts cache...");

        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": "cache_warmer",
            "method": "prompts/list"
        });

        let router = Arc::new(super::RequestRouter::new());
        match self.handler.handle_request(request, router).await {
            Ok(response) => {
                if let Some(result) = response.result {
                    if let Some(prompts) = result.get("prompts").and_then(|p| p.as_array()) {
                        debug!("Prompts cache warmed with {} prompts", prompts.len());
                    }
                }
                Ok(())
            }
            Err(e) => {
                warn!("Failed to warm prompts cache: {}", e);
                Err(e)
            }
        }
    }

    /// Warm server capabilities cache
    async fn warm_capabilities_cache(&self) -> Result<()> {
        debug!("Warming server capabilities cache...");

        // Store capabilities for each server during initialization
        // For now, we'll fetch from each server individually
        let server_names: Vec<String> = self
            .state
            .servers
            .iter()
            .map(|entry| entry.key().clone())
            .collect();

        for server_name in server_names {
            // Check if server is running
            if let Some(info) = self.state.servers.get(&server_name) {
                let state = info.state.read().await;
                if matches!(*state, crate::state::ServerState::Running) {
                    debug!("Caching capabilities for server: {}", server_name);
                    // TODO: Store capabilities from server's initialize response
                    // For now, capabilities are stored during connection initialization
                }
            }
        }

        Ok(())
    }

    /// Invalidate all caches (call when server state changes)
    pub async fn invalidate_caches(&self) {
        info!("Invalidating all caches due to server state change");
        self.handler.clear_cache().await;
        // Immediately warm cache again
        tokio::spawn({
            let warmer = Self {
                state: self.state.clone(),
                handler: self.handler.clone(),
                refresh_interval: self.refresh_interval,
            };
            async move {
                warmer.warm_cache_once().await;
            }
        });
    }
}

/// Shared cache warmer instance that can be triggered from anywhere
pub struct CacheWarmerHandle {
    state: Arc<AppState>,
    handler: Arc<super::RequestHandler>,
}

impl CacheWarmerHandle {
    pub fn new(state: Arc<AppState>, handler: Arc<super::RequestHandler>) -> Self {
        Self { state, handler }
    }

    /// Trigger immediate cache refresh
    pub async fn refresh_now(&self) {
        let warmer = CacheWarmer::new(self.state.clone(), self.handler.clone(), 60);
        warmer.warm_cache_once().await;
    }

    /// Invalidate caches when server state changes
    pub async fn on_server_state_change(&self) {
        info!("Server state changed, invalidating caches");
        self.handler.clear_cache().await;
        self.refresh_now().await;
    }
}
