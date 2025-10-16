# Implementation Tasks: Aggregator Plugin Type

**Feature**: Aggregator Plugin for Context Optimization
**Branch**: `004-aggregator-plugin-type`
**Date**: 2025-10-13
**Spec**: [spec.md](spec.md) | **Plan**: [plan.md](plan.md) | **Data Model**: [data-model.md](data-model.md)

---

## Overview

This task list implements a JavaScript plugin using Claude Agent SDK to aggregate and optimize context from multiple MCP servers (context7, serena, etc.). Tasks are organized by user story to enable incremental, independent delivery.

**Key Principles**:
- ✅ User Story 1 (P1) = MVP - delivers core value independently
- ✅ User Stories 2-3 add optional enhancements
- ✅ Each story is independently testable
- ✅ [P] indicates tasks that can be parallelized

---

## Phase 1: Setup & Infrastructure

**Goal**: Create plugin directory, initialize npm project, install dependencies.

### T001 [Setup] Create plugin directory structure
- **File**: `src/plugins/official/aggregator-plugin/`
- **Action**: Create directory, initialize npm project with package.json
- **Commands**:
  ```bash
  mkdir -p src/plugins/official/aggregator-plugin
  cd src/plugins/official/aggregator-plugin
  npm init -y
  npm install @anthropic-ai/agent @modelcontextprotocol/sdk
  ```
- **Dependencies**: None
- **Success**: package.json exists with correct dependencies

### T002 [Setup] Create plugin entry point
- **File**: `src/plugins/official/aggregator-plugin/index.js`
- **Action**: Create main plugin file with basic structure (exports process function)
- **Details**: Follow pattern from curation-plugin, security-plugin
- **Dependencies**: T001
- **Success**: Plugin exports async process(input) function

### T003 [Setup] Add aggregator tool registration in Rust
- **File**: `src/proxy/aggregator_tools.rs`
- **Action**: Create tool registration module (like tracing_tools.rs, server_tools.rs pattern)
- **Details**: Register `mcp__proxy__aggregator__context_aggregator` tool, prepare metadata with MCP server configs
- **Dependencies**: None (can run parallel to T001-T002)
- **Success**: Tool appears in tools/list response

### T004 [Setup] Update proxy handler to route aggregator calls
- **File**: `src/proxy/handler.rs`
- **Action**: Add check for `mcp__proxy__aggregator__*` prefix, route to aggregator_tools handler
- **Dependencies**: T003
- **Success**: Aggregator tool calls route correctly

---

## Phase 2: Foundational - Plugin Infrastructure

**Goal**: Set up Claude Agent SDK integration and MCP client connections.

### T005 [Foundation] Implement MCP client initialization
- **File**: `src/plugins/official/aggregator-plugin/mcp-client.js`
- **Action**: Create helper to spawn stdio MCP clients from server configs
- **Details**: Use StdioClientTransport from @modelcontextprotocol/sdk
- **Dependencies**: T002
- **Success**: Can create stdio client for a given server config

### T006 [Foundation] Implement Claude Agent initialization
- **File**: `src/plugins/official/aggregator-plugin/agent.js`
- **Action**: Initialize Claude Agent SDK with MCP servers registered as tools
- **Details**: Configure with ANTHROPIC_API_KEY, system prompt for aggregation behavior
- **Dependencies**: T005
- **Success**: Agent initializes with MCP tools available

### T007 [Foundation] Add MCP server config to PluginMetadata
- **File**: `src/plugin/schema.rs`
- **Action**: Extend PluginMetadata struct to include mcp_servers field with server configs
- **Details**: Add Vec<McpServerConfig> with command, args, env fields
- **Dependencies**: None
- **Success**: Metadata can carry MCP server configurations to JavaScript

### T008 [Foundation] Pass MCP configs from Rust to plugin
- **File**: `src/proxy/aggregator_tools.rs`
- **Action**: Populate metadata.mcp_servers with server configs from AppState
- **Dependencies**: T007, T003
- **Success**: JavaScript plugin receives server configs in metadata

---

## Phase 3: User Story 1 (P1) - Context Aggregation

**Goal**: Implement core aggregation functionality - query MCP servers via Claude Agent, return optimized context.

**Independent Test**: Invoke aggregator tool with sample query, verify it returns results from multiple MCP servers with Claude-optimized aggregation.

