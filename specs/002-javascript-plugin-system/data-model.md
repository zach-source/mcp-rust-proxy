# Data Model: JavaScript Plugin System

**Feature Branch**: `002-javascript-plugin-system`
**Date**: 2025-10-10

This document defines the core data entities, relationships, validation rules, and state transitions for the JavaScript plugin system.

---

## Core Entities

### 1. Plugin

Represents a JavaScript module that processes MCP requests or responses.

**Fields**:
| Field | Type | Required | Description | Validation |
|-------|------|----------|-------------|------------|
| `id` | `String` | Yes | Unique identifier (derived from file path hash) | Must be unique within system |
| `name` | `String` | Yes | Human-readable name | Must match filename without extension |
| `path` | `PathBuf` | Yes | Absolute path to plugin .js file | Must exist, be readable, end in .js |
| `phase` | `PluginPhase` | Yes | Execution phase (request/response) | Must be valid enum variant |
| `server_filter` | `Vec<String>` | No | MCP server names to apply plugin to | Empty = all servers |
| `tool_filter` | `Vec<String>` | No | Tool names to apply plugin to | Empty = all tools |
| `timeout_ms` | `u64` | No | Override default timeout | Must be > 0, ≤ 600000 (10 min) |
| `enabled` | `bool` | Yes | Whether plugin is active | Default: true |
| `order` | `u32` | Yes | Execution order within phase | Lower executes first |

**Relationships**:
- Belongs to: PluginChain (many-to-many via PluginChainEntry)
- Has many: PluginExecution (execution history)

**State Transitions**:
```
[Registered] → [Enabled] → [Executing] → [Completed]
                   ↓            ↓
              [Disabled]    [Failed]
```

**Uniqueness Rules**:
- Plugin `id` must be unique globally
- Plugin `name` + `phase` combination must be unique per server

---

### 2. PluginConfiguration

Settings that control plugin loading and execution behavior.

**Fields**:
| Field | Type | Required | Description | Validation |
|-------|------|----------|-------------|------------|
| `plugin_dir` | `PathBuf` | Yes | Directory containing plugin .js files | Must be readable directory |
| `max_concurrent_executions` | `u32` | No | Global concurrency limit (semaphore) | Default: 10, range: 1-100 |
| `pool_size_per_plugin` | `u32` | No | Warm processes to maintain | Default: 5, range: 0-20 |
| `default_timeout_ms` | `u64` | No | Default plugin timeout | Default: 30000, range: 100-600000 |
| `node_executable` | `PathBuf` | No | Path to Node.js binary | Default: "node", must be executable |
| `server_assignments` | `HashMap<String, Vec<String>>` | No | Server → plugin names mapping | Plugin names must exist |

**Relationships**:
- Has many: Plugin (via plugin_dir discovery)

**Validation Rules**:
- `plugin_dir` must exist and be readable
- `max_concurrent_executions` prevents resource exhaustion
- `pool_size_per_plugin` must be less than `max_concurrent_executions`
- All plugin names in `server_assignments` must reference existing plugins

---

### 3. PluginInput

Structured data passed to plugin processes via stdin.

**Fields**:
| Field | Type | Required | Description | Validation |
|-------|------|----------|-------------|------------|
| `toolName` | `String` | Yes | Name of MCP tool being invoked | Non-empty |
| `rawContent` | `String` | Yes | Original request/response content | Can be empty |
| `maxTokens` | `Option<u32>` | No | Token limit for curation plugins | If present, must be > 0 |
| `metadata` | `PluginMetadata` | Yes | Execution context | All required fields must be present |

**Nested: PluginMetadata**:
| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `requestId` | `String` | Yes | Unique request identifier |
| `timestamp` | `String` (ISO 8601) | Yes | Execution timestamp |
| `serverName` | `String` | Yes | Name of MCP server |
| `phase` | `"request"` \| `"response"` | Yes | Execution phase |
| `userQuery` | `Option<String>` | No | Original user query (if available) |

**Serialization**:
- Format: JSON (MVP) or MessagePack (future)
- Encoding: UTF-8 for JSON, binary for MessagePack
- Newline termination: Required for JSON (enables line-based reading)

**Validation Rules**:
- All required fields must be present and non-null
- `timestamp` must be valid ISO 8601 format
- `phase` must be exactly "request" or "response"
- `maxTokens` if present must be positive integer

---

### 4. PluginOutput

Structured data returned by plugin processes via stdout.

**Fields**:
| Field | Type | Required | Description | Validation |
|-------|------|----------|-------------|------------|
| `text` | `String` | Yes | Modified content | Can be empty |
| `continue` | `bool` | Yes | Whether to continue plugin chain | Must be boolean |
| `metadata` | `Option<serde_json::Value>` | No | Plugin-specific metadata | Any valid JSON |
| `error` | `Option<String>` | No | Error message if plugin failed | If present, `text` should be original |

