# MCP Advanced Features Implementation Plan
## Leveraging Prompts, Resources, and Templates for an Effective Proxy Server

## Vision

Transform MCP Rust Proxy from a simple pass-through aggregator into an **intelligent orchestration layer** that provides:
- Guided workflows across multiple backend servers
- Dynamic access to proxy state and metrics
- Pre-configured task templates for common operations
- Self-documenting capabilities

## Current State Analysis

### What We Have Now
- ✅ 103 tools aggregated from 11 backend servers
- ✅ Basic resource support (tracing resources)
- ✅ Prompts forwarding (1 prompt from backends)
- ❌ **No proxy-native prompts** to guide multi-server workflows
- ❌ **No dynamic resources** for proxy state/configuration
- ❌ **No resource templates** for parameterized access

### Opportunity Gap

Clients using our proxy have access to 103 tools across 11 servers, but:
1. **No guidance** on how to combine tools from different servers
2. **No visibility** into proxy health, metrics, or configuration
3. **No templates** for common cross-server workflows
4. **Discovery friction** - hard to know which server to use for what

## Strategic Use Cases

### 1. Prompts: Workflow Orchestration

**Problem**: Users don't know how to combine tools from multiple servers to accomplish complex tasks.

**Solution**: Create prompts that guide multi-server workflows.

#### Example Prompts

**Prompt: `debug-mcp-server`**
```yaml
name: debug-mcp-server
description: Debug why an MCP server isn't working correctly
arguments:
  - name: server_name
    description: Name of the server to debug
    required: true

workflow:
  1. Use mcp__proxy__server__list to check server state
  2. Use resources to read server logs
  3. Use mcp__proxy__tracing__get_trace to see recent activity
  4. Suggest fixes based on error patterns
```

**Prompt: `analyze-codebase-with-context`**
```yaml
name: analyze-codebase-with-context
description: Analyze a codebase using Serena + Context7 for deep understanding
arguments:
  - name: project_path
    description: Path to the project
    required: true
  - name: library_name
    description: Framework/library used (for Context7 docs)
    required: false

workflow:
  1. Activate project with mcp__proxy__serena__activate_project
  2. Get symbols overview with mcp__proxy__serena__get_symbols_overview
  3. Resolve library docs with mcp__proxy__context7__resolve_library_id
  4. Combine code + docs for comprehensive analysis
```

**Prompt: `optimize-context-quality`**
```yaml
name: optimize-context-quality
description: Review and improve context tracing quality scores
arguments:
  - name: min_score_threshold
    description: Minimum acceptable score
    required: false
    default: 0.5

workflow:
  1. Get quality report with mcp__proxy__tracing__quality_report
  2. Identify low-scoring contexts
  3. Query impact with mcp__proxy__tracing__query_context_impact
  4. Suggest re-indexing or updates
```

### 2. Resources: Proxy Introspection

**Problem**: No programmatic access to proxy state, configuration, or operational metrics.

**Solution**: Expose proxy internals as MCP resources.

#### Proposed Resources

**Resource: `proxy://config`**
- **Type**: `application/json`
- **Content**: Current proxy configuration (sanitized, no secrets)
- **Use Case**: LLMs can read config to understand server topology
- **Example**: `{"servers": {"serena": {"transport": "stdio", "enabled": true}}}`

**Resource: `proxy://metrics`**
- **Type**: `application/json`
- **Content**: Real-time proxy metrics
- **Use Case**: Monitor performance, identify bottlenecks
- **Example**: `{"requests_total": 1543, "requests_per_server": {...}, "avg_response_time_ms": 45}`

**Resource: `proxy://health`**
- **Type**: `application/json`
- **Content**: Health check status for all servers
- **Use Case**: Quick overview of system health
- **Example**: `{"healthy": 9, "failed": 1, "stopped": 1, "servers": [...]}`

**Resource: `proxy://topology`**
- **Type**: `application/json`
- **Content**: Server dependency graph and capabilities
- **Use Case**: LLMs can understand which servers provide what
- **Example**: `{"servers": [{"name": "serena", "provides": ["code-analysis", "editing"], "tools_count": 20}]}`

