use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::{Duration, Instant};

/// Initialize request sent to MCP server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeRequest {
    pub jsonrpc: String, // Always "2.0"
    pub id: Value,
    pub method: String, // Always "initialize"
    pub params: InitializeParams,
}

/// Parameters for initialize request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeParams {
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    pub capabilities: ClientCapabilities,
    #[serde(rename = "clientInfo")]
    pub client_info: Implementation,
}

/// Client capabilities
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ClientCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roots: Option<RootsCapability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sampling: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental: Option<Value>,
}

/// Roots capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootsCapability {
    #[serde(rename = "listChanged", skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

/// Initialize response from MCP server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeResponse {
    pub jsonrpc: String,
    pub id: Value,
    pub result: InitializeResult,
}

/// Result of initialize response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeResult {
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    pub capabilities: ServerCapabilities,
    #[serde(rename = "serverInfo")]
    pub server_info: Implementation,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
}

/// Server capabilities
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServerCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<ToolsCapability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<ResourcesCapability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts: Option<PromptsCapability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logging: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental: Option<Value>,
}

/// Tools capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsCapability {
    #[serde(rename = "listChanged", skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

/// Resources capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcesCapability {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscribe: Option<bool>,
    #[serde(rename = "listChanged", skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

/// Prompts capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptsCapability {
    #[serde(rename = "listChanged", skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

/// Implementation info (client or server)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Implementation {
    pub name: String,
    pub version: String,
}

/// Initialized notification sent after receiving initialize response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializedNotification {
    pub jsonrpc: String, // Always "2.0"
    pub method: String,  // Always "notifications/initialized"
}

impl InitializedNotification {
    pub fn new() -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method: "notifications/initialized".to_string(),
        }
    }
}

impl Default for InitializedNotification {
    fn default() -> Self {
        Self::new()
    }
}

/// Tracks timing and status of initialization handshake
#[derive(Debug, Clone)]
pub struct InitializationHandshakeTracker {
    pub started_at: Instant,
    pub initialize_sent_at: Option<Instant>,
    pub initialize_received_at: Option<Instant>,
    pub initialized_sent_at: Option<Instant>,
    pub completed_at: Option<Instant>,
    pub timeout: Duration,
}

impl InitializationHandshakeTracker {
    pub fn new(timeout: Duration) -> Self {
        Self {
            started_at: Instant::now(),
            initialize_sent_at: None,
            initialize_received_at: None,
            initialized_sent_at: None,
            completed_at: None,
            timeout,
        }
    }

    pub fn mark_initialize_sent(&mut self) {
        self.initialize_sent_at = Some(Instant::now());
    }

    pub fn mark_initialize_received(&mut self) {
        self.initialize_received_at = Some(Instant::now());
    }

    pub fn mark_initialized_sent(&mut self) {
        self.initialized_sent_at = Some(Instant::now());
    }

    pub fn mark_completed(&mut self) {
        self.completed_at = Some(Instant::now());
    }

    pub fn is_timed_out(&self) -> bool {
        self.started_at.elapsed() > self.timeout
    }

    pub fn total_duration(&self) -> Option<Duration> {
        self.completed_at
            .map(|end| end.duration_since(self.started_at))
    }

    pub fn phase_durations(&self) -> InitializationPhaseTimings {
        InitializationPhaseTimings {
            send_initialize: self
                .initialize_sent_at
                .map(|t| t.duration_since(self.started_at)),
            wait_for_response: self.initialize_received_at.and_then(|recv| {
                self.initialize_sent_at
                    .map(|sent| recv.duration_since(sent))
            }),
            send_initialized: self.initialized_sent_at.and_then(|sent| {
                self.initialize_received_at
                    .map(|recv| sent.duration_since(recv))
            }),
            mark_ready: self.completed_at.and_then(|comp| {
                self.initialized_sent_at
                    .map(|sent| comp.duration_since(sent))
            }),
        }
    }
}

#[derive(Debug, Clone)]
pub struct InitializationPhaseTimings {
    pub send_initialize: Option<Duration>,
    pub wait_for_response: Option<Duration>,
    pub send_initialized: Option<Duration>,
    pub mark_ready: Option<Duration>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_initialized_notification_serialization() {
        let notification = InitializedNotification::new();
        let json = serde_json::to_value(&notification).unwrap();

        assert_eq!(json["jsonrpc"], "2.0");
        assert_eq!(json["method"], "notifications/initialized");
    }

    #[test]
    fn test_initialize_response_deserialization() {
        let json = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "protocolVersion": "2025-03-26",
                "capabilities": {
                    "tools": {},
                    "resources": {}
                },
                "serverInfo": {
                    "name": "test-server",
                    "version": "1.0.0"
                }
            }
        });

        let response: InitializeResponse = serde_json::from_value(json).unwrap();
        assert_eq!(response.result.protocol_version, "2025-03-26");
        assert_eq!(response.result.server_info.name, "test-server");
    }

    #[test]
    fn test_handshake_tracker_timing() {
        let mut tracker = InitializationHandshakeTracker::new(Duration::from_secs(60));

        tracker.mark_initialize_sent();
        std::thread::sleep(Duration::from_millis(10));
        tracker.mark_initialize_received();
        tracker.mark_initialized_sent();
        tracker.mark_completed();

        let timings = tracker.phase_durations();
        assert!(timings.send_initialize.is_some());
        assert!(timings.wait_for_response.is_some());
        assert!(tracker.total_duration().is_some());
    }

    #[test]
    fn test_handshake_tracker_timeout() {
        let tracker = InitializationHandshakeTracker::new(Duration::from_millis(1));
        std::thread::sleep(Duration::from_millis(5));

        assert!(tracker.is_timed_out());
    }
}
