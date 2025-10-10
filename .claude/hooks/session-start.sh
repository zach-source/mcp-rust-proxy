#!/bin/bash
# Claude Code Hook: Session Start
# Triggers when a new Claude session begins
# Initializes context tracing for the session

set -e

# Read hook input from stdin (contains session metadata)
INPUT=$(cat)

# Extract session ID from Claude metadata
SESSION_ID=$(echo "$INPUT" | jq -r '.sessionId // empty')

if [ -z "$SESSION_ID" ]; then
    # Generate fallback session ID
    SESSION_ID="session_$(date +%s)_$$"
fi

# Store session ID for use by other hooks
echo "$SESSION_ID" > /tmp/mcp-proxy-session-id

# Log session start
echo "[$(date -Iseconds)] Session started: $SESSION_ID" >> ~/.mcp-proxy/session-log.txt

# Create user query context if prompt is available
USER_PROMPT=$(echo "$INPUT" | jq -r '.prompt // empty')

if [ -n "$USER_PROMPT" ]; then
    # Call MCP proxy to record user query as context
    # This would be done via a direct API call or MCP tool
    echo "[$(date -Iseconds)] User query: ${USER_PROMPT:0:100}..." >> ~/.mcp-proxy/session-log.txt
fi

# Output: Add context about session tracking
cat << EOF
{
  "action": "addContext",
  "context": "🔍 Context Tracing Active: This session ($SESSION_ID) is being tracked. All tool calls will be recorded with full provenance. You can query your own lineage using mcp__proxy__tracing__get_trace."
}
EOF

exit 0
