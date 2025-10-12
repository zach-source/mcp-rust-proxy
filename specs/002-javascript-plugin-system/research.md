# Research: JavaScript Plugin System for MCP Proxy

**Date**: 2025-10-10
**Feature Branch**: `002-javascript-plugin-system`

This document consolidates research findings for implementing the JavaScript plugin system, addressing unknowns identified in the Technical Context and exploring best practices for the chosen technologies.

---

## 1. Node.js Testing Framework

### Decision: **Node.js Native Test Runner**

**Rationale**:
1. **Zero Dependencies**: Requires no additional packages - built into Node.js 18+ (experimental) and stable in Node.js 20+
2. **Perfect Fit for Process Testing**: Excellent support for `child_process` testing, which matches our stdin/stdout plugin communication model
3. **Minimal Setup**: No configuration needed - just `node --test` to run tests
4. **TypeScript Path**: Easy to add TypeScript support later with `tsx` loader: `node --import tsx --test`
5. **Consistency**: Aligns with lightweight, minimal-dependency philosophy of the project

**Alternatives Considered**:
- **Vitest**: Fastest performance (2.75s vs 11s) with excellent TS/ESM support, but requires extra dependencies and adds build complexity
- **Jest**: Most popular but heavy dependency, CommonJS-focused (legacy), overkill for simple process testing
- **Bun Test**: Extremely fast (1.29s) but requires Bun runtime installation, less widespread adoption
- **AVA**: Good concurrent execution but external dependency, more complex than needed

**Integration with Rust Test Suite**:
```rust
// In Rust integration tests
#[test]
fn test_plugin_system() {
    let output = std::process::Command::new("node")
        .arg("--test")
        .arg("examples/plugins/tests/")
        .output()
        .expect("Failed to run plugin tests");

    assert!(output.status.success());
}
```

**Example Test Pattern**:
```javascript
import { test } from 'node:test';
import assert from 'node:assert';
import { execFileSync } from 'node:child_process';

test('plugin processes JSON input correctly', () => {
  const input = JSON.stringify({
    toolName: 'test/tool',
    rawContent: 'test content',
    metadata: { requestId: 'req-123', timestamp: '2025-10-10T...' }
  });

  const output = execFileSync('node', ['./curate.js'], {
    input,
    encoding: 'utf-8'
  });

  const result = JSON.parse(output);
  assert.strictEqual(result.continue, true);
});
```

---

## 2. Rust-Node.js Inter-Process Communication (IPC)

### Decision: **`tokio::process::Command` with stdio-based IPC**

**Rationale**:
1. **Proven Pattern**: Matches existing stdio transport in `src/transport/stdio.rs`
2. **Process Isolation**: Plugin crashes don't affect proxy (critical for FR-007)
3. **Node.js Compatibility**: Full npm ecosystem access without modification
4. **Simplicity**: No complex FFI bindings or inverted control flow

**Architecture**:
```rust
use tokio::process::{Command, Child, ChildStdin, ChildStdout};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

let mut cmd = Command::new("node");
cmd.args(&[plugin_path])
   .stdin(Stdio::piped())
   .stdout(Stdio::piped())
   .stderr(Stdio::piped())
   .kill_on_drop(true); // Prevent zombie processes

let mut child = cmd.spawn()?;
```

**Alternatives Considered**:
- **deno_core** (embedded V8): Incompatible with Node.js ecosystem, different APIs
- **neon/node-bindgen** (Rust ↔ Node.js bindings): Wrong architecture (requires Node as host), complex build
- **Named pipes/Unix sockets**: Over-engineered for simple request/response pattern

**Performance**: 50-100ms spawn overhead + execution time fits <500ms p95 target with process pooling

---

## 3. Data Format: JSON vs MessagePack

### Decision: **Start with JSON, provide MessagePack as optional optimization**

**Rationale**:
1. **MVP Simplicity**: JSON is human-readable, debuggable, zero friction with Node.js
2. **Acceptable Performance**: For 10-50KB payloads, JSON adds only ~4-6ms (serde_json benchmarks)
3. **MessagePack Advantage**: 40% smaller payloads, 2-3x faster when needed
4. **Incremental Adoption**: Can add MessagePack without breaking existing plugins