**Serialization**:
- Format: JSON (MVP) or MessagePack (future)
- Encoding: UTF-8 for JSON, binary for MessagePack
- Newline termination: Required for JSON

**Validation Rules**:
- Schema validation against expected structure
- If `error` is present:
  - `continue` must be `false`
  - `text` should contain original content or fallback
- If `continue` is `false`, no further plugins in chain execute

**Error Handling**:
- Malformed JSON → `PluginError::InvalidOutput`
- Missing required fields → `PluginError::InvalidOutput`
- If `error` field present → graceful degradation (return original content)

---

### 5. PluginChain

An ordered sequence of plugins applied to a specific MCP server or tool.

**Fields**:
| Field | Type | Required | Description | Validation |
|-------|------|----------|-------------|------------|
| `id` | `String` | Yes | Unique identifier | Auto-generated UUID |
| `server_name` | `String` | Yes | MCP server this chain applies to | Must match configured server |
| `phase` | `PluginPhase` | Yes | Request or response phase | Must be valid enum |
| `plugins` | `Vec<PluginChainEntry>` | Yes | Ordered list of plugins | Sorted by `order` field |

**Nested: PluginChainEntry**:
| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `plugin_id` | `String` | Yes | Reference to Plugin entity |
| `order` | `u32` | Yes | Execution order |
| `enabled` | `bool` | Yes | Whether this entry is active |

**Relationships**:
- Belongs to: MCP Server (one-to-many)
- Has many: Plugin (via PluginChainEntry)
- Has many: PluginExecution (execution history)

**Execution Logic**:
```
1. Filter enabled plugins
2. Sort by order (ascending)
3. Execute sequentially
4. If plugin returns continue=false, halt chain
5. If plugin errors, halt chain and return error
6. Pass output of plugin N as input to plugin N+1
```

**Validation Rules**:
- All `plugin_id` references must exist
- No duplicate plugin_id within same chain
- Order values need not be contiguous but must be sortable
- At least one plugin must be enabled for chain to execute

---

### 6. PluginExecution

Execution history record for observability and debugging.

**Fields**:
| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | `String` | Yes | Unique execution ID |
| `request_id` | `String` | Yes | Parent request identifier |
| `plugin_id` | `String` | Yes | Plugin that executed |
| `chain_id` | `String` | Yes | Chain this execution belongs to |
| `started_at` | `DateTime<Utc>` | Yes | Execution start time |
| `duration_ms` | `u64` | Yes | Execution duration |
| `status` | `ExecutionStatus` | Yes | Success/Failure/Timeout |
| `input_size_bytes` | `usize` | Yes | Input payload size |
| `output_size_bytes` | `Option<usize>` | No | Output payload size (if success) |
| `error_message` | `Option<String>` | No | Error details (if failed) |

**Enum: ExecutionStatus**:
```rust
enum ExecutionStatus {
    Success,
    Failed,
    Timeout,
    PoolExhausted,
}
```

**Relationships**:
- Belongs to: Plugin (many-to-one)
- Belongs to: PluginChain (many-to-one)

**Retention**:
- Store in-memory for recent executions (last 1000 per plugin)
- Optionally persist to database for audit trail
- Expose via metrics and logs

---

### 7. PluginProcess

Represents a running Node.js process in the process pool.

**Fields**:
| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `pid` | `u32` | Yes | Process ID |
| `plugin_id` | `String` | Yes | Plugin this process executes |
| `spawned_at` | `DateTime<Utc>` | Yes | When process was created |
| `last_used_at` | `DateTime<Utc>` | Yes | Last execution timestamp |
| `execution_count` | `u64` | Yes | Number of executions served |
| `is_healthy` | `bool` | Yes | Health check status |
| `stdin` | `ChildStdin` | Yes | Process stdin handle |
| `stdout` | `ChildStdout` | Yes | Process stdout handle |
| `stderr` | `ChildStderr` | Yes | Process stderr handle |

**Lifecycle**:
```
[Spawned] → [Available] → [Executing] → [Available]
                               ↓
                          [Unhealthy] → [Terminated]
```

**Health Check**:
- Process still running (via `try_wait()`)
- Execution count < max (default: 1000)
- Age < max lifetime (default: 1 hour)
- Last stderr output < threshold

**Cleanup Triggers**:
- Health check fails → Remove from pool, spawn new
- Max executions reached → Graceful shutdown, spawn new
- Max lifetime exceeded → Graceful shutdown, spawn new
- Pool shutdown → Terminate all processes

