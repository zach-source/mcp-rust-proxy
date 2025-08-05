let ws = null;
let reconnectInterval = null;
const API_BASE = '/api';
let currentLogServer = null;
let pendingAction = null;

// Initialize the application
document.addEventListener('DOMContentLoaded', () => {
    connectWebSocket();
    loadInitialData();
    
    // Refresh metrics every 5 seconds
    setInterval(loadMetrics, 5000);
});

function connectWebSocket() {
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const wsUrl = `${protocol}//${window.location.host}/api/ws`;
    
    ws = new WebSocket(wsUrl);
    
    ws.onopen = () => {
        console.log('WebSocket connected');
        updateConnectionStatus(true);
        
        if (reconnectInterval) {
            clearInterval(reconnectInterval);
            reconnectInterval = null;
        }
    };
    
    ws.onmessage = (event) => {
        const message = JSON.parse(event.data);
        handleWebSocketMessage(message);
    };
    
    ws.onclose = () => {
        console.log('WebSocket disconnected');
        updateConnectionStatus(false);
        
        // Attempt to reconnect every 5 seconds
        if (!reconnectInterval) {
            reconnectInterval = setInterval(() => {
                console.log('Attempting to reconnect...');
                connectWebSocket();
            }, 5000);
        }
    };
    
    ws.onerror = (error) => {
        console.error('WebSocket error:', error);
    };
}

function updateConnectionStatus(connected) {
    const statusEl = document.getElementById('connection-status');
    statusEl.textContent = connected ? 'Connected' : 'Disconnected';
    statusEl.className = `status-indicator ${connected ? 'connected' : 'disconnected'}`;
}

function handleWebSocketMessage(message) {
    switch (message.type) {
        case 'initial':
        case 'update':
            updateServers(message.data.servers);
            if (message.data.stats) {
                updateStats(message.data.stats);
            }
            break;
        case 'log':
            if (message.data.server === currentLogServer) {
                appendLog(message.data);
            }
            break;
    }
}

function updateServers(servers) {
    const container = document.getElementById('servers-list');
    container.innerHTML = '';
    
    servers.forEach(server => {
        const card = createServerCard(server);
        container.appendChild(card);
    });
}

function createServerCard(server) {
    const card = document.createElement('div');
    card.className = 'server-card';
    
    const stateLower = server.state.toLowerCase();
    const isRunning = stateLower === 'running';
    const isStopped = stateLower === 'stopped';
    
    // Format health check status
    let healthStatus = '';
    if (server.health_check_enabled) {
        if (server.last_health_check) {
            const isHealthy = server.last_health_check.success;
            const healthClass = isHealthy ? 'healthy' : 'unhealthy';
            const responseTime = server.last_health_check.response_time_ms 
                ? `${server.last_health_check.response_time_ms}ms` 
                : 'N/A';
            healthStatus = `
                <div>
                    <span class="health-status">
                        <span class="health-indicator ${healthClass}"></span>
                        Health: ${isHealthy ? 'Healthy' : 'Unhealthy'} (${responseTime})
                    </span>
                </div>
                <div class="time-ago">Last check: ${formatTimeAgo(server.last_health_check.timestamp)}</div>
            `;
        } else {
            healthStatus = `
                <div>
                    <span class="health-status">
                        <span class="health-indicator disabled"></span>
                        Health: Pending
                    </span>
                </div>
            `;
        }
    } else {
        healthStatus = `
            <div>
                <span class="health-status">
                    <span class="health-indicator disabled"></span>
                    Health checks disabled
                </span>
            </div>
        `;
    }
    
    // Format last access time
    const lastAccess = server.last_access_time 
        ? `<div class="time-ago">Last accessed: ${formatTimeAgo(server.last_access_time)}</div>`
        : '<div class="time-ago">Never accessed</div>';
    
    card.innerHTML = `
        <div class="server-header">
            <span class="server-name">${server.name}</span>
            <span class="server-state ${stateLower}">${server.state}</span>
        </div>
        <div class="server-info">
            <div>Restarts: ${server.restart_count || 0}</div>
            ${healthStatus}
            ${lastAccess}
        </div>
        <div class="server-actions">
            <button class="btn btn-start" ${!isStopped ? 'disabled' : ''} 
                    onclick="confirmServerAction('${server.name}', 'start')">Start</button>
            <button class="btn btn-stop" ${!isRunning ? 'disabled' : ''} 
                    onclick="confirmServerAction('${server.name}', 'stop')">Stop</button>
            <button class="btn btn-restart" ${!isRunning ? 'disabled' : ''} 
                    onclick="confirmServerAction('${server.name}', 'restart')">Restart</button>
            <button class="btn btn-logs" onclick="viewLogs('${server.name}')">Logs</button>
        </div>
    `;
    
    return card;
}