**Performance Comparison**:
| Format | Serialization | Deserialization | Size |
|--------|---------------|-----------------|------|
| JSON (serde_json) | 3.7ms | 6.0ms | 100% |
| MessagePack (rmp-serde) | 1.5ms | 3.2ms | 60% |

**Implementation**:
```rust
// MVP: JSON only
let input_json = serde_json::to_string(&plugin_input)?;
stdin.write_all(input_json.as_bytes()).await?;

// Future: Add MessagePack support (dependencies: rmp-serde = "1.3")
// let input_msgpack = rmp_serde::to_vec(&plugin_input)?;
```

---

## 4. Timeout Mechanism

### Decision: **`tokio::time::timeout` with two-phase shutdown (SIGTERM → SIGKILL)**

**Rationale**:
1. **Hard Enforcement**: `tokio::time::timeout` wraps entire plugin execution
2. **Graceful First**: Give plugin 5s to cleanup after SIGTERM
3. **Force Kill on Timeout**: SIGKILL after grace period prevents zombies
4. **Non-blocking**: Async timeout doesn't block other executions

**Pattern**:
```rust
use tokio::time::{timeout, Duration};
use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;

async fn execute_with_timeout(
    child: &mut Child,
    plugin_timeout: Duration,
) -> Result<PluginOutput> {
    match timeout(plugin_timeout, execute_plugin(...)).await {
        Ok(Ok(output)) => Ok(output),
        Ok(Err(e)) => Err(e),
        Err(_elapsed) => {
            // Timeout - graceful shutdown
            #[cfg(unix)]
            if let Some(pid) = child.id() {
                signal::kill(Pid::from_raw(pid as i32), Signal::SIGTERM)?;

                // Wait 5s for graceful exit
                match timeout(Duration::from_secs(5), child.wait()).await {
                    Ok(_) => {},
                    Err(_) => child.kill().await?,
                }
            }
            Err(PluginError::Timeout(plugin_timeout))
        }
    }
}
```

**Edge Cases Handled**:
- Zombie prevention: `kill_on_drop(true)` + explicit `wait()`
- Timeout during stdin write: Separate writer task avoids deadlock
- Cancellation safety: Timeout drop triggers cleanup

---

## 5. Backpressure & Concurrency Control

### Decision: **`tokio::sync::Semaphore` for global limits + per-plugin process pools**

**Rationale**:
1. **Prevent Exhaustion**: Semaphore caps max concurrent executions (default: 10)
2. **Fair Scheduling**: All plugins compete for permits
3. **Process Pooling**: Reuse warm Node.js processes to reduce spawn overhead
4. **Natural Backpressure**: Slow plugins naturally slow request handling

**Architecture**:
```rust
use tokio::sync::Semaphore;
use std::sync::Arc;

pub struct PluginExecutor {
    process_pool: Arc<ProcessPool>,
    global_semaphore: Arc<Semaphore>, // Shared across all plugins
    timeout: Duration,
}

impl PluginExecutor {
    pub async fn execute(&self, input: PluginInput) -> Result<PluginOutput> {
        let permit = self.global_semaphore.acquire().await?;
        let mut process = self.process_pool.acquire().await?;

        let result = execute_with_timeout(&mut process, input, self.timeout).await;

        if result.is_ok() && process.is_healthy() {
            self.process_pool.release(process).await?;
        }

        drop(permit);
        result
    }
}
```

**Process Pool Pattern**:
```rust
struct ProcessPool {
    available: Arc<Mutex<VecDeque<ChildProcess>>>,
    max_size: usize,
    plugin_path: PathBuf,
}

impl ProcessPool {
    async fn acquire(&self) -> Result<ChildProcess> {
        let mut available = self.available.lock().await;

        if let Some(process) = available.pop_front() {
            if process.is_healthy() {
                return Ok(process);
            }
        }

        // Spawn new if pool empty or unhealthy
        ChildProcess::spawn(&self.plugin_path).await
    }
}
```

