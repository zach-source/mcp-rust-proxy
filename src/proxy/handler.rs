use super::{CallParams, MCPError, MCPResponse, ReadParams, RequestRouter};
use crate::error::{ProxyError, Result};
use crate::state::AppState;
use serde_json::Value;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

struct CachedResponse {
    value: Value,
    expires_at: Instant,
}

#[derive(Clone)]
pub struct RequestHandler {
    state: Arc<AppState>,
    tools_list_cache: Arc<RwLock<Option<CachedResponse>>>,
}

impl RequestHandler {
    pub fn new(state: Arc<AppState>) -> Self {
        Self {
            state,
            tools_list_cache: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn handle_request(
        &self,
        request: Value,
        router: Arc<RequestRouter>,
    ) -> Result<MCPResponse> {
        // Extract request ID
        let id = request.get("id").cloned();

        // Parse method
        let method = request
            .get("method")
            .and_then(|m| m.as_str())
            .ok_or_else(|| ProxyError::InvalidRequest("Missing method".to_string()))?;

        // Start context tracking if enabled (for tool calls and reads)
        let tracking_response_id =
            if matches!(method, "call" | "tools/call" | "read" | "resources/read") {
                if let Some(tracker) = &*self.state.context_tracker.read().await {
                    match tracker
                        .start_response("mcp-proxy".to_string(), "client".to_string())
                        .await
                    {
                        Ok(resp_id) => {
                            tracing::debug!("Started tracking response: {}", resp_id);
                            Some(resp_id)
                        }
                        Err(e) => {
                            tracing::warn!("Failed to start response tracking: {}", e);
                            None
                        }
                    }
                } else {
                    None
                }
            } else {
                None
            };

        // Handle based on method
        let result = match method {
            "initialize" => {
                // Return MCP server capabilities
                let _config = self.state.config.read().await;
                serde_json::json!({
                    "protocolVersion": "2025-03-26",
                    "capabilities": {
                        "tools": { "listChanged": false },
                        "resources": { "subscribe": false, "listChanged": false },
                        "prompts": { "listChanged": false }
                    },
                    "serverInfo": {
                        "name": "mcp-rust-proxy",
                        "version": env!("CARGO_PKG_VERSION")
                    }
                })
            }
            "list" => {
                let params = request
                    .get("params")
                    .ok_or_else(|| ProxyError::InvalidRequest("Missing params".to_string()))?;
                self.handle_list(params, router).await?
            }
            "call" | "tools/call" => {
                // Support both "call" and "tools/call" methods
                let params = request
                    .get("params")
                    .cloned()
                    .ok_or_else(|| ProxyError::InvalidRequest("Missing params".to_string()))?;

                // MCP spec uses "name" field, our CallParams uses "tool"
                let tool_name = params
                    .get("name")
                    .or_else(|| params.get("tool"))
                    .and_then(|n| n.as_str())
                    .ok_or_else(|| ProxyError::InvalidRequest("Missing tool name".to_string()))?
                    .to_string();

                let arguments = params
                    .get("arguments")
                    .cloned()
                    .unwrap_or(serde_json::json!({}));

                // Check if this is a proxy management tool
                if tool_name.starts_with("mcp__proxy__tracing__") {
                    let tracing_tool = tool_name.strip_prefix("mcp__proxy__tracing__").unwrap();
                    match super::tracing_tools::handle_tracing_tool(
                        tracing_tool,
                        arguments,
                        self.state.clone(),
                    )
                    .await
                    {
                        Ok(result) => result,
                        Err(e) => {
                            return Ok(MCPResponse {
                                jsonrpc: "2.0".to_string(),
                                id,
                                result: None,
                                error: Some(MCPError {
                                    code: -32603,
                                    message: e,
                                    data: None,
                                }),
                            });
                        }
                    }
                } else if tool_name.starts_with("mcp__proxy__server__") {
                    let server_tool = tool_name.strip_prefix("mcp__proxy__server__").unwrap();
                    match super::server_tools::handle_server_tool(
                        server_tool,
                        arguments,
                        self.state.clone(),
                    )
                    .await
                    {
                        Ok(result) => result,
                        Err(e) => {
                            return Ok(MCPResponse {
                                jsonrpc: "2.0".to_string(),
                                id,
                                result: None,
                                error: Some(MCPError {
                                    code: -32603,
                                    message: e,
                                    data: None,
                                }),
                            });
                        }
                    }
                } else if tool_name.starts_with("mcp__proxy__aggregator__") {
                    let aggregator_tool =
                        tool_name.strip_prefix("mcp__proxy__aggregator__").unwrap();
                    match super::aggregator_tools::handle_aggregator_tool(
                        aggregator_tool,
                        arguments,
                        self.state.clone(),
                    )
                    .await
                    {
                        Ok(result) => result,
                        Err(e) => {
                            return Ok(MCPResponse {
                                jsonrpc: "2.0".to_string(),
                                id,
                                result: None,
                                error: Some(MCPError {
                                    code: -32603,
                                    message: e,
                                    data: None,
                                }),
                            });
                        }
                    }
                } else {
                    let call_params = CallParams {
                        tool: tool_name,
                        arguments,
                    };
                    self.handle_call_with_tracking(call_params, router, &tracking_response_id)
                        .await?
                }
            }
            "read" | "resources/read" => {
                let params: ReadParams = serde_json::from_value(
                    request
                        .get("params")
                        .cloned()
                        .ok_or_else(|| ProxyError::InvalidRequest("Missing params".to_string()))?,
                )
                .map_err(|e| ProxyError::InvalidRequest(e.to_string()))?;

                // Check if this is a tracing resource
                if params.uri.starts_with("trace://") {
                    match super::tracing_tools::handle_tracing_resource(
                        &params.uri,
                        self.state.clone(),
                    )
                    .await
                    {
                        Ok(result) => result,
                        Err(e) => {
                            return Ok(MCPResponse {
                                jsonrpc: "2.0".to_string(),
                                id,
                                result: None,
                                error: Some(MCPError {
                                    code: -32603,
                                    message: e,
                                    data: None,
                                }),
                            });
                        }
                    }
                } else if params.uri.starts_with("proxy://") {
                    // Handle proxy-native resources
                    match super::resources::handle_proxy_resource(&params.uri, self.state.clone())
                        .await
                    {
                        Ok(result) => result,
                        Err(e) => {
                            return Ok(MCPResponse {
                                jsonrpc: "2.0".to_string(),
                                id,
                                result: None,
                                error: Some(MCPError {
                                    code: -32603,
                                    message: format!("Failed to read proxy resource: {e}"),
                                    data: None,
                                }),
                            });
                        }
                    }
                } else {
                    self.handle_read(params, router).await?
                }
            }
            "ping" => {
                // Handle ping locally
                serde_json::json!({})
            }
            "tools/list" => {
                // Check cache first
                let cache = self.tools_list_cache.read().await;
                if let Some(cached) = cache.as_ref() {
                    if cached.expires_at > Instant::now() {
                        tracing::debug!("Returning cached tools/list response");
                        return Ok(MCPResponse {
                            jsonrpc: "2.0".to_string(),
                            id,
                            result: Some(cached.value.clone()),
                            error: None,
                        });
                    }
                }
                drop(cache);

                // Cache miss or expired, fetch fresh data
                tracing::debug!("Cache miss for tools/list, fetching from servers");
                match self
                    .forward_to_all_servers(method, request.get("params"))
                    .await
                {
                    Ok(mut result) => {
                        // Always add proxy management tools
                        if let Some(tools_array) =
                            result.get_mut("tools").and_then(|t| t.as_array_mut())
                        {
                            let tracing_tools = super::tracing_tools::get_tracing_tools();
                            let server_tools = super::server_tools::get_server_tools();
                            let aggregator_tools = super::aggregator_tools::get_aggregator_tools();
                            tools_array.extend(tracing_tools);
                            tools_array.extend(server_tools);
                            tools_array.extend(aggregator_tools);
                        }

                        // Update cache
                        let mut cache = self.tools_list_cache.write().await;
                        *cache = Some(CachedResponse {
                            value: result.clone(),
                            expires_at: Instant::now() + Duration::from_secs(120), // 2 minutes
                        });
                        result
                    }
                    Err(e) => {
                        tracing::warn!("Failed to fetch tools/list: {}", e);
                        return Ok(MCPResponse {
                            jsonrpc: "2.0".to_string(),
                            id,
                            result: None,
                            error: Some(MCPError {
                                code: -32603,
                                message: "Failed to fetch tools list".to_string(),
                                data: None,
                            }),
                        });
                    }
                }
            }
            "resources/list" => {
                // Use list_resources() to ensure proxy resources are included
                self.list_resources(router).await?
            }
            "prompts/list" => {
                // Use list_prompts() to ensure proxy prompts are included
                self.list_prompts(router).await?
            }
            "resources/templates/list" => {
                // Forward to backend servers for resource templates
                match self
                    .forward_to_all_servers(method, request.get("params"))
                    .await
                {
                    Ok(result) => result,
                    Err(_) => serde_json::json!({ "resourceTemplates": [] }),
                }
            }
            "prompts/get" => {
                // Check if this is a proxy-native prompt first
                let params = request.get("params");
                let prompt_name = params
                    .and_then(|p| p.get("name"))
                    .and_then(|n| n.as_str())
                    .ok_or_else(|| ProxyError::InvalidRequest("Missing prompt name".to_string()))?;

                // Try to get from proxy prompts
                if let Some(prompt) = super::prompts::get_prompt(
                    prompt_name,
                    params.and_then(|p| p.get("arguments")).cloned(),
                ) {
                    prompt
                } else {
                    // Forward to backend servers
                    match self
                        .forward_to_all_servers(method, request.get("params"))
                        .await
                    {
                        Ok(result) => result,
                        Err(_e) => {
                            return Ok(MCPResponse {
                                jsonrpc: "2.0".to_string(),
                                id,
                                result: None,
                                error: Some(MCPError {
                                    code: -32601,
                                    message: format!("Prompt not found: {prompt_name}"),
                                    data: None,
                                }),
                            });
                        }
                    }
                }
            }
            _ => {
                // For other unrecognized methods, try to forward to all servers
                match self
                    .forward_to_all_servers(method, request.get("params"))
                    .await
                {
                    Ok(result) => result,
                    Err(_) => {
                        return Ok(MCPResponse {
                            jsonrpc: "2.0".to_string(),
                            id,
                            result: None,
                            error: Some(MCPError {
                                code: -32601,
                                message: format!("Method not found: {method}"),
                                data: None,
                            }),
                        });
                    }
                }
            }
        };

        // Finalize context tracking if we were tracking this request
        if let Some(resp_id) = tracking_response_id {
            if let Some(tracker) = &*self.state.context_tracker.read().await {
                match tracker.finalize_response(resp_id.clone(), None).await {
                    Ok(manifest) => {
                        tracing::info!(
                            "Response {} tracked: {} contexts, manifest generated",
                            resp_id,
                            manifest.context_tree.len()
                        );
                    }
                    Err(e) => {
                        tracing::warn!("Failed to finalize response tracking: {}", e);
                    }
                }
            }
        }

        Ok(MCPResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        })
    }

    async fn handle_list(&self, params: &Value, router: Arc<RequestRouter>) -> Result<Value> {
        let list_type = params
            .get("type")
            .and_then(|t| t.as_str())
            .ok_or_else(|| ProxyError::InvalidRequest("Missing list type".to_string()))?;

        match list_type {
            "tools" => self.list_tools(router).await,
            "resources" => self.list_resources(router).await,
            "prompts" => self.list_prompts(router).await,
            _ => Err(ProxyError::InvalidRequest(format!(
                "Unknown list type: {list_type}"
            ))),
        }
    }

    async fn handle_call_with_tracking(
        &self,
        params: CallParams,
        router: Arc<RequestRouter>,
        tracking_response_id: &Option<String>,
    ) -> Result<Value> {
        let result = self.handle_call(params.clone(), router).await?;

        // Record context from this backend call
        if let Some(resp_id) = tracking_response_id {
            // Extract server name from tool name
            let server_name = if params.tool.starts_with("mcp__proxy__") {
                let parts: Vec<&str> = params.tool.splitn(4, "__").collect();
                if parts.len() >= 3 {
                    parts[2].to_string()
                } else {
                    "unknown".to_string()
                }
            } else {
                "unknown".to_string()
            };

            self.record_context_from_server(
                &Some(resp_id.clone()),
                &server_name,
                "tools/call",
                &params.tool,
            )
            .await;
        }

        Ok(result)
    }

    async fn handle_call(&self, params: CallParams, router: Arc<RequestRouter>) -> Result<Value> {
        // Check if tool name has proxy prefix: mcp__proxy__{server}__{tool}
        let (server_name, original_tool_name) = if params.tool.starts_with("mcp__proxy__") {
            // Parse the prefixed name to extract server and original tool name
            let parts: Vec<&str> = params.tool.splitn(4, "__").collect();
            if parts.len() == 4 && parts[0] == "mcp" && parts[1] == "proxy" {
                let server = parts[2].replace("_", "-");
                let tool = parts[3].to_string();
                (server, tool)
            } else {
                // Malformed prefix, try original routing
                let server = router.get_server_for_tool(&params.tool).ok_or_else(|| {
                    ProxyError::ServerNotFound(format!("No server handles tool: {}", params.tool))
                })?;
                (server, params.tool.clone())
            }
        } else {
            // No prefix, use router to find server
            let server = router.get_server_for_tool(&params.tool).ok_or_else(|| {
                ProxyError::ServerNotFound(format!("No server handles tool: {}", params.tool))
            })?;
            (server, params.tool.clone())
        };

        // Check if server is enabled
        let config = self.state.config.read().await;
        let server_enabled = config
            .servers
            .get(&server_name)
            .map(|s| s.enabled)
            .unwrap_or(true);
        drop(config);

        if !server_enabled {
            return Err(ProxyError::ServerNotFound(format!(
                "Server '{server_name}' is disabled. Enable it with mcp__proxy__server__enable"
            )));
        }

        // Extract tokens parameter from arguments (for Context7 and similar servers)
        let max_tokens = params
            .arguments
            .get("tokens")
            .and_then(|t| t.as_u64())
            .map(|t| t as u32);

        // Apply request-phase plugins before forwarding to server
        let processed_arguments = self
            .apply_request_plugins(&server_name, &original_tool_name, params.arguments.clone())
            .await?;

        // Get connection from pool
        let conn = self.state.connection_pool.get(&server_name).await?;

        // Forward request to server with ORIGINAL tool name (no prefix)
        // Use MCP spec format: tools/call with "name" field
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "tools/call",
            "params": {
                "name": original_tool_name,
                "arguments": processed_arguments,
            },
            "id": 1
        });

