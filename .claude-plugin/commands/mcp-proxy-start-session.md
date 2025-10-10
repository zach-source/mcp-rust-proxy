---
description: Start a new context tracking session for this conversation
tags: [mcp, tracing, session]
---

Start a new tracking session to group all responses in this conversation.

Arguments:
- $1: User's original question/request (optional)

Example:
  /mcp-proxy:start-session "Help me build a REST API"

Instructions for Claude:
1. Call mcp__proxy__tracing__start_session tool with:
   - user_query: $1 (if provided)
2. Store the returned session_id for reference
3. Inform user: "Session tracking started - all responses will be grouped"
4. Mention that they can end the session with /mcp-proxy:end-session
