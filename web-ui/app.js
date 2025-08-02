let ws = null;
let reconnectInterval = null;
const API_BASE = '/api';

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
    
    card.innerHTML = `
        <div class="server-header">
            <span class="server-name">${server.name}</span>
            <span class="server-state ${stateLower}">${server.state}</span>
        </div>
        <div class="server-info">
            ${server.restart_count !== undefined ? `Restarts: ${server.restart_count}` : ''}
        </div>
        <div class="server-actions">
            <button class="btn btn-start" ${!isStopped ? 'disabled' : ''} 
                    onclick="serverAction('${server.name}', 'start')">Start</button>
            <button class="btn btn-stop" ${!isRunning ? 'disabled' : ''} 
                    onclick="serverAction('${server.name}', 'stop')">Stop</button>
            <button class="btn btn-restart" ${!isRunning ? 'disabled' : ''} 
                    onclick="serverAction('${server.name}', 'restart')">Restart</button>
        </div>
    `;
    
    return card;
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