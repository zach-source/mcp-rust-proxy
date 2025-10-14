use dashmap::DashMap;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Queued request waiting for server initialization
#[derive(Debug, Clone)]
pub struct QueuedRequest {
    pub request_id: String,
    pub method: String,
    pub params: Option<Value>,
    pub response_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<Result<Value, String>>>>>,
}

pub struct RequestRouter {
    // Maps resource/tool/prompt names to server names
    pub tool_to_server: DashMap<String, String>,
    pub resource_to_server: DashMap<String, String>,
    pub prompt_to_server: DashMap<String, String>,

    // Request queues per server (for requests during initialization)
    pub request_queues: DashMap<String, Arc<Mutex<Vec<QueuedRequest>>>>,
}

impl RequestRouter {
    pub fn new() -> Self {
        Self {
            tool_to_server: DashMap::new(),
            resource_to_server: DashMap::new(),
            prompt_to_server: DashMap::new(),
            request_queues: DashMap::new(),
        }
    }

    pub fn register_tool(&self, tool_name: String, server_name: String) {
        self.tool_to_server.insert(tool_name, server_name);
    }

    pub fn register_resource(&self, resource_uri: String, server_name: String) {
        self.resource_to_server.insert(resource_uri, server_name);
    }

    pub fn register_prompt(&self, prompt_name: String, server_name: String) {
        self.prompt_to_server.insert(prompt_name, server_name);
    }

    pub fn get_server_for_tool(&self, tool_name: &str) -> Option<String> {
        self.tool_to_server.get(tool_name).map(|v| v.clone())
    }

    pub fn get_server_for_resource(&self, resource_uri: &str) -> Option<String> {
        self.resource_to_server.get(resource_uri).map(|v| v.clone())
    }

    pub fn get_server_for_prompt(&self, prompt_name: &str) -> Option<String> {
        self.prompt_to_server.get(prompt_name).map(|v| v.clone())
    }

    pub fn unregister_server(&self, server_name: &str) {
        // Remove all entries for this server
        self.tool_to_server.retain(|_, v| v != server_name);
        self.resource_to_server.retain(|_, v| v != server_name);
        self.prompt_to_server.retain(|_, v| v != server_name);
    }

    pub fn clear(&self) {
        self.tool_to_server.clear();
        self.resource_to_server.clear();
        self.prompt_to_server.clear();
    }

    // ========================================================================
    // T027: Request Queuing During Initialization
    // ========================================================================

    /// Queue a request for a server that's not yet ready
    pub async fn queue_request(&self, server_name: &str, request: QueuedRequest) {
        let queue = self
            .request_queues
            .entry(server_name.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(Vec::new())))
            .clone();

        let mut queue_guard = queue.lock().await;
        queue_guard.push(request);
    }

    /// Process all queued requests for a server (called when server becomes Ready)
    pub async fn process_queued_requests(&self, server_name: &str) -> Vec<QueuedRequest> {
        if let Some((_key, queue)) = self.request_queues.remove(server_name) {
            let mut queue_guard = queue.lock().await;
            std::mem::take(&mut *queue_guard)
        } else {
            Vec::new()
        }
    }

    /// Get the number of queued requests for a server
    pub async fn queued_request_count(&self, server_name: &str) -> usize {
        if let Some(queue) = self.request_queues.get(server_name) {
            let queue_guard = queue.lock().await;
            queue_guard.len()
        } else {
            0
        }
    }

    /// Clear all queued requests for a server (e.g., on failure/timeout)
    pub async fn clear_queued_requests(&self, server_name: &str) -> Vec<QueuedRequest> {
        self.process_queued_requests(server_name).await
    }
}