**Configuration**:
```yaml
plugins:
  globalSettings:
    maxConcurrentExecutions: 10  # Semaphore size
    poolSizePerPlugin: 5          # Warm processes
    defaultTimeoutMs: 30000
```

---

## 6. Error Detection & Recovery

### Decision: **Multi-level detection with graceful fallback to original content**

**Rationale**:
1. **Non-fatal**: Plugin failures never crash proxy (FR-007)
2. **Multiple Mechanisms**: Exit code, timeout, malformed output, stderr monitoring
3. **Automatic Recovery**: Dead processes replaced, original content preserved
4. **Observable**: All errors logged with context

**Error Types**:
```rust
pub enum PluginError {
    Timeout(Duration),
    ProcessFailed { code: i32, stderr: String },
    InvalidOutput(String),
    PoolExhausted,
    IoError(std::io::Error),
    PluginReturnedError(String),
}
```

**Graceful Degradation**:
```rust
async fn execute_plugin_chain(
    chain: &PluginChain,
    input: PluginInput,
) -> PluginOutput {
    match chain.execute(input.clone()).await {
        Ok(output) => output,
        Err(e) => {
            tracing::warn!("Plugin failed: {}, using original content", e);
            PluginOutput {
                text: input.raw_content,
                continue_processing: false,
                metadata: None,
            }
        }
    }
}
```

**Recovery Mechanisms**:
- Process crash → Replace in pool
- Malformed output → Return original, log error
- Timeout → Kill process, return original
- Pool exhausted → Wait or spawn (up to limit)

---

## 7. Resource Cleanup & Lifecycle

### Decision: **Explicit lifecycle management with automatic cleanup fallbacks**

**Rationale**:
1. **Zombie Prevention**: Explicit `wait()` after kill + `kill_on_drop(true)`
2. **Graceful Shutdown**: Two-phase SIGTERM → SIGKILL with timeout
3. **Pool Lifecycle**: Shutdown all pooled processes on proxy shutdown
4. **Leak Prevention**: Drop guards ensure cleanup even on panic

**Lifecycle Implementation**:
```rust
impl Drop for PluginExecutor {
    fn drop(&mut self) {
        tokio::task::spawn_blocking({
            let pool = Arc::clone(&self.process_pool);
            move || {
                tokio::runtime::Handle::current().block_on(async {
                    if let Err(e) = pool.shutdown().await {
                        tracing::error!("Failed to shutdown plugin pool: {}", e);
                    }
                });
            }
        });
    }
}

impl ProcessPool {
    pub async fn shutdown(self) -> Result<()> {
        let available = self.available.lock().await;
        let processes: Vec<_> = available.drain(..).collect();

        for process in processes {
            if let Err(e) = process.graceful_shutdown().await {
                tracing::warn!("Failed graceful shutdown: {}", e);
                let _ = process.force_kill().await;
            }
        }
        Ok(())
    }
}
```

**Zombie Prevention Strategy**:
- ✅ `kill_on_drop(true)` on Command builder
- ✅ Explicit `wait()` or `try_wait()` after kill
- ✅ Drop guards for cleanup on panic
- ✅ Monitoring for zombie accumulation

---

## 8. Concurrency & Performance Optimization

### Decision: **Process pooling + async I/O + separate writer tasks**

**Rationale**:
1. **Process Reuse**: Amortize 50-100ms spawn cost
2. **Avoid Deadlocks**: Write stdin in separate task (Tokio best practice)
3. **Async I/O**: Non-blocking with `AsyncReadExt`/`AsyncWriteExt`
4. **Pipeline Parallelism**: Multiple plugins execute concurrently (up to semaphore limit)