**Resource: `proxy://session/{session_id}`**
- **Type**: `application/json`
- **Content**: Context tracing session data
- **Use Case**: Review session history, quality scores
- **Example**: Session summary with all tracked responses

### 3. Resource Templates: Dynamic Access Patterns

**Problem**: Static resources don't scale for dynamic proxy state (logs, per-server metrics, traces).

**Solution**: Use URI templates for parameterized access.

#### Proposed Resource Templates

**Template: `proxy://logs/{server_name}`**
- **Pattern**: `proxy://logs/{server_name}?lines={lines}&level={level}`
- **Content**: Recent log lines from specified server
- **Use Case**: LLMs can debug server issues by reading logs
- **Example**: `proxy://logs/serena?lines=100&level=error`

**Template: `proxy://metrics/{server_name}`**
- **Pattern**: `proxy://metrics/{server_name}?period={period}`
- **Content**: Per-server metrics (request count, latency, errors)
- **Use Case**: Performance analysis per server
- **Example**: `proxy://metrics/context7?period=1h`

**Template: `proxy://trace/{response_id}`**
- **Pattern**: `proxy://trace/{response_id}?format={format}`
- **Content**: Full context trace for a response
- **Use Case**: Already implemented via tools, expose as resource too
- **Example**: `proxy://trace/resp_abc123?format=tree`

**Template: `proxy://server/{server_name}/config`**
- **Pattern**: `proxy://server/{server_name}/config`
- **Content**: Configuration for specific server
- **Use Case**: LLMs can check server settings before making requests
- **Example**: `proxy://server/serena/config`

**Template: `proxy://server/{server_name}/capabilities`**
- **Pattern**: `proxy://server/{server_name}/capabilities`
- **Content**: Capabilities from server's initialize response
- **Use Case**: Discover what a server supports dynamically
- **Example**: `proxy://server/playwright/capabilities`

## Implementation Plan

### Phase 1: Proxy-Native Prompts (Week 1)

**File**: `src/proxy/prompts.rs`

```rust
pub fn get_proxy_prompts() -> Vec<Value> {
    vec![
        prompt("debug-mcp-server", "Debug an MCP server that isn't working",
            vec![arg("server_name", "Name of the server to debug", true)],
            "Use mcp__proxy__server__list to check state, then read logs..."),

        prompt("analyze-codebase", "Analyze codebase with Serena + Context7",
            vec![arg("project_path", "Path to project", true)],
            "Activate project, get overview, resolve library docs..."),

        prompt("review-context-quality", "Review and improve context tracing quality",
            vec![],
            "Get quality report, identify low scorers, suggest improvements..."),
    ]
}
```

**Integration**:
- Add to `prompts/list` response in handler.rs
- Implement `prompts/get` to return prompt details

### Phase 2: Static Proxy Resources (Week 2)

**File**: `src/proxy/resources.rs`

```rust
pub fn get_proxy_resources() -> Vec<Value> {
    vec![
        resource("proxy://config", "Current proxy configuration", "application/json"),
        resource("proxy://metrics", "Real-time proxy metrics", "application/json"),
        resource("proxy://health", "Server health status", "application/json"),
        resource("proxy://topology", "Server topology and capabilities", "application/json"),
    ]
}
```

**Implementation**:
- Add handler for `resources/read` when URI starts with `proxy://`
- Generate JSON from current AppState
- Sanitize sensitive data (API keys, tokens)

### Phase 3: Resource Templates (Week 3)

**File**: `src/proxy/resource_templates.rs`

```rust
pub fn get_proxy_resource_templates() -> Vec<Value> {
    vec![
        template("proxy://logs/{server_name}", "Server log access"),
        template("proxy://metrics/{server_name}", "Per-server metrics"),
        template("proxy://trace/{response_id}", "Context trace"),
        template("proxy://server/{server_name}/config", "Server configuration"),
        template("proxy://server/{server_name}/capabilities", "Server capabilities"),
    ]
}
```

**Implementation**:
- Parse URI template parameters
- Route to appropriate handler based on template
- Return error for invalid parameters

### Phase 4: Advanced Workflows (Week 4)

