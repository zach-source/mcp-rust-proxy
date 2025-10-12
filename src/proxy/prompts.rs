use serde_json::{json, Value};

/// Get all proxy-native prompts
pub fn get_proxy_prompts() -> Vec<Value> {
    vec![
        create_prompt(
            "debug-mcp-server",
            "Debug an MCP server that isn't responding or has errors",
            vec![create_argument(
                "server_name",
                "Name of the MCP server to debug",
                true,
            )],
            "This prompt helps you diagnose issues with an MCP server by:\n\
            1. Checking the server status with mcp__proxy__server__list\n\
            2. Reading recent server logs using the proxy://logs/{server_name} resource\n\
            3. Reviewing recent traced requests with mcp__proxy__tracing__get_trace\n\
            4. Suggesting fixes based on common error patterns\n\n\
            Example usage:\n\
            - First, list all servers to get the exact name\n\
            - Then use this prompt with the server name\n\
            - Follow the diagnostic steps in order",
        ),
        create_prompt(
            "analyze-codebase-with-context",
            "Perform comprehensive codebase analysis using Serena + Context7",
            vec![
                create_argument("project_path", "Absolute path to the project root", true),
                create_argument(
                    "library_name",
                    "Main framework/library used (optional)",
                    false,
                ),
            ],
            "This prompt combines semantic code analysis with framework documentation:\n\
            1. Activate the project: mcp__proxy__serena__activate_project\n\
            2. Get high-level structure: mcp__proxy__serena__get_symbols_overview\n\
            3. If library_name provided, get docs: mcp__proxy__context7__resolve_library_id\n\
            4. Use mcp__proxy__serena__find_symbol for targeted code reading\n\
            5. Combine code + docs for comprehensive understanding\n\n\
            This approach is token-efficient and provides deep insight into unfamiliar codebases.",
        ),
        create_prompt(
            "review-context-quality",
            "Review and improve AI context tracing quality scores",
            vec![create_argument(
                "min_score_threshold",
                "Minimum acceptable quality score (0.0-1.0)",
                false,
            )],
            "This prompt helps maintain high-quality context tracking:\n\
            1. Get quality report: mcp__proxy__tracing__quality_report\n\
            2. Identify contexts below threshold\n\
            3. Use mcp__proxy__tracing__query_context_impact to see which responses were affected\n\
            4. Submit feedback to improve scores: mcp__proxy__tracing__submit_feedback\n\
            5. Review evolution history: mcp__proxy__tracing__get_evolution_history\n\n\
            Regular quality review ensures the context tracing system improves over time.",
        ),
        create_prompt(
            "optimize-server-performance",
            "Analyze and optimize proxy server performance",
            vec![],
            "This prompt guides you through performance optimization:\n\
            1. Read proxy metrics: proxy://metrics resource\n\
            2. Identify slow servers from response times\n\
            3. Check server health: mcp__proxy__server__list\n\
            4. Review error logs for failed requests\n\
            5. Suggest configuration changes (pool size, timeouts, etc.)\n\n\
            Use this when you notice slow response times or want to tune the proxy.",
        ),
        create_prompt(
            "cross-server-workflow",
            "Execute complex workflows spanning multiple MCP servers",
            vec![create_argument(
                "workflow_description",
                "Natural language description of desired workflow",
                true,
            )],
            "This meta-prompt helps you design and execute multi-server workflows:\n\
            1. Read proxy topology: proxy://topology resource\n\
            2. Identify which servers provide needed capabilities\n\
            3. Design workflow using available tools\n\
            4. Execute steps in logical order\n\
            5. Track progress with context tracing\n\n\
            Example workflows:\n\
            - 'Find all uses of a function across repos' (Serena + Git)\n\
            - 'Update code and validate with docs' (Serena + Context7)\n\
            - 'Test web app and analyze results' (Playwright + Memory)",
        ),
    ]
}

/// Create a prompt definition
fn create_prompt(
    name: &str,
    description: &str,
    arguments: Vec<Value>,
    instructions: &str,
) -> Value {
    json!({
        "name": name,
        "description": description,
        "arguments": arguments,
        "instructions": instructions
    })
}

/// Create a prompt argument
fn create_argument(name: &str, description: &str, required: bool) -> Value {
    json!({
        "name": name,
        "description": description,
        "required": required
    })
}

/// Get a specific prompt by name
pub fn get_prompt(name: &str, _arguments: Option<Value>) -> Option<Value> {
    let prompts = get_proxy_prompts();
    prompts.into_iter().find(|p| {
        p.get("name")
            .and_then(|n| n.as_str())
            .map(|n| n == name)
            .unwrap_or(false)
    })
}
