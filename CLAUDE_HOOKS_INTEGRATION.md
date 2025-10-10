# Claude Code Hooks Integration for Context Tracing

## Overview

This integration uses Claude Code's hooks system to automatically manage context tracing sessions, collect feedback, and enhance the multi-turn conversation experience.

## Architecture

```
Claude Session Start
    ↓
session-start.sh hook
    ↓
Create session_id, store for tracking
    ↓
Inject context: "🔍 Context Tracing Active"
    ↓
Claude uses MCP tools
    ↓
post-tool-use.sh hook (after each tool)
    ↓
Record response_id, notify Claude
    ↓
Claude can query lineage or submit feedback
    ↓
Session End
    ↓
session-end.sh hook
    ↓
Prompt for final feedback
    ↓
Cleanup temp files
```

## Hooks Implemented

### 1. Session Start Hook (`.claude/hooks/session-start.sh`)

**Triggers:** When Claude session begins

**Actions:**
- Generates or extracts session_id
- Stores session_id in `/tmp/mcp-proxy-session-id`
- Logs session start to `~/.mcp-proxy/session-log.txt`
- Injects context message: "Context Tracing Active"

**Output:** Adds context to make Claude aware it's being tracked

### 2. Post Tool Use Hook (`.claude/hooks/post-tool-use.sh`)

**Triggers:** After every MCP tool completes successfully

**Matchers:**
- Matches: `mcp__proxy__*` (backend tools)
- Excludes: `mcp__proxy__tracing__*` (tracing tools themselves)

**Actions:**
- Queries database for most recent response_id
- Stores response_id in `/tmp/mcp-proxy-last-response-id`
- Logs tool usage
- Injects context: "Response tracked: resp_xyz..."

**Output:** Makes Claude aware of the response_id for potential feedback

### 3. Session End Hook (`.claude/hooks/session-end.sh`)

**Triggers:** When Claude session ends

**Actions:**
- Retrieves last response_id
- Logs session end
- Injects context: Prompts Claude to submit feedback
- Cleans up temp files

**Output:** Encourages feedback before session closes

## Slash Commands

### `/mcp-proxy:give-feedback <score> [comment]`

Submit feedback on the most recent response.

**Usage:**
```
/mcp-proxy:give-feedback 0.9 "Excellent code that worked perfectly"
/mcp-proxy:give-feedback -0.5 "Code had bugs and needed fixes"
```

**Behind the scenes:**
1. Reads response_id from `/tmp/mcp-proxy-last-response-id`
2. Calls `mcp__proxy__tracing__submit_feedback`
3. Shows propagation results

### `/mcp-proxy:show-trace [response_id] [format]`

Show lineage for a response.

**Usage:**
```
/mcp-proxy:show-trace
/mcp-proxy:show-trace resp_abc123 compact
/mcp-proxy:show-trace resp_abc123 tree
```

**Behind the scenes:**
1. Uses last response_id if not provided
2. Calls `mcp__proxy__tracing__get_trace`
3. Displays formatted provenance tree

### `/mcp-proxy:quality-report`

Generate analytics report on context quality.

**Usage:**
```
/mcp-proxy:quality-report
```

**Behind the scenes:**
1. Reads quality resources
2. Analyzes feedback trends
3. Shows top/bottom performers
4. Makes recommendations

## Configuration

### `.claude/settings.json`

```json
{
  "hooks": {
    "sessionStart": [{
      "command": ".claude/hooks/session-start.sh"
    }],
    "sessionEnd": [{
      "command": ".claude/hooks/session-end.sh"
    }],
    "postToolUse": [{
      "matchers": [{
        "tool": "mcp__proxy__*",
        "negate": ["mcp__proxy__tracing__*"]
      }],
      "command": ".claude/hooks/post-tool-use.sh"
    }]
  }
}
```

## Enhanced Multi-Turn Tracking

### Session Grouping

With hooks, all responses in a session can be linked:

**Without hooks:**
```
resp_001 (isolated)
resp_002 (isolated)
resp_003 (isolated)
```

**With hooks:**
```
session_abc123
  ├─ resp_001
  ├─ resp_002
  └─ resp_003
```

### Response Chaining

Future enhancement: Link responses in a conversation:

