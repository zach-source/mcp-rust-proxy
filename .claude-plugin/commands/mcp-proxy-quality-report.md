---
description: Show quality metrics and top/bottom performing contexts
tags: [mcp, tracing, quality, analytics]
---

Generate a quality report showing context performance metrics.

Instructions for Claude:
1. Read trace://quality/recent-feedback resource
2. Read trace://quality/top-contexts resource (when available)
3. Read trace://quality/deprecated-contexts resource (when available)
4. Query the database for aggregate statistics
5. Present a formatted report showing:
   - Recent feedback trends (scores over time)
   - Top performing contexts (high scores)
   - Contexts needing review (low scores)
   - Recommendations for improvement
6. Suggest which contexts to update or deprecate