### T009 [US1] Implement main plugin logic
- **File**: `src/plugins/official/aggregator-plugin/index.js`
- **Action**: Complete process() function - parse input, init Claude Agent, run query, return result
- **Details**:
  - Extract query from input.rawContent
  - Get MCP server configs from input.metadata.mcpServers
  - Create MCP clients
  - Initialize Agent with MCP tools
  - Run agent with query
  - Return aggregated context
- **Dependencies**: T006, T008
- **Success**: Plugin processes query and returns Claude's response

### T010 [US1] Add system prompt for aggregation behavior
- **File**: `src/plugins/official/aggregator-plugin/agent.js`
- **Action**: Define system prompt that instructs Claude to aggregate context efficiently
- **Details**: Prompt should emphasize: query multiple servers, rank by relevance, optimize for LLM context
- **Dependencies**: T006
- **Success**: Claude Agent uses aggregation-focused prompts

### T011 [US1] Configure aggregator in mcp-proxy-config.yaml
- **File**: `mcp-proxy-config.yaml`
- **Action**: Add aggregator plugin configuration
- **Details**:
  ```yaml
  aggregator:
    enabled: true
    mcpServers: ["context7", "serena", "memory", "filesystem"]
    systemPrompt: "..."
    timeoutSecs: 10
  ```
- **Dependencies**: T001
- **Success**: Configuration loads, aggregator enabled

### T012 [US1] Test end-to-end aggregation with real servers
- **File**: Manual testing
- **Action**: Call aggregator tool with sample query, verify results from multiple servers
- **Test Cases**:
  - Query: "How to use React hooks?"
  - Expect: Results from context7 (documentation)
  - Expect: Optimized, ranked context in response
  - Expect: Processing time < 10 seconds
- **Dependencies**: T009, T010, T011
- **Success**: Aggregation works with real MCP servers

### T013 [US1] Verify US1 acceptance criteria
- **File**: Manual verification
- **Action**: Test all 4 acceptance scenarios from spec.md
- **Dependencies**: T012
- **Success**: All US1 scenarios pass

---

## Phase 4: User Story 2 (P2) - Configurable Server Selection

**Goal**: Allow admins to configure which servers are queried for different query types.

**Independent Test**: Configure specific servers for query patterns, verify only those servers are used.

### T014 [US2] Implement server selection patterns in config
- **File**: `mcp-proxy-config.yaml`
- **Action**: Add serverRules section with pattern matching
- **Details**:
  ```yaml
  aggregator:
    serverRules:
      - pattern: "documentation|docs"
        servers: ["context7"]
      - pattern: "code|implementation"
        servers: ["serena"]
  ```
- **Dependencies**: T011
- **Success**: Configuration supports server selection rules

### T015 [US2] Implement pattern matching in plugin
- **File**: `src/plugins/official/aggregator-plugin/index.js`
- **Action**: Match query against patterns, filter MCP servers accordingly
- **Details**: Apply rules from config, fallback to all servers if no match
- **Dependencies**: T014, T009
- **Success**: Plugin queries only matching servers

### T016 [US2] Test server selection
- **File**: Manual testing
- **Action**: Test queries with different patterns, verify correct servers queried
- **Test Cases**:
  - "React documentation" → only context7
  - "function implementation" → only serena
  - "general query" → all servers (fallback)
- **Dependencies**: T015
- **Success**: Server selection works as configured

### T017 [US2] Verify US2 acceptance criteria
- **File**: Manual verification
- **Action**: Test all 3 acceptance scenarios from spec.md
- **Dependencies**: T016
- **Success**: All US2 scenarios pass

---

## Phase 5: User Story 3 (P3) - Response Quality Metrics

**Goal**: Add metadata showing relevance scores, server diversity, and quality metrics.

**Independent Test**: Inspect aggregator responses for metadata fields, verify accuracy.

### T018 [US3] Add metadata extraction from Claude Agent
- **File**: `src/plugins/official/aggregator-plugin/index.js`
- **Action**: Extract tool usage data from Agent response, count servers used
- **Details**: Agent SDK provides tool call history - use to compute metadata
- **Dependencies**: T009
- **Success**: Plugin returns metadata about which servers were queried

### T019 [US3] Format metadata in response
- **File**: `src/plugins/official/aggregator-plugin/index.js`
- **Action**: Return metadata object with serversQueried, serverDiversity, etc.
- **Details**: Follow contract schema from contracts/aggregator-tool-api.md
- **Dependencies**: T018
- **Success**: Response includes metadata section

