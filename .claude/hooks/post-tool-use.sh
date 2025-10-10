#!/bin/bash
# Claude Code Hook: Post Tool Use
# Triggers after any tool completes
# Records response_id and enables feedback collection

set -e

# Read tool execution data from stdin
INPUT=$(cat)

# Extract tool information
TOOL_NAME=$(echo "$INPUT" | jq -r '.tool // empty')
EXIT_CODE=$(echo "$INPUT" | jq -r '.exitCode // 0')

# Check if this was a successful backend tool call (not a tracing tool)
if [[ "$TOOL_NAME" == mcp__proxy__* ]] && [[ "$TOOL_NAME" != mcp__proxy__tracing__* ]]; then
    # This was a backend MCP tool - it was auto-tracked
    # Query the most recent response_id from database
    DB_PATH="$HOME/.mcp-proxy/context-tracing.db"

    if [ -f "$DB_PATH" ]; then
        RESPONSE_ID=$(sqlite3 "$DB_PATH" "SELECT id FROM responses ORDER BY timestamp DESC LIMIT 1;" 2>/dev/null || echo "")

        if [ -n "$RESPONSE_ID" ]; then
            # Store response ID for potential feedback
            echo "$RESPONSE_ID" > /tmp/mcp-proxy-last-response-id

            # Log the tracking
            echo "[$(date -Iseconds)] Tool: $TOOL_NAME → Response: $RESPONSE_ID" >> ~/.mcp-proxy/session-log.txt

            # Add context about the tracked response
            cat << EOF
{
  "action": "addContext",
  "context": "📊 Response tracked: $RESPONSE_ID (from $TOOL_NAME). You can submit feedback using /mcp-proxy:give-feedback or the mcp__proxy__tracing__submit_feedback tool."
}
EOF
            exit 0
        fi
    fi
fi

# No action needed for tracing tools or failed tools
echo '{"action": "none"}'
exit 0