---

## Entity Relationships (ER Diagram)

```
PluginConfiguration
    ↓ (1:N)
Plugin ←→ PluginChain (N:M via PluginChainEntry)
  ↓ (1:N)                ↓ (1:N)
PluginProcess      PluginExecution
```

---

## Data Validation Summary

### Input Validation (PluginInput)
✅ All required fields present
✅ `toolName` non-empty string
✅ `maxTokens` if present is positive
✅ `timestamp` valid ISO 8601
✅ `phase` is "request" or "response"

### Output Validation (PluginOutput)
✅ Valid JSON/MessagePack structure
✅ Required fields: `text`, `continue`
✅ If `error` present, `continue` must be false
✅ `metadata` is valid JSON (if present)

### Configuration Validation (PluginConfiguration)
✅ `plugin_dir` exists and is readable
✅ `max_concurrent_executions` in range [1, 100]
✅ `pool_size_per_plugin` < `max_concurrent_executions`
✅ `default_timeout_ms` in range [100, 600000]
✅ All plugin names in `server_assignments` exist

### Plugin Validation
✅ `path` points to existing .js file
✅ `id` is globally unique
✅ `name` + `phase` unique per server
✅ `timeout_ms` in valid range
✅ `order` is valid u32

---

## State Machine: Plugin Execution Flow

```
[Request Received]
        ↓
[Match Plugins to Server]
        ↓
[Filter by Tool Name] (if tool_filter set)
        ↓
[Sort by Order]
        ↓
[Acquire Semaphore Permit]
        ↓
[Get Process from Pool]
        ↓
[Serialize PluginInput]
        ↓
[Write to Process stdin] ──timeout→ [Kill Process, Return Error]
        ↓
[Read from Process stdout] ──timeout→ [Kill Process, Return Error]
        ↓
[Deserialize PluginOutput]
        ↓
  [Valid Output?] ──No→ [Return Original Content]
        ↓ Yes
  [Error Present?] ──Yes→ [Return Original Content]
        ↓ No
  [Continue=true?] ──No→ [Stop Chain, Return Current Output]
        ↓ Yes
[Pass Output to Next Plugin]
        ↓
[Release Permit, Return Process to Pool]
```

---

## Indexing & Performance

### Primary Keys
- `Plugin.id`: Hash index for O(1) lookup
- `PluginChain.id`: Hash index
- `PluginExecution.id`: Hash index

### Secondary Indexes
- `Plugin.name`: For user-friendly lookups
- `Plugin.server_filter`: For server matching
- `PluginChain.server_name`: For request routing
- `PluginExecution.request_id`: For tracing

### In-Memory Storage
- Use `DashMap` for concurrent access (already in codebase)
- `Arc<DashMap<String, Plugin>>` for plugins
- `Arc<DashMap<String, PluginChain>>` for chains
- `VecDeque<PluginProcess>` for process pools (behind Arc<Mutex>)

---

## Data Flow Example

**Request Flow (Documentation Curation)**:
```
1. User → MCP Proxy: "Explain React hooks"
2. Proxy → Context7 Server: [request]
3. Context7 → Proxy: [50KB response]
4. Proxy matches server "context7" → PluginChain (response phase)
5. Chain has 1 plugin: "curation-plugin"
6. Acquire semaphore permit (9/10 available)
7. Get process from pool for "curation-plugin"
8. Serialize PluginInput:
   {
     "toolName": "context7/get-docs",
     "rawContent": "<50KB docs>",
     "maxTokens": 1200,
     "metadata": {
       "requestId": "req-xyz",
       "timestamp": "2025-10-10T12:00:00Z",
       "serverName": "context7",
       "phase": "response",
       "userQuery": "Explain React hooks"
     }
   }
9. Write to plugin stdin
10. Plugin runs curation logic (uses Claude SDK internally)
11. Read from plugin stdout:
   {
     "text": "<10KB curated docs>",
     "continue": true,
     "metadata": {"tokensUsed": 1150}
   }
12. No more plugins in chain
13. Return process to pool
14. Release semaphore permit
15. Proxy → User: [10KB curated response]
```

---

## Migration & Versioning

**V1 (MVP)**:
- JSON-only serialization
- File-based plugin discovery (scan plugin_dir)
- In-memory execution history (last 1000)

**V2 (Future)**:
- MessagePack support for performance
- Database-backed plugin registry
- Persistent execution audit trail
- Hot-reloading of plugin code

**Backward Compatibility**:
- All V1 plugins work in V2 (JSON always supported)
- Config schema versioning for smooth upgrades
- Graceful handling of unknown fields (forward compatibility)

---

**Last Updated**: 2025-10-10
**Schema Version**: 1.0.0
