# Plugin System Production Deployment Guide

**Version**: 1.0.0  
**Last Updated**: 2025-10-11

This guide covers production deployment best practices for the MCP Rust Proxy JavaScript plugin system.

---

## Table of Contents

1. [Production Configuration](#production-configuration)
2. [Performance Tuning](#performance-tuning)
3. [Monitoring & Alerting](#monitoring--alerting)
4. [Security Considerations](#security-considerations)
5. [Troubleshooting](#troubleshooting)
6. [Operational Runbook](#operational-runbook)

---

## Production Configuration

### Recommended Settings

```yaml
plugins:
  pluginDir: /opt/mcp-proxy/plugins  # Absolute path
  nodeExecutable: /usr/bin/node      # Specific Node.js version
  maxConcurrentExecutions: 20        # Based on server capacity
  poolSizePerPlugin: 10              # Warm processes per plugin
  defaultTimeoutMs: 45000            # 45 seconds default

  servers:
    context7:
      response:
        - name: curation-plugin
          order: 1
          enabled: true
          timeoutMs: 60000  # AI calls need more time
```

### Environment Variables

```bash
# Required for AI-powered plugins
export ANTHROPIC_API_KEY=your_production_key_here

# Optional: Node.js configuration
export NODE_OPTIONS="--max-old-space-size=2048"

# Optional: Enable debug logging
export RUST_LOG=mcp_rust_proxy=info,plugin=debug
```

### File Permissions

```bash
# Plugin directory
chmod 755 /opt/mcp-proxy/plugins

# Plugin files (read + execute)
chmod 755 /opt/mcp-proxy/plugins/*.js

# Config file (read only)
chmod 600 /opt/mcp-proxy/config.yaml
```

---

## Performance Tuning

### 1. Concurrency Settings

**`maxConcurrentExecutions`**: Global limit for simultaneous plugin executions

| Server Capacity | Recommended Value | Notes |
|----------------|-------------------|-------|
| Small (1-2 CPU) | 5-10 | Prevent resource exhaustion |
| Medium (4-8 CPU) | 10-20 | Balance throughput & latency |
| Large (16+ CPU) | 20-50 | Maximize throughput |

**`poolSizePerPlugin`**: Warm processes per plugin

| Plugin Type | Recommended Value | Notes |
|-------------|-------------------|-------|
| Fast (<100ms) | 3-5 | Low overhead |
| AI-powered (>1s) | 5-10 | Amortize spawn cost |
| CPU-intensive | 2-3 | Limit resource usage |

### 2. Timeout Configuration

```yaml
plugins:
  defaultTimeoutMs: 30000  # Default for most plugins
  
  servers:
    context7:
      response:
        - name: curation-plugin
          timeoutMs: 60000      # AI calls: 60s
        - name: path-normalizer
          timeoutMs: 5000       # Fast transform: 5s
```

**Guidelines:**
- Fast transformations: 3-10 seconds
- AI-powered plugins: 45-90 seconds
- Network calls: 15-30 seconds
- Disk I/O: 10-20 seconds

### 3. Process Pool Optimization

**Warm Pool Benefits:**
- Eliminates 50-100ms Node.js spawn overhead
- Keeps npm package dependencies loaded
- Reduces CPU spikes from cold starts

**Pool Size Tuning:**
```
Optimal Pool Size = (Peak Concurrent Requests / Plugin Count) + 2
```

Example: 20 req/s, 2 plugins → Pool size = 20/2 + 2 = 12 per plugin

---

## Monitoring & Alerting

### Prometheus Metrics

The plugin system exports the following metrics at `/metrics`:

#### Execution Metrics
```
# Total plugin executions
mcp_proxy_plugin_executions_total{plugin_name, server_name, phase}

# Execution duration (histogram)
mcp_proxy_plugin_execution_duration_seconds

# Error count by type
mcp_proxy_plugin_errors_total{plugin_name, server_name, error_type}

# Timeout count
mcp_proxy_plugin_timeouts_total{plugin_name, server_name}
```

#### Pool Metrics
```
# Current pool size
mcp_proxy_plugin_pool_size

# Available processes
mcp_proxy_plugin_pool_available

# Processes spawned (lifetime)
mcp_proxy_plugin_processes_spawned_total

# Processes killed (lifetime)
mcp_proxy_plugin_processes_killed_total
```

### Recommended Alerts

**1. High Error Rate**
```yaml
alert: PluginErrorRateHigh
expr: |
  rate(mcp_proxy_plugin_errors_total[5m]) > 0.1
labels:
  severity: warning
annotations:
  summary: "Plugin error rate above 10%"
```

**2. Frequent Timeouts**
```yaml
alert: PluginTimeoutsHigh
expr: |
  rate(mcp_proxy_plugin_timeouts_total[5m]) > 0.05
labels:
  severity: critical
annotations:
  summary: "Plugin timeouts above 5%"
```

**3. Pool Exhaustion**
```yaml
alert: PluginPoolExhausted
expr: |
  mcp_proxy_plugin_pool_available == 0
labels:
  severity: warning
annotations:
  summary: "Plugin pool has no available processes"
```

### Grafana Dashboard Queries

**Plugin Execution Rate**
```promql
rate(mcp_proxy_plugin_executions_total[5m])
```

**p95 Latency by Plugin**
```promql
histogram_quantile(0.95, 
  rate(mcp_proxy_plugin_execution_duration_seconds_bucket[5m]))
```

**Error Rate by Plugin**
```promql
rate(mcp_proxy_plugin_errors_total[5m]) / 
rate(mcp_proxy_plugin_executions_total[5m])
```

---

## Security Considerations

### 1. Plugin Code Review

**Pre-Deployment Checklist:**
- [ ] All plugins reviewed for malicious code
- [ ] No hardcoded credentials in plugin files
- [ ] Input validation implemented
- [ ] Error handling doesn't leak sensitive data
- [ ] Dependencies scanned for vulnerabilities (`npm audit`)

### 2. Least Privilege

```yaml
# Run proxy as non-root user
User=mcp-proxy
Group=mcp-proxy

# Limit plugin directory access
chmod 755 /opt/mcp-proxy/plugins
chown mcp-proxy:mcp-proxy /opt/mcp-proxy/plugins
```

### 3. Secrets Management

**DON'T:**
```javascript
// ❌ Hardcoded API key
const apiKey = 'sk-abc123...';
```

**DO:**
```javascript
// ✅ Environment variable
const apiKey = process.env.ANTHROPIC_API_KEY;
if (!apiKey) {
  console.error(JSON.stringify({
    text: "", continue: false, 
    error: "API key not configured"
  }));
  process.exit(1);
}
```

### 4. Network Isolation

If plugins make external API calls:
- Use dedicated network namespace
- Implement egress filtering
- Monitor outbound connections
- Rate limit API calls

---

## Troubleshooting

### High Latency

**Symptoms:** Plugin execution duration >500ms p95

**Diagnosis:**
```bash
# Check metrics
curl http://localhost:3001/metrics | grep plugin_execution_duration

# Enable debug logging
RUST_LOG=plugin=debug cargo run -- --config config.yaml
```

**Solutions:**
1. Increase `poolSizePerPlugin` (reduce spawn overhead)
2. Optimize plugin code (reduce AI calls, cache results)
3. Increase `maxConcurrentExecutions` if CPU available
4. Consider async I/O in plugins

### Frequent Timeouts

**Symptoms:** `mcp_proxy_plugin_timeouts_total` increasing

**Diagnosis:**
```bash
# Check timeout metrics by plugin
curl http://localhost:3001/metrics | grep plugin_timeouts_total

# Check proxy logs for timeout messages
journalctl -u mcp-proxy | grep "timed out"
```

**Solutions:**
1. Increase `timeoutMs` for slow plugins
2. Optimize plugin logic (profile with `node --prof`)
3. Check network latency (if plugin makes external calls)
4. Verify Node.js not CPU-starved

### Memory Leaks

**Symptoms:** Proxy memory usage growing over time

**Diagnosis:**
```bash
# Monitor process memory
ps aux | grep mcp-rust-proxy

# Check for zombie Node.js processes
ps aux | grep node | wc -l

# Check pool metrics
curl http://localhost:3001/metrics | grep plugin_pool
```

**Solutions:**
1. Verify processes are being killed (check `plugin_processes_killed_total`)
2. Reduce `poolSizePerPlugin` if too many warm processes
3. Add health checks to kill unhealthy processes
4. Restart proxy periodically (graceful reload)

### Plugin Not Executing

**Diagnosis:**
```bash
# Check plugin discovery
RUST_LOG=plugin=debug cargo run -- --config config.yaml 2>&1 | grep "Discovered"

# Verify plugin file permissions
ls -la /opt/mcp-proxy/plugins/*.js

# Test plugin standalone
cat test-input.json | node /opt/mcp-proxy/plugins/your-plugin.js
```

**Common Issues:**
- Plugin file not executable (`chmod +x`)
- Plugin name mismatch (config vs filename)
- Plugin directory path incorrect
- Node.js not in PATH

---

## Operational Runbook

### Daily Operations

**1. Health Check**
```bash
# Check metrics endpoint
curl -s http://localhost:3001/metrics | grep -E "plugin_(executions|errors|timeouts)"

# Expected: Low error rate (<1%), no sustained timeouts
```

**2. Log Review**
```bash
# Check for plugin errors
journalctl -u mcp-proxy --since "1 hour ago" | grep -i "plugin.*error"

# Check for performance issues
journalctl -u mcp-proxy --since "1 hour ago" | grep -E "timed out|slow"
```

### Deployment Procedure

**1. Pre-Deployment Testing**
```bash
# Test new plugins locally
cat test-input.json | node plugins/new-plugin.js

# Run integration tests
cargo test --test plugin_*

# Verify config syntax
yq eval . config.yaml
```

**2. Staged Rollout**
```yaml
# Step 1: Deploy plugin with enabled=false
plugins:
  servers:
    context7:
      response:
        - name: new-plugin
          order: 10
          enabled: false  # Test mode

# Step 2: Enable for 10% of traffic (use routing rules)
# Step 3: Monitor metrics for 1 hour
# Step 4: Enable for 100% if stable
```

**3. Rollback Plan**
```yaml
# Quick rollback: Set enabled=false
plugins:
  servers:
    context7:
      response:
        - name: problematic-plugin
          enabled: false  # Disables without restart

# Full rollback: Revert config and restart
git revert <commit>
systemctl restart mcp-proxy
```

### Incident Response

**Plugin Causing Errors:**
```bash
# 1. Identify problem plugin
curl http://localhost:3001/metrics | grep plugin_errors_total

# 2. Disable immediately
vim /etc/mcp-proxy/config.yaml  # Set enabled: false
systemctl reload mcp-proxy

# 3. Investigate
journalctl -u mcp-proxy --since "1 hour ago" | grep "plugin-name"

# 4. Fix and redeploy with testing
```

**High Latency Event:**
```bash
# 1. Check plugin duration metrics
curl http://localhost:3001/metrics | grep plugin_execution_duration

# 2. Identify slow plugin
# Duration histogram shows which plugins are slow

# 3. Temporary mitigation: Increase timeout or disable
# 4. Root cause: Profile plugin code, optimize
```

### Maintenance

**Weekly:**
- Review error rates and timeouts
- Check pool utilization (`plugin_pool_available`)
- Update plugin dependencies (`npm audit fix`)
- Review plugin logs for warnings

**Monthly:**
- Performance baseline comparison
- Plugin code audit
- Dependency security scan
- Capacity planning review

**Quarterly:**
- Load testing with realistic traffic
- Disaster recovery drill
- Documentation updates
- Node.js version upgrade evaluation

---

## Capacity Planning

### Estimating Resource Needs

**CPU:**
```
Required CPU Cores = 
  (Avg Plugin Executions/sec * Avg Plugin Duration) / 1000
  + 2 (proxy overhead)

Example: 100 req/s * 200ms = 20 cores + 2 = 22 cores
```

**Memory:**
```
Required Memory = 
  (Number of Plugins * Pool Size * 50MB) + 500MB (proxy)

Example: 3 plugins * 10 pool * 50MB = 1.5GB + 500MB = 2GB
```

### Scaling Guidelines

| Traffic Level | Config |
|---------------|--------|
| <10 req/s | Default settings |
| 10-100 req/s | maxConcurrent=20, pool=10 |
| 100-1000 req/s | maxConcurrent=50, pool=20, horizontal scaling |
| >1000 req/s | Multiple proxy instances + load balancer |

---

## Production Checklist

Before deploying to production:

- [ ] All plugins tested with production-like data
- [ ] Error handling verified (graceful degradation)
- [ ] Timeout values tuned based on load testing
- [ ] API keys stored in secrets manager (not config files)
- [ ] Monitoring dashboards configured
- [ ] Alerts set up for errors and timeouts
- [ ] Runbook tested (deployment, rollback, incident response)
- [ ] Logs shipped to centralized logging system
- [ ] Backup configuration stored securely
- [ ] Disaster recovery plan documented

---

## Support & Resources

- [Plugin Quickstart Guide](../PLUGIN_QUICKSTART.md)
- [Plugin API Contract](../specs/002-javascript-plugin-system/contracts/plugin-api.md)
- [Project Documentation](../CLAUDE.md)

For issues or questions, open a GitHub issue or consult the project documentation.

---

**Document Version**: 1.0.0  
**Plugin System Version**: 1.0.0 (Feature Branch: 002-javascript-plugin-system)