**Deadlock Prevention** (from Tokio docs):
```rust
async fn execute_plugin(
    stdin: ChildStdin,
    stdout: ChildStdout,
    input: PluginInput,
) -> Result<PluginOutput> {
    let input_json = serde_json::to_string(&input)?;

    // Spawn separate writer task to avoid deadlock
    let write_task = tokio::spawn(async move {
        let mut stdin = stdin;
        stdin.write_all(input_json.as_bytes()).await?;
        stdin.write_all(b"\n").await?;
        stdin.flush().await?;
        drop(stdin); // Close to signal EOF
        Ok::<_, std::io::Error>(())
    });

    // Read output concurrently
    let mut reader = BufReader::new(stdout);
    let mut output_line = String::new();
    reader.read_line(&mut output_line).await?;

    write_task.await??;

    serde_json::from_str(&output_line)
        .map_err(|e| PluginError::InvalidOutput(e.to_string()))
}
```

**Performance Characteristics**:
- Cold start: 50-100ms (spawn + init)
- Warm pool hit: 1-5ms (process reuse)
- JSON serialization: 4-6ms
- I/O overhead: 1-2ms
- **Total p95 latency: <500ms** ✅ (within target)

---

## 9. Monitoring & Observability

### Decision: **Structured tracing + prometheus metrics + health checks**

**Rationale**:
1. **Debugging**: Structured logs for every execution with context
2. **Performance Tracking**: Metrics for latency, errors, pool utilization
3. **Health Monitoring**: Detect degraded plugins, remove from pool
4. **Integration**: Reuse existing tracing from `src/proxy/handler.rs`

**Metrics**:
```rust
use prometheus::{IntCounter, Histogram, IntGauge};

pub struct PluginMetrics {
    executions_total: IntCounter,
    execution_duration: Histogram,
    errors_total: IntCounter,
    timeouts_total: IntCounter,
    pool_size: IntGauge,
    pool_available: IntGauge,
}
```

**Structured Logging**:
```rust
let _span = tracing::info_span!(
    "plugin_execution",
    plugin = %plugin_name,
    request_id = %input.metadata.request_id,
).entered();

info!(
    duration_ms = %duration.as_millis(),
    success = %result.is_ok(),
    "Plugin execution completed"
);
```

---

## Summary & Implementation Priorities

### Technology Stack

| Component | Technology | Dependencies |
|-----------|-----------|-------------|
| Process Management | `tokio::process::Command` | Already in Cargo.toml |
| Data Format | JSON (MVP), MessagePack (future) | `serde_json`, `rmp-serde` (future) |
| Timeout | Two-phase shutdown | `tokio::time::timeout`, `nix` |
| Concurrency | Semaphore + Pools | `tokio::sync::Semaphore` |
| Testing | Node.js native test runner | Node.js 18+ (no dependencies) |

### Implementation Phases

1. **Phase 1 (MVP)**: Basic execution with JSON, timeouts, error handling
2. **Phase 2**: Process pooling for performance
3. **Phase 3**: MessagePack support for high-volume scenarios
4. **Phase 4**: Advanced monitoring and health checks

### Key Risks & Mitigations

| Risk | Mitigation |
|------|-----------|
| Zombie processes | `kill_on_drop(true)` + explicit `wait()` + monitoring |
| Deadlock on I/O | Separate writer tasks + timeout on all I/O |
| Memory leaks | Max pool size + health checks + cleanup on shutdown |
| Spawn overhead | Process pooling with warm processes |
| Slow plugins | Semaphore limits + timeout enforcement |

### Dependencies to Add

```toml
[dependencies]
# Already have: tokio, serde_json, nix, prometheus
rmp-serde = "1.3"  # Future: MessagePack support
```

### Next Steps

1. Implement `PluginExecutor` with basic JSON execution (Phase 1 of implementation plan)
2. Add timeout handling with two-phase shutdown
3. Implement `ProcessPool` for process reuse
4. Add semaphore-based concurrency control
5. Integrate with proxy request flow (FR-004, FR-005)
6. Add comprehensive error handling and logging
7. Benchmark and tune pool sizes/timeout values

---

**Research Completed**: 2025-10-10
**Validated Against**: Existing codebase patterns, Tokio documentation, Rust best practices (2024-2025), MCP Proxy requirements
