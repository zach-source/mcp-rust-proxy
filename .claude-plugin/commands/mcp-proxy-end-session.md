---
description: End the current tracking session and optionally submit session feedback
tags: [mcp, tracing, session, feedback]
---

End the current tracking session and generate session analytics.

Arguments:
- $1: Session score from -1.0 to 1.0 (optional)
- $2: Session feedback comment (optional)

Example:
  /mcp-proxy:end-session
  /mcp-proxy:end-session 0.8 "Productive session, solved the problem"

Instructions for Claude:
1. Read current session_id from /tmp/mcp-proxy-current-session
2. Call mcp__proxy__tracing__end_session tool
3. If session score provided ($1), also call mcp__proxy__tracing__submit_feedback for each response in session
4. Display session summary:
   - Total responses in session
   - Contexts used
   - Average quality score
5. Suggest improvements for next session
