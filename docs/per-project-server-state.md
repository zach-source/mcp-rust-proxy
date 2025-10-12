# Per-Project Server State System

## Goal
Allow MCP tool calls (`mcp__proxy__server__enable`/`disable`) to override the base configuration and persist those changes per-project.

## Architecture

### State Priority (Highest to Lowest)
1. **Runtime Overrides** - Changes made via MCP tools (stored in `.mcp-proxy-overrides.json`)
2. **Base Config** - Main configuration file (`mcp-proxy-config.yaml`)

### File Structure

#### Base Config (`mcp-proxy-config.yaml`)
```yaml
servers:
  serena:
    command: "serena-mcp-launcher"
    enabled: true  # Default state
    # ... other config
```

#### Runtime Overrides (`.mcp-proxy-overrides.json`)
```json
{
  "project": "/Users/ztaylor/repos/workspaces/mcp-rust-proxy/main",
  "overrides": {
    "serena": {
      "enabled": false
    },
    "github": {
      "enabled": true
    }
  },
  "lastModified": "2025-10-11T21:00:00Z"
}
```

### Behavior

**On Startup:**
1. Load base config from `mcp-proxy-config.yaml`
2. Detect current project directory (from `PWD` or config)
3. Load overrides from `.mcp-proxy-overrides.json` if exists
4. Apply overrides to base config
5. Start servers with final enabled state

**On `mcp__proxy__server__enable`:**
1. Enable server in runtime state
2. Update `.mcp-proxy-overrides.json` with `{server: {enabled: true}}`
3. Start the server process
4. Clear tools/list cache

**On `mcp__proxy__server__disable`:**
1. Disable server in runtime state
2. Update `.mcp-proxy-overrides.json` with `{server: {enabled: false}}`
3. Stop the server process
4. Clear tools/list cache

### File Location

**Option 1: Project-specific (Recommended)**
```
/Users/ztaylor/repos/workspaces/mcp-rust-proxy/main/.mcp-proxy-overrides.json
```
- ✅ Per-project isolation
- ✅ Can be gitignored
- ✅ Clear ownership

**Option 2: Global with project key**
```
~/.mcp-proxy/project-overrides.json
{
  "/path/to/project1": { "serena": {"enabled": false} },
  "/path/to/project2": { "serena": {"enabled": true} }
}
```
- ✅ Centralized management
- ❌ Complex project detection
- ❌ Path resolution issues

**Decision: Use Option 1 (project-specific files)**

### Implementation Plan

#### 1. Add Override Config Schema (`src/config/overrides.rs`)
```rust
pub struct ServerOverrides {
    pub project: PathBuf,
    pub overrides: HashMap<String, ServerOverride>,
    pub last_modified: DateTime<Utc>,
}

pub struct ServerOverride {
    pub enabled: Option<bool>,
    // Future: could add other overridable fields
}
```

#### 2. Update Config Loader (`src/config/loader.rs`)
```rust
pub async fn load_with_overrides(
    base_path: &Path,
    project_dir: &Path,
) -> Result<Config> {
    let mut config = load_from_path(base_path).await?;

    // Load overrides
    let override_path = project_dir.join(".mcp-proxy-overrides.json");
    if override_path.exists() {
        let overrides = load_overrides(&override_path).await?;
        apply_overrides(&mut config, overrides);
    }

    Ok(config)
}
```

#### 3. Update Server Tools (`src/proxy/server_tools.rs`)
```rust
async fn enable_server(server_name: String, state: Arc<AppState>) -> Result<Value> {
    // Enable in runtime
    state.enable_server(&server_name).await?;

    // Persist to overrides file
    persist_server_override(&server_name, true, &state).await?;

    // Clear cache
    if let Some(handler) = &state.request_handler {
        handler.clear_cache().await;
    }

    Ok(json!({"status": "enabled", "server": server_name}))
}

async fn persist_server_override(
    server_name: &str,
    enabled: bool,
    state: &AppState,
) -> Result<()> {
    let project_dir = detect_project_dir()?;
    let override_path = project_dir.join(".mcp-proxy-overrides.json");

    let mut overrides = if override_path.exists() {
        load_overrides(&override_path).await?
    } else {
        ServerOverrides {
            project: project_dir.clone(),
            overrides: HashMap::new(),
            last_modified: Utc::now(),
        }
    };

    overrides.overrides.insert(
        server_name.to_string(),
        ServerOverride { enabled: Some(enabled) }
    );
    overrides.last_modified = Utc::now();

    save_overrides(&override_path, &overrides).await?;
    Ok(())
}
```

#### 4. Project Directory Detection
```rust
fn detect_project_dir() -> Result<PathBuf> {
    // Try environment variable first
    if let Ok(project) = env::var("MCP_PROXY_PROJECT_DIR") {
        return Ok(PathBuf::from(project));
    }

    // Try current working directory
    env::current_dir()
        .map_err(|e| ConfigError::Parse(format!("Failed to get CWD: {}", e)))
}
```

### Migration Strategy

**For existing deployments:**
1. No action needed - overrides file is optional
2. First enable/disable call creates `.mcp-proxy-overrides.json`
3. Base config remains source of truth until overridden

### Testing

**Test Case 1: Fresh Start**
1. Start proxy with `serena.enabled: true` in base config
2. Call `mcp__proxy__server__disable` with `server_name: "serena"`
3. Verify `.mcp-proxy-overrides.json` created with `serena: {enabled: false}`
4. Restart proxy
5. Verify serena is still disabled (override persisted)

**Test Case 2: Override Removal**
1. Delete `.mcp-proxy-overrides.json`
2. Restart proxy
3. Verify serena enabled state reverts to base config

**Test Case 3: Multiple Projects**
1. Start proxy in project A, disable serena
2. Start proxy in project B, enable serena
3. Switch back to project A
4. Verify serena is disabled in A, enabled in B

### Future Enhancements

**Possible additional overrides:**
- `max_restarts`
- `initialization_delay_ms`
- `health_check` settings
- Environment variables

**UI Integration:**
- Web UI shows override indicator (⚙️ icon)
- Ability to reset to base config
- Show diff between base and effective config

### Security Considerations

**Gitignore:**
Add to `.gitignore`:
```
.mcp-proxy-overrides.json
```

**Permissions:**
- Override file is user-writable only
- No sensitive data stored (just enabled flags)
- Project directory must be writable

### API Changes

**New Config Fields:**
```rust
pub struct Config {
    // Existing fields...

    /// Project directory for per-project overrides (optional)
    #[serde(default)]
    pub project_dir: Option<PathBuf>,

    /// Whether to use per-project overrides (default: true)
    #[serde(default = "default_true")]
    pub enable_project_overrides: bool,
}
```

**New State Fields:**
```rust
pub struct AppState {
    // Existing fields...

    /// Path to project overrides file
    pub overrides_path: Option<PathBuf>,

    /// Runtime overrides (separate from config)
    pub runtime_overrides: Arc<RwLock<ServerOverrides>>,
}
```

## Summary

This design allows:
- ✅ Base config sets defaults
- ✅ MCP tools override per-project
- ✅ Changes persist across restarts
- ✅ Multi-project support
- ✅ No breaking changes to existing configs
- ✅ Optional feature (can be disabled)