function formatTimeAgo(timestamp) {
    const date = new Date(timestamp);
    const now = new Date();
    const diff = now - date;
    
    const seconds = Math.floor(diff / 1000);
    const minutes = Math.floor(seconds / 60);
    const hours = Math.floor(minutes / 60);
    const days = Math.floor(hours / 24);
    
    if (days > 0) return `${days}d ago`;
    if (hours > 0) return `${hours}h ago`;
    if (minutes > 0) return `${minutes}m ago`;
    return `${seconds}s ago`;
}

function updateStats(stats) {
    document.getElementById('total-servers').textContent = stats.total_servers || 0;
    document.getElementById('running-servers').textContent = stats.running_servers || 0;
}

async function loadInitialData() {
    try {
        const response = await fetch(`${API_BASE}/servers`);
        const data = await response.json();
        updateServers(data.servers);
    } catch (error) {
        console.error('Failed to load servers:', error);
    }
    
    loadMetrics();
}

async function loadMetrics() {
    try {
        const response = await fetch(`${API_BASE}/metrics`);
        const data = await response.json();
        displayMetrics(data.metrics);
    } catch (error) {
        console.error('Failed to load metrics:', error);
    }
}

function displayMetrics(metrics) {
    const container = document.getElementById('metrics-container');
    container.innerHTML = '';
    
    // Display key metrics
    const keyMetrics = [
        { name: 'Total Requests', key: 'mcp_proxy_requests_total' },
        { name: 'Failed Requests', key: 'mcp_proxy_requests_failed' },
        { name: 'Active Connections', key: 'mcp_proxy_active_connections' },
        { name: 'Connection Errors', key: 'mcp_proxy_connection_errors_total' },
        { name: 'Health Checks', key: 'mcp_proxy_health_checks_total' },
        { name: 'Failed Health Checks', key: 'mcp_proxy_health_checks_failed' },
    ];
    
    keyMetrics.forEach(({ name, key }) => {
        const metric = metrics.find(m => m.name === key);
        if (metric && metric.metrics.length > 0) {
            const card = document.createElement('div');
            card.className = 'metric-card';
            card.innerHTML = `
                <div class="metric-name">${name}</div>
                <div class="metric-value">${Math.round(metric.metrics[0].value)}</div>
            `;
            container.appendChild(card);
        }
    });
}

function confirmServerAction(serverName, action) {
    pendingAction = { serverName, action };
    const modal = document.getElementById('action-modal');
    const message = document.getElementById('action-message');
    
    message.textContent = `Are you sure you want to ${action} the server "${serverName}"?`;
    modal.style.display = 'flex';
}

function cancelAction() {
    pendingAction = null;
    document.getElementById('action-modal').style.display = 'none';
}

async function confirmAction() {
    if (!pendingAction) return;
    
    const { serverName, action } = pendingAction;
    document.getElementById('action-modal').style.display = 'none';
    
    await serverAction(serverName, action);
    pendingAction = null;
}

async function serverAction(serverName, action) {
    try {
        const response = await fetch(`${API_BASE}/servers/${serverName}/${action}`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
        });
        
        const data = await response.json();
        
        if (response.ok) {
            console.log(`${action} ${serverName}:`, data.message);
        } else {
            console.error(`Failed to ${action} ${serverName}:`, data.error);
            alert(`Failed to ${action} ${serverName}: ${data.error}`);
        }
    } catch (error) {
        console.error(`Error ${action} ${serverName}:`, error);
        alert(`Error ${action} ${serverName}: ${error.message}`);
    }
}

function viewLogs(serverName) {
    currentLogServer = serverName;
    document.getElementById('logs-server-name').textContent = serverName;
    document.getElementById('logs-container').innerHTML = '';
    document.getElementById('logs-modal').style.display = 'flex';
    
    // Subscribe to logs via WebSocket
    if (ws && ws.readyState === WebSocket.OPEN) {
        ws.send(JSON.stringify({
            type: 'subscribe_logs',
            server: serverName
        }));
    }
}

function closeLogs() {
    if (currentLogServer && ws && ws.readyState === WebSocket.OPEN) {
        ws.send(JSON.stringify({
            type: 'unsubscribe_logs',
            server: currentLogServer
        }));
    }
    
    currentLogServer = null;
    document.getElementById('logs-modal').style.display = 'none';
    document.getElementById('logs-container').innerHTML = '';
}

function clearLogs() {
    document.getElementById('logs-container').innerHTML = '';
}

function appendLog(logData) {
    const container = document.getElementById('logs-container');
    const entry = document.createElement('div');
    entry.className = 'log-entry';
    
    // Add log level class if available
    if (logData.level) {
        entry.classList.add(logData.level.toLowerCase());
    }
    
    entry.textContent = logData.message || logData.line || JSON.stringify(logData);
    container.appendChild(entry);
    
    // Auto-scroll if enabled
    if (document.getElementById('auto-scroll').checked) {
        container.scrollTop = container.scrollHeight;
    }
}

// Modal close on outside click
window.onclick = function(event) {
    const actionModal = document.getElementById('action-modal');
    const logsModal = document.getElementById('logs-modal');
    
    if (event.target === actionModal) {
        cancelAction();
    } else if (event.target === logsModal) {
        closeLogs();
    }
};