        conn.send(bytes::Bytes::from(format!("{request}\n")))
            .await?;

        // Get response
        let response = conn.recv().await?;
        let response: Value = serde_json::from_slice(&response)?;

        // Extract result
        let mut result = response
            .get("result")
            .cloned()
            .ok_or_else(|| ProxyError::InvalidRequest("No result in response".to_string()))?;

        // Apply response-phase plugins if configured
        result = self
            .apply_response_plugins(
                &server_name,
                &original_tool_name,
                result,
                max_tokens,
                Some(params.arguments.clone()),
            )
            .await?;

        Ok(result)
    }

    async fn handle_read(&self, params: ReadParams, router: Arc<RequestRouter>) -> Result<Value> {
        // Find server that handles this resource
        let server_name = router.get_server_for_resource(&params.uri).ok_or_else(|| {
            ProxyError::ServerNotFound(format!("No server handles resource: {}", params.uri))
        })?;

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

        conn.send(bytes::Bytes::from(format!("{request}\n")))
            .await?;

        // Get response
        let response = conn.recv().await?;
        let response: Value = serde_json::from_slice(&response)?;

        // Extract result
        response
            .get("result")
            .cloned()
            .ok_or_else(|| ProxyError::InvalidRequest("No result in response".to_string()))
    }

    async fn list_tools(&self, _router: Arc<RequestRouter>) -> Result<Value> {
        // Aggregate tools from all backend servers
        match self.forward_to_all_servers("tools/list", None).await {
            Ok(mut result) => {
                // Add proxy management tools
                if let Some(tools_array) = result.get_mut("tools").and_then(|t| t.as_array_mut()) {
                    let tracing_tools = super::tracing_tools::get_tracing_tools();
                    let server_tools = super::server_tools::get_server_tools();
                    tools_array.extend(tracing_tools);
                    tools_array.extend(server_tools);
                }
                Ok(result)
            }
            Err(_) => {
                // If no backends available, return only proxy tools
                let mut proxy_tools = super::tracing_tools::get_tracing_tools();
                proxy_tools.extend(super::server_tools::get_server_tools());
                Ok(serde_json::json!({ "tools": proxy_tools }))
            }
        }
    }

    async fn list_resources(&self, _router: Arc<RequestRouter>) -> Result<Value> {
        // Aggregate resources from all backend servers
        match self.forward_to_all_servers("resources/list", None).await {
            Ok(mut result) => {
                // Add proxy-native resources
                if let Some(resources_array) =
                    result.get_mut("resources").and_then(|r| r.as_array_mut())
                {
                    let proxy_resources = super::resources::get_proxy_resources();
                    resources_array.extend(proxy_resources);
                }
                Ok(result)
            }
            Err(_) => {
                // If no backends available, return proxy + tracing resources
                let mut all_resources = super::tracing_tools::get_tracing_resources();
                all_resources.extend(super::resources::get_proxy_resources());
                Ok(serde_json::json!({ "resources": all_resources }))
            }
        }
    }

    async fn list_prompts(&self, _router: Arc<RequestRouter>) -> Result<Value> {
        // Aggregate prompts from all backend servers
        match self.forward_to_all_servers("prompts/list", None).await {
            Ok(mut result) => {
                // Add proxy-native prompts
                if let Some(prompts_array) =
                    result.get_mut("prompts").and_then(|p| p.as_array_mut())
                {
                    let proxy_prompts = super::prompts::get_proxy_prompts();
                    prompts_array.extend(proxy_prompts);
                }
                Ok(result)
            }
            Err(_) => {
                // If no backends available, return only proxy prompts
                let proxy_prompts = super::prompts::get_proxy_prompts();
                Ok(serde_json::json!({ "prompts": proxy_prompts }))
            }
        }
    }

    async fn forward_to_all_servers(&self, method: &str, params: Option<&Value>) -> Result<Value> {
        // For methods like "tools/list", we need to aggregate results from all servers
        use futures::future::join_all;
        use tokio::time::{timeout, Duration};

        // Collect server names
        let server_names: Vec<String> = self
            .state
            .servers
            .iter()
            .map(|entry| entry.key().clone())
            .collect();

        // Create concurrent requests with timeout
        let mut futures = Vec::new();
        for server_name in server_names {
            let method = method.to_string();
            let params = params.cloned();
            let handler = self.clone();

            futures.push(tokio::spawn(async move {
                // Apply a per-server timeout of 30 seconds (to accommodate slow initialization)
                let result = timeout(
                    Duration::from_secs(30),
                    handler.forward_to_server(&server_name, &method, params.as_ref()),
                )
                .await;

                match result {
                    Ok(Ok(value)) => (server_name.clone(), Ok(value)),
                    Ok(Err(e)) => (server_name, Err(e)),
                    Err(_) => (server_name.clone(), Err(ProxyError::Timeout)),
                }
            }));
        }

        // Wait for all requests to complete
        let results = join_all(futures).await;

        // Collect successful results with server name
        let mut aggregated_results: Vec<(String, Value)> = Vec::new();
        for result in results {
            match result {
                Ok((server_name, Ok(value))) => {
                    tracing::debug!("Server {} successfully handled {}", server_name, method);
                    aggregated_results.push((server_name, value));
                }
                Ok((server_name, Err(e))) => {
                    tracing::debug!("Server {} failed to handle {}: {}", server_name, method, e);
                }
                Err(e) => {
                    tracing::warn!("Task join error: {}", e);
                }
            }
        }

        if aggregated_results.is_empty() {
            return Err(ProxyError::InvalidRequest(
                "No servers could handle the request".to_string(),
            ));
        }

        // Aggregate results based on method type
        match method {
            "tools/list" => {
                let mut all_tools = Vec::new();
                for (server_name, result) in aggregated_results {
                    if let Some(tools) = result.get("tools").and_then(|t| t.as_array()) {
                        // Prefix each tool name with mcp__proxy__{server_name}__
                        for tool in tools {
                            let mut prefixed_tool = tool.clone();
                            if let Some(tool_obj) = prefixed_tool.as_object_mut() {
                                if let Some(name) = tool_obj.get("name").and_then(|n| n.as_str()) {
                                    let name_str = name.to_string();
                                    let prefixed_name = format!(
                                        "mcp__proxy__{}__{}",
                                        server_name.replace("-", "_"),
                                        name_str
                                    );
                                    tool_obj.insert(
                                        "name".to_string(),
                                        serde_json::json!(prefixed_name),
                                    );

                                    // Also store original name for reference
                                    tool_obj.insert(
                                        "originalName".to_string(),
                                        serde_json::json!(name_str),
                                    );
                                    tool_obj.insert(
                                        "server".to_string(),
                                        serde_json::json!(server_name.clone()),
                                    );
                                }
                            }
                            all_tools.push(prefixed_tool);
                        }
                    }
                }
                Ok(serde_json::json!({ "tools": all_tools }))
            }
            "resources/list" => {
                let mut all_resources = Vec::new();
                for (server_name, result) in aggregated_results {
                    if let Some(resources) = result.get("resources").and_then(|r| r.as_array()) {
                        // Prefix each resource URI with server name
                        for resource in resources {
                            let mut prefixed_resource = resource.clone();
                            if let Some(res_obj) = prefixed_resource.as_object_mut() {
                                if let Some(uri) = res_obj.get("uri").and_then(|u| u.as_str()) {
                                    let uri_str = uri.to_string();
                                    let prefixed_uri = format!(
                                        "mcp__proxy__{}://{}",
                                        server_name.replace("-", "_"),
                                        uri_str
                                    );
                                    res_obj
                                        .insert("uri".to_string(), serde_json::json!(prefixed_uri));
                                    res_obj.insert(
                                        "originalUri".to_string(),
                                        serde_json::json!(uri_str),
                                    );
                                    res_obj.insert(
                                        "server".to_string(),
                                        serde_json::json!(server_name.clone()),
                                    );
                                }
                            }
                            all_resources.push(prefixed_resource);
                        }
                    }
                }

                // Add context tracing resources
                let tracing_resources = super::tracing_tools::get_tracing_resources();
                all_resources.extend(tracing_resources);

                Ok(serde_json::json!({ "resources": all_resources }))
            }
            "prompts/list" => {
                let mut all_prompts = Vec::new();
                for (server_name, result) in aggregated_results {
                    if let Some(prompts) = result.get("prompts").and_then(|p| p.as_array()) {
                        // Prefix each prompt name with server name
                        for prompt in prompts {
                            let mut prefixed_prompt = prompt.clone();
                            if let Some(prompt_obj) = prefixed_prompt.as_object_mut() {
                                if let Some(name) = prompt_obj.get("name").and_then(|n| n.as_str())
                                {
                                    let name_str = name.to_string();
                                    let prefixed_name = format!(
                                        "mcp__proxy__{}__{}",
                                        server_name.replace("-", "_"),
                                        name_str
                                    );
                                    prompt_obj.insert(
                                        "name".to_string(),
                                        serde_json::json!(prefixed_name),
                                    );
                                    prompt_obj.insert(
                                        "originalName".to_string(),
                                        serde_json::json!(name_str),
                                    );
                                    prompt_obj.insert(
                                        "server".to_string(),
                                        serde_json::json!(server_name.clone()),
                                    );
                                }
                            }
                            all_prompts.push(prefixed_prompt);
                        }
                    }
                }
                Ok(serde_json::json!({ "prompts": all_prompts }))
            }
            _ => {
                // For other methods, just return the first successful result
                Ok(aggregated_results.into_iter().next().unwrap().1)
            }
        }
    }

    /// Record a context unit from a backend server call
    async fn record_context_from_server(
        &self,
        tracking_response_id: &Option<String>,
        server_name: &str,
        method: &str,
        tool_or_resource: &str,
    ) {
        if let Some(resp_id) = tracking_response_id {
            if let Some(tracker) = &*self.state.context_tracker.read().await {
                use crate::context::types::{ContextType, ContextUnit};
                use chrono::Utc;
                use uuid::Uuid;

                let context = ContextUnit {
                    id: format!("ctx_{}", Uuid::new_v4()),
                    r#type: ContextType::External,
                    source: format!("{server_name}::{tool_or_resource}"),
                    timestamp: Utc::now(),
                    embedding_id: None,
                    summary: Some(format!("{method} from {server_name}")),
                    version: 1,
                    previous_version_id: None,
                    aggregate_score: 0.0,
                    feedback_count: 0,
                };

                // Add context with default retrieval score of 0.8
                if let Err(e) = tracker
                    .add_context(resp_id.clone(), context, Some(0.8))
                    .await
                {
                    tracing::warn!("Failed to record context: {}", e);
                }
            }
        }
    }

    async fn forward_to_server(
        &self,
        server_name: &str,
        method: &str,
        params: Option<&Value>,
    ) -> Result<Value> {
        // T025: Check if server is ready before forwarding request
        if let Some(server_info) = self.state.servers.get(server_name) {
            if let Some(connection_state) = &server_info.connection_state {
                // Check if request can be sent in current state
                if !connection_state.can_send_request(method).await {
                    tracing::debug!(
                        server = server_name,
                        method = method,
                        "Request blocked - server not ready"
                    );
                    return Err(ProxyError::ServerNotReady(format!(
                        "Server '{server_name}' is not ready to handle '{method}' requests. Server is still initializing."
                    )));
                }
            }

            // Update last access time
            let mut last_access = server_info.last_access_time.write().await;
            *last_access = Some(chrono::Utc::now());
        }

        let conn = self.state.connection_pool.get(server_name).await?;

        let mut request = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params.cloned().unwrap_or(serde_json::json!({})),
            "id": 1
        });

        // T047: Translate request using protocol adapter if available
        if let Some(server_info) = self.state.servers.get(server_name) {
            if let Some(connection_state) = &server_info.connection_state {
                if let Some(adapter) = connection_state.get_adapter().await {
                    request = adapter.translate_request(request).await.map_err(|e| {
                        ProxyError::InvalidRequest(format!("Translation error: {e}"))
                    })?;
                }
            }
        }

        let request_bytes = bytes::Bytes::from(format!("{request}\n"));
        conn.send(request_bytes).await?;

        let response_bytes = conn.recv().await?;
        let mut response: Value = serde_json::from_slice(&response_bytes)?;

        // T047: Translate response using protocol adapter if available
        if let Some(server_info) = self.state.servers.get(server_name) {
            if let Some(connection_state) = &server_info.connection_state {
                if let Some(adapter) = connection_state.get_adapter().await {
                    response = adapter.translate_response(response).await.map_err(|e| {
                        ProxyError::InvalidRequest(format!("Translation error: {e}"))
                    })?;
                }
            }
        }

        // Check for error
        if let Some(error) = response.get("error") {
            return Err(ProxyError::InvalidRequest(format!(
                "Server error: {}",
                error
                    .get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("Unknown")
            )));
        }

        response
            .get("result")
            .cloned()
            .ok_or_else(|| ProxyError::InvalidRequest("No result in response".to_string()))
    }

    /// Apply request-phase plugins to modify or block requests before forwarding
    async fn apply_request_plugins(
        &self,
        server_name: &str,
        tool_name: &str,
        arguments: Value,
    ) -> Result<Value> {
        // Check if plugins are configured
        let plugin_manager = match &self.state.plugin_manager {
            Some(manager) => manager,
            None => return Ok(arguments), // No plugins configured
        };

        // Get plugin config from state
        let config_guard = self.state.config.read().await;
        let plugin_config = match &config_guard.plugins {
            Some(config) => config.clone(),
            None => return Ok(arguments),
        };
        drop(config_guard);

        // Build plugin chain for this server (request phase)
        use crate::plugin::chain::PluginChain;
        use crate::plugin::schema::{PluginError, PluginInput, PluginMetadata, PluginPhase};
        use uuid::Uuid;

        let chain = PluginChain::new(
            server_name.to_string(),
            PluginPhase::Request,
            plugin_manager.clone(),
            Arc::new(plugin_config.clone()),
        );

        // Convert arguments to string for plugin processing
        let raw_content = serde_json::to_string(&arguments).map_err(|e| {
            ProxyError::InvalidRequest(format!("Failed to serialize arguments: {e}"))
        })?;

        // Create plugin input
        let input = PluginInput {
            tool_name: tool_name.to_string(),
            raw_content,
            max_tokens: None,
            metadata: PluginMetadata {
                request_id: Uuid::new_v4().to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
                server_name: server_name.to_string(),
                phase: PluginPhase::Request,
                user_query: None,
                tool_arguments: Some(arguments.clone()),
                mcp_servers: None,
            },
        };

        // Execute plugin chain
        let output = match chain.execute(input.clone()).await {
            Ok(output) => {
                // Check if plugin blocked the request
                if !output.continue_ {
                    return Err(ProxyError::InvalidRequest(output.error.unwrap_or_else(
                        || "Request blocked by security plugin".to_string(),
                    )));
                }
                output
            }
            Err(PluginError::Timeout { .. }) => {
                tracing::warn!("Request plugin timed out, using original arguments");
                return Ok(arguments);
            }
            Err(e) => {
                tracing::warn!("Request plugin failed: {}, using original arguments", e);
                return Ok(arguments);
            }
        };

        // Parse the modified text back to JSON
        let modified_arguments: Value = match serde_json::from_str(&output.text) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!("Plugin output is not valid JSON, returning original: {}", e);
                // If plugin output is not valid JSON, fall back to original
                arguments
            }
        };

        Ok(modified_arguments)
    }

    /// Apply response-phase plugins to modify server response
    async fn apply_response_plugins(
        &self,
        server_name: &str,
        tool_name: &str,
        result: Value,
        max_tokens: Option<u32>,
        tool_arguments: Option<Value>,
    ) -> Result<Value> {
        // Check if plugins are configured
        let plugin_manager = match &self.state.plugin_manager {
            Some(manager) => manager,
            None => return Ok(result), // No plugins configured
        };

        // Get plugin config from state
        let config_guard = self.state.config.read().await;
        let plugin_config = match &config_guard.plugins {
            Some(config) => config.clone(),
            None => return Ok(result),
        };
        drop(config_guard);

        // Build plugin chain for this server
        use crate::plugin::chain::PluginChain;
        use crate::plugin::schema::{PluginInput, PluginMetadata, PluginPhase};
        use uuid::Uuid;

        let chain = PluginChain::new(
            server_name.to_string(),
            PluginPhase::Response,
            plugin_manager.clone(),
            Arc::new(plugin_config.clone()),
        );

        // Convert result to string for plugin processing
        let raw_content = serde_json::to_string(&result).map_err(|e| {
            ProxyError::InvalidRequest(format!("Failed to serialize response: {e}"))
        })?;

        // Create plugin input (max_tokens and tool_arguments passed from handle_call)
        let input = PluginInput {
            tool_name: tool_name.to_string(),
            raw_content,
            max_tokens,
            metadata: PluginMetadata {
                request_id: Uuid::new_v4().to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
                server_name: server_name.to_string(),
                phase: PluginPhase::Response,
                user_query: None, // TODO: Extract from request context
                tool_arguments,
                mcp_servers: None,
            },
        };

        // Execute plugin chain (safe execution always returns content)
        let output = chain.execute_safe(input).await;

        // Parse the modified text back to JSON
        let modified_result: Value = match serde_json::from_str(&output.text) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!("Plugin output is not valid JSON, returning original: {}", e);
                // If plugin output is not valid JSON, fall back to original
                result
            }
        };

        Ok(modified_result)
    }

    /// Clear the tools/list cache
    pub async fn clear_cache(&self) {
        let mut cache = self.tools_list_cache.write().await;
        *cache = None;
        tracing::debug!("Cleared tools/list cache");
    }
}
