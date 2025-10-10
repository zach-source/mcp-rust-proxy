---
description: Show the lineage trace for the last response or a specific response
tags: [mcp, tracing, lineage]
---

Display the complete provenance tree showing which context units influenced a response.

Arguments:
- $1: Response ID (optional - uses last if not provided)
- $2: Format: json|tree|compact (optional - defaults to tree)

Example:
  /mcp-proxy:show-trace
  /mcp-proxy:show-trace resp_abc123 tree

Instructions for Claude:
1. If no response_id provided, read from /tmp/mcp-proxy-last-response-id
2. Use mcp__proxy__tracing__get_trace tool with:
   - response_id: $1 or from file
   - format: $2 or "tree"
3. Display the lineage in a readable format
4. Explain:
   - Which contexts contributed
   - Their contribution weights
   - What each context represents
5. Suggest improvements if certain contexts seem problematic
