# Implementation Plan: JavaScript Plugin System for MCP Proxy

**Branch**: `002-javascript-plugin-system` | **Date**: 2025-10-10 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/002-javascript-plugin-system/spec.md`

**Note**: This template is filled in by the `/speckit.plan` command. See `.specify/templates/commands/plan.md` for the execution workflow.

## Summary

Implement a JavaScript plugin system that enables end users to run custom middleware for different MCP servers. Plugins run in isolated Node.js processes and can intercept/modify requests and responses using a standardized JSON/MessagePack I/O contract. The primary use case is content curation for documentation tools to reduce token usage by 60-80%, with additional support for security middleware and response transformation. The system uses a fail-fast error strategy where any plugin failure causes the entire request to fail with detailed error information.

## Technical Context

**Language/Version**: Rust 1.75+ (proxy), Node.js 18+ (plugin runtime)
**Primary Dependencies**: tokio (async runtime), serde_json/rmp-serde (serialization), nix (process management), warp (HTTP), MessagePack (binary protocol)
**Storage**: Plugin configuration in YAML/JSON/TOML (via figment), execution metrics in existing prometheus setup
**Testing**: cargo test (Rust unit/integration), Node.js native test runner (node --test) for plugin examples
**Target Platform**: Linux/macOS servers (same as existing MCP proxy)
**Project Type**: Single project (extending existing Rust proxy codebase)
**Performance Goals**: <500ms plugin execution latency (p95), support up to 5 chained plugins per request
**Constraints**: Process-based isolation (separate Node.js process per plugin), fail-fast error handling, 30s default timeout per plugin
**Scale/Scope**: Support 10+ concurrent plugin processes, handle plugin I/O up to 50KB (typical doc responses)

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

**Status**: No constitution file exists yet - proceeding with general best practices validation.

### General Architecture Gates (without formal constitution)

✅ **Modularity**: Plugin system is isolated module extending existing proxy - no existing code modification required
✅ **Testability**: Clear contracts for plugin I/O enable unit and integration testing
✅ **Error Handling**: Fail-fast strategy ensures no silent failures, all errors logged with context
✅ **Performance**: Explicit latency targets (<500ms p95) and timeout mechanisms prevent degradation
✅ **Security**: Process isolation provides basic security boundary (though sandboxing explicitly out of scope)

**Note**: Once project constitution is established, this section should be re-evaluated against formal principles.

## Project Structure

### Documentation (this feature)

```
specs/[###-feature]/
├── plan.md              # This file (/speckit.plan command output)
├── research.md          # Phase 0 output (/speckit.plan command)
├── data-model.md        # Phase 1 output (/speckit.plan command)
├── quickstart.md        # Phase 1 output (/speckit.plan command)
├── contracts/           # Phase 1 output (/speckit.plan command)
└── tasks.md             # Phase 2 output (/speckit.tasks command - NOT created by /speckit.plan)
```

### Source Code (repository root)

```
src/
├── plugin/                    # NEW: Plugin system implementation
│   ├── mod.rs                # Module entry point
│   ├── manager.rs            # Plugin lifecycle management
│   ├── process.rs            # Node.js process spawning & IPC
│   ├── schema.rs             # Plugin I/O schema definitions
│   ├── chain.rs              # Plugin chaining logic
│   └── config.rs             # Plugin configuration parsing
├── proxy/                     # EXISTING: Extend for plugin hooks
│   ├── handler.rs            # Add plugin middleware hooks
│   └── router.rs             # Route to plugin-enabled servers
├── config/                    # EXISTING: Extend schema
│   └── schema.rs             # Add PluginConfig types
└── transport/                 # EXISTING: No changes needed

tests/
├── plugin/                    # NEW: Plugin system tests
│   ├── unit/
│   │   ├── schema_tests.rs
│   │   └── chain_tests.rs
│   └── integration/
│       ├── process_ipc_test.rs
│       └── end_to_end_test.rs
└── fixtures/                  # NEW: Test plugins
    └── plugins/
        ├── echo.js           # Simple test plugin
        └── curation.js       # Example curation plugin

examples/                      # NEW: Example plugins for users
├── plugins/
│   ├── curation-plugin.js    # Reference implementation
│   ├── security-plugin.js    # Security middleware example
│   └── package.json          # Plugin dependencies template
└── configs/
    └── plugin-config.yaml    # Example configuration
```

**Structure Decision**: Extending existing single-project Rust codebase with new `plugin/` module. Plugin examples live in `examples/` directory for user reference. Test plugins in `tests/fixtures/` for integration testing.

## Complexity Tracking

*Fill ONLY if Constitution Check has violations that must be justified*

**Status**: No constitution violations - complexity is justified by requirements.

Note: Process-based isolation adds operational complexity but is necessary for:
- Plugin fault isolation (FR-014)
- Independent dependency management (per clarifications)
- Fail-fast error handling without proxy crashes (FR-007)

---

## Planning Artifacts Generated

### Phase 0: Research (✅ Complete)
- **[research.md](./research.md)**: Comprehensive research on:
  - Node.js testing framework decision (Native Test Runner)
  - Rust-Node.js IPC patterns (tokio::process)
  - Data format comparison (JSON vs MessagePack)
  - Timeout mechanisms (two-phase SIGTERM → SIGKILL)
  - Backpressure & concurrency control (Semaphore + Process Pools)
  - Error detection & recovery strategies
  - Resource cleanup & lifecycle management
  - Performance optimization techniques
  - Monitoring & observability approach

### Phase 1: Design & Contracts (✅ Complete)
- **[data-model.md](./data-model.md)**: Complete entity definitions:
  - 7 core entities (Plugin, PluginConfiguration, PluginInput, PluginOutput, PluginChain, PluginExecution, PluginProcess)
  - Entity relationships and ER diagram
  - State machines and lifecycle flows
  - Validation rules and constraints
  - Data flow examples

- **[contracts/plugin-api.md](./contracts/plugin-api.md)**: API contract specification:
  - Input/Output JSON schemas
  - 3 example plugins (echo, curation, security)
  - Unit test examples with Node.js test runner
  - Configuration schema (YAML/JSON)
  - Error handling semantics
  - Best practices and troubleshooting

- **[quickstart.md](./quickstart.md)**: User-friendly getting started guide:
  - 5-minute setup walkthrough
  - Echo plugin tutorial
  - AI-powered curation plugin example
  - Common patterns (chaining, security, conditional processing)
  - Testing, debugging, and troubleshooting tips
  - Production deployment checklist

## Next Steps

### Phase 2: Task Generation (Use `/speckit.tasks`)
The next command to run is `/speckit.tasks` which will:
1. Generate dependency-ordered implementation tasks in `tasks.md`
2. Break down each feature into atomic, testable units
3. Assign priorities based on critical path
4. Map tasks to the project structure defined above

### Implementation Roadmap
Based on research findings, implementation will proceed in these phases:

**MVP (Phase 1)**:
- Basic plugin execution with JSON I/O
- Process spawning and timeout handling
- Error detection and graceful degradation
- Plugin configuration parsing

**Phase 2**:
- Process pooling for performance
- Plugin chain execution
- Metrics and observability

**Phase 3** (Future):
- MessagePack binary format support
- Hot-reloading of plugins
- Advanced health checks and monitoring

## Key Decisions Summary

| Decision Point | Resolution | Rationale |
|---------------|------------|-----------|
| **Testing Framework** | Node.js Native Test Runner | Zero dependencies, perfect for process testing, matches existing pattern |
| **IPC Mechanism** | tokio::process with stdio | Proven pattern in codebase, process isolation, simple request/response |
| **Data Format** | JSON (MVP), MessagePack (future) | Human-readable for debugging, binary option for performance |
| **Timeout Strategy** | Two-phase SIGTERM → SIGKILL | Graceful shutdown first, force kill as fallback |
| **Concurrency Control** | Semaphore + Process Pools | Prevents exhaustion, amortizes spawn costs |
| **Error Handling** | Fail-fast with graceful degradation | All errors caught, original content preserved |
| **Lifecycle Management** | Explicit cleanup + drop guards | Prevents zombies, ensures resource cleanup |

---

**Planning Phase Complete**: All research, design, and contract artifacts generated.
**Branch**: `002-javascript-plugin-system`
**Ready for**: Task generation via `/speckit.tasks`