```rust
// In post-tool-use hook
if let Some(prev_resp_id) = get_previous_response() {
    // Add previous response as context to current response
    tracker.add_context(
        current_response_id,
        create_response_context(prev_resp_id),
        Some(0.6)
    ).await;
}
```

This creates a chain:
```
User Query (root)
  ↓
resp_001 uses [ctx_memory_docs]
  ↓
resp_002 uses [ctx_memory_docs, resp_001] ← Builds on previous!
  ↓
resp_003 uses [ctx_memory_docs, resp_001, resp_002]
```

## Automatic Feedback Collection

### Implicit Feedback Signals

The hooks can detect quality signals:

```bash
# In post-tool-use.sh
if [ "$EXIT_CODE" -ne 0 ]; then
    # Tool failed → implicit negative feedback
    auto_submit_feedback "$RESPONSE_ID" -0.4 "Tool execution failed"
fi

# Check for user corrections
if grep -q "actually\|correction\|fix that" <<< "$NEXT_PROMPT"; then
    # User correcting → implicit negative feedback
    auto_submit_feedback "$LAST_RESPONSE_ID" -0.3 "Required user correction"
fi
```

### Explicit Feedback Prompts

At session end, Claude can be prompted:

```
💭 Before you go, how was this session?
   /mcp-proxy:give-feedback 0.8 "Helped me complete the task"
```

## Benefits

### 1. Automatic Session Management
- No manual session tracking needed
- Response IDs automatically captured
- Session boundaries clear

### 2. Seamless Feedback Loop
- Claude reminded to give feedback
- Easy slash command interface
- Feedback propagates immediately

### 3. Self-Awareness Enhancement
- Claude sees "Response tracked: resp_xyz" after each tool
- Claude can query its own lineage mid-conversation
- Claude understands its context sources

### 4. Quality Improvement
- Every session contributes feedback
- Context scores evolve over time
- Poor contexts get deprecated
- Good contexts get prioritized

## Usage

### Enable Hooks

Hooks are automatically loaded from `.claude/hooks/` and `.claude/settings.json` when running Claude Code in this directory.

### Test the Integration

```bash
# Start Claude with the proxy
claude --mcp-config '{"mcpServers":{"proxy":{"command":"./target/debug/mcp-rust-proxy","args":["--config","mcp-proxy-config.yaml","--stdio"]}}}'

# Session starts → Hook injects "Context Tracing Active" message
# Use a tool → Hook captures response_id
# Query lineage → /mcp-proxy:show-trace
# Submit feedback → /mcp-proxy:give-feedback 0.9 "Great work!"
# Session ends → Hook prompts for feedback
```

## Future Enhancements

### Conversation Context Capture

```rust
// Hook to capture user's original question
pub struct ConversationContext {
    session_id: String,
    user_query: String,
    response_chain: Vec<String>,
    feedback_scores: Vec<f32>,
}
```

### Smart Feedback Suggestions

Based on execution patterns:
- All tools succeeded → Suggest score 0.7-1.0
- Some tools failed → Suggest score 0.0-0.5
- User had to retry → Suggest score 0.2-0.4
- Task completed first try → Suggest score 0.8-1.0

### Cross-Session Learning

```rust
// Track patterns across sessions
pub struct SessionAnalytics {
    successful_patterns: Vec<ContextPattern>,
    failed_patterns: Vec<ContextPattern>,
    improvement_rate: f32,
}
```

## Integration with Claude-Flow Patterns

Inspired by claude-flow's architecture:

**Queen-Worker Model:**
- Main Claude instance = Queen (coordinates)
- Each tool call = Worker (specialized task)
- Context tracing = Hive memory (shared knowledge)

**Hooks as Workflow Orchestration:**
- Pre-hooks: Setup, validation
- Post-hooks: Feedback, analytics
- Session hooks: State management

**Persistent Memory:**
- SQLite storage (like claude-flow)
- Context units = Memory entries
- Lineage = Task dependencies

## Summary

The hooks integration transforms the context tracing system from **passive tracking** to **active participant** in Claude's workflow:

✅ Automatic session management
✅ Real-time response tracking
✅ Easy feedback submission
✅ Self-awareness messages
✅ Quality analytics

This makes Claude **conscious of its own context usage** and enables **continuous improvement through feedback**! 🚀
