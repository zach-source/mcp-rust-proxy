---
description: Submit quality feedback on the last tracked response
tags: [mcp, tracing, feedback]
---

Submit feedback on your most recent response to improve context quality.

Arguments:
- $1: Score from -1.0 (poor) to 1.0 (excellent)
- $2: Feedback comment (optional)

Example:
  /mcp-proxy:give-feedback 0.8 "Accurate and helpful response"

Instructions for Claude:
1. Get the last tracked response_id from /tmp/mcp-proxy-last-response-id
2. Use the mcp__proxy__tracing__submit_feedback tool with:
   - response_id: (from file)
   - score: $1
   - feedback_text: $2 (if provided)
   - user_id: "claude-assistant"
3. Display the propagation results
4. Explain which contexts were updated and by how much
