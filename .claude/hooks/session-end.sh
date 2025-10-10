#!/bin/bash
# Claude Code Hook: Session End
# Triggers when a Claude session ends
# Prompts for feedback on the conversation

set -e

# Read session end data
INPUT=$(cat)

# Get the last response ID if available
LAST_RESPONSE_ID=$(cat /tmp/mcp-proxy-last-response-id 2>/dev/null || echo "")

if [ -n "$LAST_RESPONSE_ID" ]; then
    # Prompt user for feedback
    echo "[$(date -Iseconds)] Session ended. Last response: $LAST_RESPONSE_ID" >> ~/.mcp-proxy/session-log.txt

    cat << EOF
{
  "action": "addContext",
  "context": "💭 Session ending. Consider submitting feedback on your last response ($LAST_RESPONSE_ID) using:

  /mcp-proxy:give-feedback <score> <comment>

  Or call: mcp__proxy__tracing__submit_feedback

  Score: -1.0 (bad) to 1.0 (excellent)"
}
EOF
else
    echo '{"action": "none"}'
fi

# Clean up temp files
rm -f /tmp/mcp-proxy-last-response-id
rm -f /tmp/mcp-proxy-session-id

exit 0
