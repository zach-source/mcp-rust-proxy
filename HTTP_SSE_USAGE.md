# MCP Rust Proxy - HTTP/SSE Multi-Client Usage Guide

## Overview

The MCP Rust Proxy now supports multiple Claude Code instances connecting simultaneously via HTTP using `mcp-remote@latest`.

## Setup (Option 1 - Simple HTTP Proxy)

###  1. Start the Proxy Server

In a dedicated terminal:

```bash
cd /Users/ztaylor/repos/workspaces/mcp-rust-proxy/main
cargo run -- --config mcp-proxy-config.yaml
```

The proxy will start on:
- **Port 3000**: JSON-RPC proxy endpoint (for MCP protocol)
- **Port 3001**: Web UI and management API

You should see logs indicating servers are starting:
```
INFO Starting MCP Rust Proxy Server
INFO Proxy will listen on 0.0.0.0:3000
INFO Web UI will be available on 0.0.0.0:3001
INFO Server context7 started successfully
INFO Server filesystem started successfully
...
```

### 2. Configure Claude Code

The `.mcp.json` has been updated to use `server-remote`:

```json
{
  "mcpServers": {
    "mcpRustProxy": {
      "command": "npx",
      "args": [
        "-y",
        "mcp-remote@latest",
        "http://localhost:3000",
        "--allow-http"
      ],
      "env": {},
      "disabled": false
    }
  }
}
```

**What this does:**
- `npx -y mcp-remote@latest` - Downloads and runs the HTTP/SSE bridge
- `http://localhost:3000` - Connects to your running proxy
- Claude Code communicates with server-remote via stdio
- server-remote handles HTTP/SSE to the proxy

### 3. Use Claude Code

1. **Start Claude Code** - It will automatically connect to the proxy via server-remote
2. **Multiple Instances** - You can open 2-3 Claude Code windows simultaneously
3. **All instances** share the same proxy and backend MCP servers

### 4. Verify Connection

Check the proxy logs (`/tmp/mcp-proxy.log`) for incoming requests:

```bash
tail -f /tmp/mcp-proxy.log
```

You should see DEBUG logs when Claude Code connects:
```
DEBUG hyper::proto::h1::io: parsed N headers
DEBUG mcp_rust_proxy::proxy: Received RPC request
```

## Architecture

```
Claude Code #1 ─┐
                 ├─> server-remote ─┐
Claude Code #2 ─┘                   ├─> HTTP ─> Proxy (port 3000) ─> Backend Servers
Claude Code #3 ──> server-remote ───┘
```

**Benefits:**
- ✅ Multiple clients supported
- ✅ Single proxy instance
- ✅ Shared backend connections
- ✅ Minimal configuration
- ✅ No custom SSE implementation needed

## Troubleshooting

### Proxy Not Starting

**Error:** `Address already in use (os error 48)`

**Solution:** Kill existing proxy instances:
```bash
pkill -f "mcp-rust-proxy"
# Wait a moment, then restart
cargo run -- --config mcp-proxy-config.yaml
```

### Can't Connect from Claude Code

1. **Verify proxy is running:**
   ```bash
   lsof -i :3000 | grep LISTEN
   ```

2. **Check proxy logs:**
   ```bash
   tail -50 /tmp/mcp-proxy.log
   ```

3. **Test with curl:**
   ```bash
   curl -s http://localhost:3001/health | jq .
   # Should return: {"status":"healthy","service":"mcp-proxy-web-ui"}
   ```

### Backend Server Errors

Some backend servers may fail to start (this is normal):

```
ERROR Failed to start server github: Transport error: Transport closed
```

**This is OK** - The proxy continues to work with other servers. Check configuration if a specific server is critical.

## Managing the Proxy

### Stop the Proxy

```bash
# Find the process
lsof -i :3000 | grep mcp-rust

# Kill gracefully (Ctrl+C in the terminal) or:
pkill -f "mcp-rust-proxy"
```

### Restart the Proxy

```bash
cargo run -- --config mcp-proxy-config.yaml
```

### View Logs

```bash
# Real-time
tail -f /tmp/mcp-proxy.log

# Last 100 lines
tail -100 /tmp/mcp-proxy.log

# Search for errors
grep ERROR /tmp/mcp-proxy.log
```

## Web UI

Access the management UI:
```
http://localhost:3001
```

Features:
- Server status monitoring
- Real-time logs
- Server control (start/stop/restart)
- Health checks

## Backend Server Configuration

The proxy connects to these MCP servers (from `mcp-proxy-config.yaml`):

- **context7** - Library documentation
- **filesystem** - File system access
- **playwright** - Browser automation
- **git** - Git operations
- **fetch** - Web content fetching
- **sequential-thinking** - Advanced reasoning
- **memory** - Persistent storage
- **time** - Time utilities
- **github** - GitHub API (may fail without auth)

## Performance

- **Concurrent Clients:** Tested with 2-3 Claude Code instances
- **Backend Pooling:** Connection pools per backend server
- **Request Timeout:** 30 seconds default
- **Health Checks:** Automatic with retries

## Next Steps

### Test Multiple Clients

1. Open 2-3 Claude Code instances
2. In each, try a tool call that uses the proxy
3. Monitor logs to see requests from different clients
4. Verify no conflicts or session issues

### Monitor Performance

```bash
# Watch active connections
watch -n 2 'lsof -i :3000 -i :3001 | grep LISTEN'

# Monitor resource usage
top -pid $(pgrep -f mcp-rust-proxy)
```

## Reverting to Stdio Mode

If you want to go back to the original stdio mode:

```json
{
  "mcpServers": {
    "mcpRustProxy": {
      "command": "sh",
      "args": [
        "-c",
        "cd /Users/ztaylor/repos/workspaces/mcp-rust-proxy/main && cargo run -- --config mcp-proxy-config.yaml --stdio"
      ],
      "env": {}
    }
  }
}
```

This launches the proxy as a subprocess for each Claude Code instance (no shared proxy).

## Future: Native SSE Support (Option 2)

For direct SSE support without `server-remote`, see:
- `MCP_HTTP_SSE_IMPLEMENTATION_PLAN.md` - Full implementation plan
- Native `/mcp` endpoint with SSE streaming
- Direct Claude Code connection without npm bridge
- Estimated 4-5 hours implementation time

## Support

- **Logs:** `/tmp/mcp-proxy.log`
- **Web UI:** `http://localhost:3001`
- **Health Check:** `http://localhost:3001/health`
- **Configuration:** `mcp-proxy-config.yaml`
- **Client Config:** `.mcp.json`
