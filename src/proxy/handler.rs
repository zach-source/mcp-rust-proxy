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
            _ => {
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
        
        conn.send(bytes::Bytes::from(request.to_string())).await?;
        
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
        
        conn.send(bytes::Bytes::from(request.to_string())).await?;
        
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
}