**Goal**: Create composite prompts that demonstrate proxy value

#### Prompt: `cross-server-analysis`
Combines Serena (code), Context7 (docs), Git (history), Memory (learning):
1. Analyze code structure with Serena
2. Fetch relevant documentation with Context7
3. Check git history for context
4. Store learnings in Memory
5. Return comprehensive analysis

#### Prompt: `performance-audit`
Uses multiple servers for holistic performance review:
1. Read proxy metrics resource
2. Identify slow servers
3. Read server logs for errors
4. Query context tracing for quality issues
5. Generate optimization recommendations

#### Prompt: `onboard-developer`
Help new developers understand the codebase:
1. Check if Serena onboarding complete
2. If not, run mcp__proxy__serena__onboarding
3. Generate topology resource to show architecture
4. List key memories with mcp__proxy__serena__list_memories
5. Provide guided tour of codebase

## Benefits & Impact

### For LLM Agents
- **Reduced token usage**: Read structured resources instead of raw logs
- **Better decision making**: Access to proxy topology and metrics
- **Guided workflows**: Prompts show best practices for tool combinations
- **Self-discovery**: Resource templates enable exploration

### For Users
- **Faster debugging**: Prompts guide through diagnostic workflows
- **Better insights**: Resources expose proxy internals programmatically
- **Reduced errors**: Templated workflows reduce trial-and-error
- **Documentation as code**: Prompts serve as executable documentation

### For Proxy
- **Differentiation**: More than aggregation - intelligent orchestration
- **Observability**: Resources make proxy transparent to LLMs
- **Extensibility**: Easy to add new prompts/resources for custom workflows
- **Self-improving**: Context tracing + prompts create feedback loop

## Technical Implementation Details

### Prompt Storage
**Option A**: Static Rust code (fast, compiled)
**Option B**: YAML config files (flexible, no rebuild)
**Recommendation**: Start with static, add config file support later

### Resource Implementation
```rust
// In handler.rs
"resources/read" => {
    if params.uri.starts_with("proxy://") {
        self.handle_proxy_resource(&params.uri).await?
    } else if params.uri.starts_with("trace://") {
        // Existing tracing resource handler
        ...
    } else {
        self.handle_read(params, router).await?
    }
}
```

### Resource Template Parsing
```rust
fn parse_resource_template(uri: &str) -> Option<(String, HashMap<String, String>)> {
    // proxy://logs/{server_name}?lines=100
    // Returns: ("logs", {"server_name": "serena", "lines": "100"})
}
```

## Success Metrics

- [ ] 5+ useful prompts for common workflows
- [ ] 10+ resources exposing proxy state
- [ ] 5+ resource templates for dynamic access
- [ ] Documentation showing prompt-driven workflows
- [ ] Integration tests for all new resources/prompts
- [ ] Performance benchmarks (resource read < 10ms)

## Migration Path

**Phase 1**: Add prompts without breaking changes
**Phase 2**: Add static resources alongside existing
**Phase 3**: Add templates, test with real workflows
**Phase 4**: Gather feedback, iterate on prompts

All changes are **backward compatible** - existing clients continue to work.

## File Structure

```
src/proxy/
├── handler.rs          # Route to new handlers
├── prompts.rs         # NEW: Prompt definitions and prompts/get
├── resources.rs       # NEW: Static resource handlers
├── resource_templates.rs # NEW: Template parsing and handling
├── mod.rs             # Export new modules
└── workflows/         # NEW: Example workflow implementations
    ├── debug_server.rs
    ├── analyze_code.rs
    └── optimize_quality.rs
```

## Next Steps

1. Create prompts.rs with 3-5 initial prompts
2. Create resources.rs with proxy state resources
3. Implement resource template parsing
4. Add integration tests
5. Document workflows in README
6. Create example client scripts demonstrating workflows

## Open Questions

1. Should prompts be configurable via YAML or hardcoded?
2. What's the right caching strategy for dynamic resources?
3. Should we support prompt composition (prompts calling other prompts)?
4. How to handle versioning of prompts/resources?
5. Should resource templates support wildcards (e.g., `proxy://logs/*`)?
