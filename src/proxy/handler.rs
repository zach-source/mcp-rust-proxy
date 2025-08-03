use std::sync::Arc;
use serde_json::Value;
use crate::error::{Result, ProxyError};
use crate::state::AppState;
use super::{MCPResponse, MCPError, CallParams, ReadParams, RequestRouter};

pub struct RequestHandler {
    state: Arc<AppState>,
}

impl RequestHandler {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }

    pub async fn handle_request(
        &self,
        request: Value,
        router: Arc<RequestRouter>,
    ) -> Result<MCPResponse> {
        // Extract request ID
        let id = request.get("id").cloned();
        
        // Parse method
        let method = request.get("method")
            .and_then(|m| m.as_str())
            .ok_or_else(|| ProxyError::InvalidRequest("Missing method".to_string()))?;
        
        // Handle based on method
        let result = match method {
            "list" => {
                let params = request.get("params")
                    .ok_or_else(|| ProxyError::InvalidRequest("Missing params".to_string()))?;
                self.handle_list(params, router).await?
            }
            "call" => {
                let params: CallParams = serde_json::from_value(
                    request.get("params").cloned()
                        .ok_or_else(|| ProxyError::InvalidRequest("Missing params".to_string()))?
                ).map_err(|e| ProxyError::InvalidRequest(e.to_string()))?;
                self.handle_call(params, router).await?
            }
            "read" => {
                let params: ReadParams = serde_json::from_value(
                    request.get("params").cloned()
                        .ok_or_else(|| ProxyError::InvalidRequest("Missing params".to_string()))?
                ).map_err(|e| ProxyError::InvalidRequest(e.to_string()))?;
                self.handle_read(params, router).await?
            }
            "ping" => {
                // Handle ping locally
                serde_json::json!({})
            }
            _ => {
                // For unrecognized methods, try to forward to all servers
                // This handles methods like "tools/list", "resources/list", etc.
                match self.forward_to_all_servers(method, request.get("params")).await {
                    Ok(result) => result,
                    Err(_) => {
                        return Ok(MCPResponse {
                            jsonrpc: "2.0".to_string(),
                            id,
                            result: None,
                            error: Some(MCPError {
                                code: -32601,
                                message: format!("Method not found: {}", method),
                                data: None,
                            }),
                        });
                    }
                }
            }
        };
        
        Ok(MCPResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        })
    }

    async fn handle_list(&self, params: &Value, router: Arc<RequestRouter>) -> Result<Value> {
        let list_type = params.get("type")
            .and_then(|t| t.as_str())
            .ok_or_else(|| ProxyError::InvalidRequest("Missing list type".to_string()))?;
        
        match list_type {
            "tools" => self.list_tools(router).await,
            "resources" => self.list_resources(router).await,
            "prompts" => self.list_prompts(router).await,
            _ => Err(ProxyError::InvalidRequest(format!("Unknown list type: {}", list_type))),
        }
    }

    async fn handle_call(&self, params: CallParams, router: Arc<RequestRouter>) -> Result<Value> {
        // Find server that handles this tool
        let server_name = router.get_server_for_tool(&params.tool)
            .ok_or_else(|| ProxyError::ServerNotFound(format!("No server handles tool: {}", params.tool)))?;
        
        // Get connection from pool
        let conn = self.state.connection_pool.get(&server_name).await?;
        
        // Forward request to server
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "call",
            "params": {
                "tool": params.tool,
                "arguments": params.arguments,
            },
            "id": 1
        });
        
        conn.send(bytes::Bytes::from(format!("{}\n", request.to_string()))).await?;
        
        // Get response
        let response = conn.recv().await?;
        let response: Value = serde_json::from_slice(&response)?;
        
        // Extract result
        response.get("result").cloned()
            .ok_or_else(|| ProxyError::InvalidRequest("No result in response".to_string()))
    }

    async fn handle_read(&self, params: ReadParams, router: Arc<RequestRouter>) -> Result<Value> {
        // Find server that handles this resource
        let server_name = router.get_server_for_resource(&params.uri)
            .ok_or_else(|| ProxyError::ServerNotFound(format!("No server handles resource: {}", params.uri)))?;
        
        // Get connection from pool
        let conn = self.state.connection_pool.get(&server_name).await?;
        
        // Forward request to server
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "read",
            "params": {
                "uri": params.uri,
            },
            "id": 1
        });
        
        conn.send(bytes::Bytes::from(format!("{}\n", request.to_string()))).await?;
        
        // Get response
        let response = conn.recv().await?;
        let response: Value = serde_json::from_slice(&response)?;
        
        // Extract result
        response.get("result").cloned()
            .ok_or_else(|| ProxyError::InvalidRequest("No result in response".to_string()))
    }

    async fn list_tools(&self, _router: Arc<RequestRouter>) -> Result<Value> {
        // TODO: Aggregate tools from all servers
        Ok(serde_json::json!({
            "tools": []
        }))
    }

    async fn list_resources(&self, _router: Arc<RequestRouter>) -> Result<Value> {
        // TODO: Aggregate resources from all servers
        Ok(serde_json::json!({
            "resources": []
        }))
    }

    async fn list_prompts(&self, _router: Arc<RequestRouter>) -> Result<Value> {
        // TODO: Aggregate prompts from all servers
        Ok(serde_json::json!({
            "prompts": []
        }))
    }

    async fn forward_to_all_servers(&self, method: &str, params: Option<&Value>) -> Result<Value> {
        // For methods like "tools/list", we need to aggregate results from all servers
        let mut aggregated_results = Vec::new();
        
        for entry in self.state.servers.iter() {
            let server_name = entry.key();
            match self.forward_to_server(server_name, method, params).await {
                Ok(result) => aggregated_results.push(result),
                Err(e) => {
                    tracing::debug!("Server {} failed to handle {}: {}", server_name, method, e);
                }
            }
        }
        
        if aggregated_results.is_empty() {
            return Err(ProxyError::InvalidRequest("No servers could handle the request".to_string()));
        }
        
        // Aggregate results based on method type
        match method {
            "tools/list" => {
                let mut all_tools = Vec::new();
                for result in aggregated_results {
                    if let Some(tools) = result.get("tools").and_then(|t| t.as_array()) {
                        all_tools.extend(tools.iter().cloned());
                    }
                }
                Ok(serde_json::json!({ "tools": all_tools }))
            }
            "resources/list" => {
                let mut all_resources = Vec::new();
                for result in aggregated_results {
                    if let Some(resources) = result.get("resources").and_then(|r| r.as_array()) {
                        all_resources.extend(resources.iter().cloned());
                    }
                }
                Ok(serde_json::json!({ "resources": all_resources }))
            }
            "prompts/list" => {
                let mut all_prompts = Vec::new();
                for result in aggregated_results {
                    if let Some(prompts) = result.get("prompts").and_then(|p| p.as_array()) {
                        all_prompts.extend(prompts.iter().cloned());
                    }
                }
                Ok(serde_json::json!({ "prompts": all_prompts }))
            }
            _ => {
                // For other methods, just return the first successful result
                Ok(aggregated_results.into_iter().next().unwrap())
            }
        }
    }
    
    async fn forward_to_server(&self, server_name: &str, method: &str, params: Option<&Value>) -> Result<Value> {
        let conn = self.state.connection_pool.get(server_name).await?;
        
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params.cloned().unwrap_or(serde_json::json!({})),
            "id": 1
        });
        
        let request_bytes = bytes::Bytes::from(format!("{}\n", request.to_string()));
        conn.send(request_bytes).await?;
        
        let response_bytes = conn.recv().await?;
        let response: Value = serde_json::from_slice(&response_bytes)?;
        
        // Check for error
        if let Some(error) = response.get("error") {
            return Err(ProxyError::InvalidRequest(
                format!("Server error: {}", error.get("message").and_then(|m| m.as_str()).unwrap_or("Unknown"))
            ));
        }
        
        response.get("result").cloned()
            .ok_or_else(|| ProxyError::InvalidRequest("No result in response".to_string()))
    }
}