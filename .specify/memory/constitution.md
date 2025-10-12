<!--
Sync Impact Report: Constitution v1.0.0 (Initial Ratification)

Version Change: None → 1.0.0 (Initial creation)
Rationale: MINOR version - First constitution establishing project governance

Modified Principles: None (initial creation)

Added Sections:
- Core Principles (7 principles)
- Performance Standards
- Development Workflow
- Governance

Removed Sections: None

Templates Requiring Updates:
- ✅ plan-template.md - Constitution Check section references this file
- ✅ spec-template.md - No changes needed (already technology-agnostic)
- ✅ tasks-template.md - No changes needed (testing emphasis already present)
- ✅ commands/*.md - No changes needed (agent-agnostic)

Follow-up TODOs: None
-->

# MCP Rust Proxy Constitution

## Core Principles

### I. Performance First (NON-NEGOTIABLE)

Rust was chosen for this project explicitly for speed, memory safety, and zero-cost abstractions. Every feature MUST be designed and implemented with performance as a primary constraint.

**Requirements:**
- Use `Arc<DashMap>` for concurrent state (lock-free reads)
- Prefer zero-copy operations where possible (bytes::Bytes, Arc cloning)
- Async operations with tokio (no blocking in async contexts)
- Benchmark critical paths (< 1ms P99 for protocol translation, < 100ms for initialization)
- Profile before optimizing, but design for performance from the start

**Rationale**: The proxy sits between clients and backend servers, handling high message throughput. Performance degradation directly impacts user experience and server capacity.

### II. Flexibility Over Rigidity

The MCP ecosystem evolves rapidly with new protocol versions, transport types, and server implementations. The proxy MUST adapt without requiring architectural rewrites.

**Requirements:**
- Support multiple protocol versions simultaneously (2024-11-05, 2025-03-26, 2025-06-18)
- Support all transport types (stdio, HTTP-SSE, WebSocket) without preference
- Plugin system for extensibility (JavaScript plugins for content processing)
- Configuration-driven behavior (YAML/JSON/TOML)
- Graceful degradation when backend servers have limited capabilities

**Rationale**: Backend servers use different protocol versions, transport mechanisms, and capabilities. Forcing uniformity breaks real-world deployments.

### III. Comprehensive Testing (NON-NEGOTIABLE)

Every feature MUST have unit tests, integration tests, and protocol compliance tests before merging. Tests are not optional.

**Requirements:**
- Unit tests for all public functions and critical private functions
- Integration tests for multi-server scenarios and protocol version combinations
- Protocol compliance tests against MCP specifications
- Mock servers for testing edge cases and failure modes
- Tests MUST pass before merge (no `--no-verify`, no disabled tests)
- Code coverage tracking (aim for 80%+ on critical paths)

**Rationale**: The proxy manages critical infrastructure (backend MCP servers). Bugs cause production outages, data loss, or security issues. Testing catches issues before deployment.

### IV. Idiomatic Rust Patterns

Follow Rust community conventions and leverage the type system for correctness. Rust's safety guarantees are a core value proposition.

**Requirements:**
- Use `Result<T, Error>` for fallible operations (no panics in library code)
- Prefer enums for state (ProtocolVersion, TransportType, ServerState)
- Use traits for polymorphism (Transport, Connection, ProtocolAdapter)
- Leverage the borrow checker (minimize clones, use Arc only when needed)
- Follow naming conventions (snake_case for functions, PascalCase for types)
- **cargo fmt** for code formatting (enforced pre-commit)
- Use thiserror for error types, tracing for logging

**Rationale**: Rust's type system catches bugs at compile time. Idiomatic code is more maintainable and easier for other Rust developers to understand.

### V. Structured Logging and Observability

Production debugging requires detailed, queryable logs. The proxy MUST provide visibility into all operations.

**Requirements:**
- Use `tracing` crate for all logging (not println!)
- Log levels: debug (development), info (key events), warn (degraded), error (failures)
- File-based logging for backend servers (rotating logs, 2-day retention)
- Structured fields for correlation (server_name, protocol_version, request_id)
- Log protocol version mismatches and initialization sequence
- Expose metrics via API (/api/metrics, /api/server/{name}/status)

**Rationale**: When production issues occur (servers crash, tools missing, protocol errors), logs are the primary diagnostic tool. Structured logging enables fast root cause analysis.

### VI. Backward Compatibility

Protocol changes MUST NOT break existing deployments. Backend servers and clients may lag behind the latest version.

**Requirements:**
- Support at least 3 MCP protocol versions simultaneously
- Graceful handling of unknown protocol versions (log warning, attempt pass-through)
- Configuration format changes MUST be backward compatible
- API changes MUST use versioned endpoints or opt-in flags
- Document breaking changes prominently in CHANGELOG

**Rationale**: Users have diverse backend servers running different protocol versions. Forcing upgrades creates deployment friction and breaks workflows.

### VII. Leverage Context7 and Serena for Development

Use the MCP ecosystem's own tools to improve development velocity and code quality.

**Requirements:**
- Use Context7 for library documentation (mcp__proxy__context7__get-library-docs)
- Use Serena for semantic code navigation (mcp__proxy__serena__list_dir, mcp__proxy__serena__get_current_config)
- Document usage patterns in quickstart guides
- Integrate into development workflow (not just ad-hoc usage)
- Keep Context7 library IDs updated in documentation

**Rationale**: These tools exist to solve real problems (finding documentation, understanding codebases). Using them validates their value and improves our own productivity.

## Performance Standards

**Latency Requirements**:
- Protocol version detection: < 100ms per server
- Message translation: < 1ms P99 latency
- Pass-through (same version): < 50μs overhead
- Tools/list aggregation: < 500ms for 9 servers
- Initialization handshake: < 60 seconds per server (timeout)

**Throughput Requirements**:
- Support 9+ backend servers concurrently
- Handle 10-100 messages/second per server
- Aggregate 76-86 tools from all servers
- Maintain persistent connections (no reconnect overhead)

**Resource Limits**:
- Memory: < 100MB base + 10MB per backend server
- CPU: < 5% idle, < 50% under load (on 4-core system)
- File handles: Manage connection lifecycle (cleanup on disconnect)

**Measurement**:
- Benchmark critical paths with criterion
- Profile with flamegraph for hot spots
- Load testing with realistic message patterns
- Monitor production metrics (Prometheus-compatible)

## Development Workflow

### Code Quality Gates

**Before Commit**:
1. Run `cargo fmt` (enforced by pre-commit hook)
2. Run `cargo clippy` (no warnings allowed)
3. Run `cargo test` (all tests pass)
4. Check for unused imports and variables
5. Update relevant documentation

**Before Merge to Main**:
1. Feature branch tested in isolation
2. Integration tests pass with main
3. Performance benchmarks run (no regressions > 10%)
4. Protocol compliance tests pass for all supported versions
5. Documentation updated (CLAUDE.md, relevant specs)

### Branching Strategy

- Main branch: Stable, always deployable
- Feature branches: `###-feature-name` (e.g., 003-mcp-protocol-support)
- Merge with `--no-ff` for clear history
- Delete feature branches after merge

### Research and Planning

When implementing complex features (protocol version support, plugin systems):

**Phase 0 - Research**:
- Use Context7 for library documentation and best practices
- Use Serena for understanding existing codebase patterns
- Document research decisions in `specs/###-feature/research.md`
- Identify unknowns and resolve them before design

**Phase 1 - Design**:
- Define data model (`data-model.md`)
- Create API contracts (`contracts/*.md`)
- Write developer quickstart (`quickstart.md`)
- Validate against constitution principles

**Phase 2 - Implementation**:
- Generate dependency-ordered tasks (`tasks.md`)
- Implement incrementally (test → implement → refactor)
- Commit working code frequently
- Update documentation as you go

### When Stuck (After 3 Attempts)

**CRITICAL**: Maximum 3 attempts per issue, then STOP.

1. Use Context7 to find relevant documentation
2. Use Serena to find similar patterns in codebase
3. Document what failed and why
4. Ask for guidance (don't continue guessing)

## Governance

### Amendment Process

Constitution amendments require:
1. Clear rationale for change (what problem does it solve?)
2. Impact analysis on existing code and practices
3. Update to dependent templates (plan, spec, tasks)
4. Version bump following semantic versioning
5. Commit with message: `docs: amend constitution to vX.Y.Z (change summary)`

### Semantic Versioning

- **MAJOR** (X.0.0): Backward incompatible principle changes (removing/redefining core principles)
- **MINOR** (0.X.0): New principles added or existing principles materially expanded
- **PATCH** (0.0.X): Clarifications, wording improvements, non-semantic refinements

### Compliance Verification

All pull requests MUST verify:
- [ ] Code follows Rust idioms (cargo clippy passes)
- [ ] Code is formatted (cargo fmt applied)
- [ ] All tests pass (cargo test succeeds)
- [ ] Performance benchmarks run (no regressions > 10%)
- [ ] Protocol compliance tests pass (for protocol changes)
- [ ] Documentation updated (if public API changed)
- [ ] Constitution principles upheld (no unjustified violations)

### Justified Violations

If a principle must be violated (e.g., performance vs. flexibility trade-off):
- Document in Complexity Tracking section of plan.md
- Explain why the violation is necessary
- Describe what simpler alternative was rejected and why
- Get explicit approval before proceeding

### Runtime Development Guidance

For day-to-day development decisions not covered by this constitution, refer to:
- **CLAUDE.md** - Project-specific development guide
- **README.md** - Architecture overview and quick start
- **specs/###-feature/quickstart.md** - Feature-specific implementation guide

**Version**: 1.0.0 | **Ratified**: 2025-10-12 | **Last Amended**: 2025-10-12