### T020 [US3] Test metadata accuracy
- **File**: Manual testing
- **Action**: Verify metadata reflects actual processing
- **Test Cases**:
  - Query 2 servers → serversQueried = 2
  - Both respond → serverDiversity = 1.0
  - Verify processingTimeMs is realistic
- **Dependencies**: T019
- **Success**: Metadata is accurate

### T021 [US3] Verify US3 acceptance criteria
- **File**: Manual verification
- **Action**: Test all 3 acceptance scenarios from spec.md
- **Dependencies**: T020
- **Success**: All US3 scenarios pass

---

## Phase 6: Polish & Integration

**Goal**: Final testing, documentation, error handling improvements.

### T022 [Polish] Add error handling for edge cases
- **File**: `src/plugins/official/aggregator-plugin/index.js`
- **Action**: Handle all servers failing, timeout, API key missing
- **Dependencies**: T009
- **Success**: Graceful error messages for all edge cases

### T023 [Polish] Add logging and debugging
- **File**: `src/plugins/official/aggregator-plugin/index.js`
- **Action**: Log server queries, timing, errors for debugging
- **Dependencies**: T009
- **Success**: Console logs show aggregation process

### T024 [Polish] Update CLAUDE.md with aggregator docs
- **File**: `CLAUDE.md`
- **Action**: Document aggregator plugin, how to use, configuration
- **Dependencies**: T021
- **Success**: Documentation complete

### T025 [Polish] Test with all configured MCP servers
- **File**: Manual testing
- **Action**: Run aggregator with full server list (context7, serena, memory, filesystem, etc.)
- **Dependencies**: T013, T017, T021
- **Success**: Works with 5+ servers, no crashes

---

## Task Summary

**Total Tasks**: 25
**Phase Breakdown**:
- Phase 1 (Setup): 4 tasks
- Phase 2 (Foundation): 4 tasks
- Phase 3 (US1 - P1): 5 tasks
- Phase 4 (US2 - P2): 4 tasks
- Phase 5 (US3 - P3): 4 tasks
- Phase 6 (Polish): 4 tasks

**Parallel Opportunities**: None marked - tasks are sequential due to dependencies

**MVP Scope**: Phase 1-3 (T001-T013) delivers User Story 1
- LLM can call aggregator tool
- Queries multiple MCP servers via Claude Agent SDK
- Returns optimized, aggregated context
- Reduces context waste

**Incremental Delivery**:
1. **After T013**: MVP complete (US1 working)
2. **After T017**: Server selection added (US2 complete)
3. **After T021**: Quality metrics added (US3 complete)
4. **After T025**: Fully polished and documented

---

## Dependencies Between Phases

```
Phase 1 (Setup)
    ↓
Phase 2 (Foundation - MCP client + Claude Agent SDK setup)
    ↓
Phase 3 (US1 - Core Aggregation) ←─── MVP COMPLETE HERE
    ↓
Phase 4 (US2 - Server Selection) ←─── Optional enhancement
    ↓
Phase 5 (US3 - Quality Metrics) ←─── Optional enhancement
    ↓
Phase 6 (Polish) ←─── Production ready
```

Each user story can be tested independently and delivers incremental value.

---

## Success Criteria Verification

- **SC-001 to SC-006**: Verified through T013 (US1), T017 (US2), T021 (US3), T025 (final)
- **Context reduction**: Measured by comparing raw vs aggregated result sizes
- **Processing time**: Measured via metadata.processingTimeMs
- **Quality improvement**: Qualitative assessment via real usage

---

## Implementation Strategy

**Recommended Approach**: Implement incrementally following user story priorities

1. **Sprint 1 (T001-T013)**: Deliver MVP (US1)
   - JavaScript plugin with Claude Agent SDK
   - Basic aggregation working
   - Test with 2-3 MCP servers
   - ~2-4 hours of work

2. **Sprint 2 (T014-T017)**: Add server selection (US2)
   - Pattern-based routing
   - Configuration flexibility
   - ~1-2 hours of work

3. **Sprint 3 (T018-T025)**: Quality metrics and polish (US3 + polish)
   - Metadata enhancement
   - Edge case handling
   - Documentation
   - ~2-3 hours of work

**Total Estimated Time**: 5-9 hours for complete implementation

---

## Notes

- **No Rust-heavy development**: Mostly JavaScript plugin code + minimal Rust integration
- **Leverage existing patterns**: Follow curation-plugin structure exactly
- **Claude Agent SDK**: Does heavy lifting for MCP orchestration
- **Testing**: Manual testing sufficient for MVP (no complex test infrastructure needed)
