# Tauri Backend Integration Design

## Overview
This document outlines the architecture for packaging and spawning the MCP Proxy backend server as a child process from the Tauri frontend application.

## Architecture Components

### 1. Backend Binary Packaging

#### Strategy: Sidecar Pattern
- **Approach**: Package the MCP proxy server as a sidecar binary within the Tauri app bundle
- **Benefits**: 
  - Self-contained distribution
  - No external dependencies
  - Platform-specific optimization
  - Automatic cleanup on app termination

#### Implementation:
```toml
# tauri-app/src-tauri/tauri.conf.json
{
  "bundle": {
    "externalBin": [
      "binaries/mcp-proxy-server"
    ],
    "resources": [
      "configs/default-config.yaml",
      "configs/examples/*"
    ]
  }
}
```

### 2. Backend Process Management

#### Using Tauri Shell Plugin
```rust
// tauri-app/src-tauri/src/backend_manager.rs
use tauri_plugin_shell::ShellExt;
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct BackendManager {
    process: Arc<RwLock<Option<CommandChild>>>,
    config_path: PathBuf,
    port: u16,
}

impl BackendManager {
    pub async fn start(&self, app_handle: AppHandle) -> Result<()> {
        let shell = app_handle.shell();
        
        // Get the sidecar binary path
        let sidecar_command = shell.sidecar("mcp-proxy-server")?;
        
        // Start the backend with configuration
        let child = sidecar_command
            .args([
                "--config", &self.config_path.to_string_lossy(),
                "--port", &self.port.to_string(),
                "--api-port", &(self.port + 1).to_string(),
            ])
            .spawn()?;
        
        // Store the process handle
        let mut process = self.process.write().await;
        *process = Some(child);
        
        // Wait for backend to be ready
        self.wait_for_ready().await?;
        
        Ok(())
    }
    
    pub async fn stop(&self) -> Result<()> {
        let mut process = self.process.write().await;
        if let Some(mut child) = process.take() {
            child.kill()?;
        }
        Ok(())
    }
    
    async fn wait_for_ready(&self) -> Result<()> {
        let max_retries = 30;
        let retry_delay = Duration::from_millis(100);
        
        for _ in 0..max_retries {
            if self.health_check().await.is_ok() {
                return Ok(());
            }
            tokio::time::sleep(retry_delay).await;
        }
        
        Err(anyhow!("Backend failed to start"))
    }
    
    async fn health_check(&self) -> Result<()> {
        let client = reqwest::Client::new();
        let url = format!("http://localhost:{}/health", self.port + 1);
        client.get(&url).send().await?;
        Ok(())
    }
}
```

### 3. IPC Communication Design

#### Dual-Mode Communication
1. **HTTP API** (Primary)
   - Backend exposes REST API on localhost
   - Frontend uses fetch/axios for requests
   - Benefits: Standard, debuggable, works with existing web tools

2. **WebSocket** (Real-time Updates)
   - For server status updates
   - For log streaming
   - Benefits: Low latency, bidirectional

#### Frontend Service Layer
```typescript
// tauri-app/src/services/backend.ts
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export class BackendService {
  private apiUrl: string;
  private wsUrl: string;
  private ws: WebSocket | null = null;
  
  constructor(port: number = 3000) {
    this.apiUrl = `http://localhost:${port + 1}/api`;
    this.wsUrl = `ws://localhost:${port + 1}/ws`;
  }
  
  async start(): Promise<void> {
    // Start backend via Tauri command
    await invoke('start_backend');
    
    // Connect WebSocket
    this.connectWebSocket();
  }
  
  async stop(): Promise<void> {
    this.disconnectWebSocket();
    await invoke('stop_backend');
  }
  
  private connectWebSocket(): void {
    this.ws = new WebSocket(this.wsUrl);
    
    this.ws.onmessage = (event) => {
      const data = JSON.parse(event.data);
      // Emit events for UI updates
      window.dispatchEvent(new CustomEvent('backend-update', { detail: data }));
    };
    
    this.ws.onerror = (error) => {
      console.error('WebSocket error:', error);
      // Attempt reconnection
      setTimeout(() => this.connectWebSocket(), 5000);
    };
  }
  
  // API Methods
  async getServers(): Promise<ServerInfo[]> {
    const response = await fetch(`${this.apiUrl}/servers`);
    return response.json();
  }
  
  async startServer(name: string): Promise<void> {
    await fetch(`${this.apiUrl}/servers/${name}/start`, { method: 'POST' });
  }
  
  async stopServer(name: string): Promise<void> {
    await fetch(`${this.apiUrl}/servers/${name}/stop`, { method: 'POST' });
  }
  
  async getLogs(server: string, lines: number = 100): Promise<LogEntry[]> {
    const response = await fetch(`${this.apiUrl}/logs/${server}?lines=${lines}`);
    return response.json();
  }
}
```

### 4. Configuration Management

#### Configuration Storage
```rust
// tauri-app/src-tauri/src/config_manager.rs
use tauri::api::path;

pub struct ConfigManager {
    app_handle: AppHandle,
}

impl ConfigManager {
    pub fn get_config_dir(&self) -> PathBuf {
        self.app_handle
            .path()
            .app_config_dir()
            .expect("Failed to get app config dir")
    }
    
    pub fn get_default_config_path(&self) -> PathBuf {
        self.get_config_dir().join("mcp-proxy-config.yaml")
    }
    
    pub async fn load_or_create_config(&self) -> Result<Config> {
        let config_path = self.get_default_config_path();
        
        if !config_path.exists() {
            // Create default config
            self.create_default_config(&config_path).await?;
        }
        
        // Load config
        let content = fs::read_to_string(&config_path).await?;
        let config: Config = serde_yaml::from_str(&content)?;
        
        Ok(config)
    }
    
    async fn create_default_config(&self, path: &Path) -> Result<()> {
        let default_config = Config {
            proxy_port: 3000,
            api_port: 3001,
            servers: vec![],
            health_check: Some(HealthCheckConfig {
                enabled: true,
                interval: 30,
                timeout: 5,
            }),
        };
        
        let content = serde_yaml::to_string(&default_config)?;
        fs::create_dir_all(path.parent().unwrap()).await?;
        fs::write(path, content).await?;
        
        Ok(())
    }
}
```

### 5. Tauri Commands

```rust
// tauri-app/src-tauri/src/commands.rs
use tauri::State;

#[tauri::command]
async fn start_backend(
    backend: State<'_, Arc<RwLock<BackendManager>>>,
    app_handle: AppHandle,
) -> Result<(), String> {
    backend
        .read()
        .await
        .start(app_handle)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn stop_backend(
    backend: State<'_, Arc<RwLock<BackendManager>>>,
) -> Result<(), String> {
    backend
        .read()
        .await
        .stop()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_backend_status(
    backend: State<'_, Arc<RwLock<BackendManager>>>,
) -> Result<BackendStatus, String> {
    backend
        .read()
        .await
        .get_status()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn update_config(
    config: Config,
    config_manager: State<'_, ConfigManager>,
) -> Result<(), String> {
    config_manager
        .save_config(config)
        .await
        .map_err(|e| e.to_string())
}
```

### 6. Platform-Specific Considerations

#### Windows
```rust
#[cfg(target_os = "windows")]
fn prepare_sidecar_command(cmd: &mut Command) {
    // Windows-specific: Hide console window
    cmd.creation_flags(CREATE_NO_WINDOW);
}
```

#### macOS
```rust
#[cfg(target_os = "macos")]
fn prepare_sidecar_command(cmd: &mut Command) {
    // macOS-specific: Set process priority
    cmd.env("NICE", "10");
}
```

#### Linux
```rust
#[cfg(target_os = "linux")]
fn prepare_sidecar_command(cmd: &mut Command) {
    // Linux-specific: Set process group
    cmd.process_group(0);
}
```

### 7. Build Configuration

#### Cargo.toml Updates
```toml
# tauri-app/src-tauri/Cargo.toml
[dependencies]
tauri = { version = "2", features = ["macos-private-api"] }
tauri-plugin-shell = "2.0"
tauri-plugin-process = "2.0"
mcp-proxy-core = { path = "../../crates/mcp-proxy-core" }
mcp-proxy-shared = { path = "../../crates/mcp-proxy-shared" }

[build-dependencies]
tauri-build = { version = "2", features = [] }
```

#### Build Script
```rust
// tauri-app/src-tauri/build.rs
fn main() {
    // Build the backend binary
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() != "android" 
        && std::env::var("CARGO_CFG_TARGET_OS").unwrap() != "ios" {
        build_backend_binary();
    }
    
    tauri_build::build()
}

fn build_backend_binary() {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let target = std::env::var("TARGET").unwrap();
    
    // Build mcp-proxy-server for the target platform
    let output = std::process::Command::new("cargo")
        .args(&[
            "build",
            "--release",
            "--package", "mcp-proxy-server",
            "--target", &target,
        ])
        .output()
        .expect("Failed to build backend binary");
    
    if !output.status.success() {
        panic!("Backend build failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    
    // Copy binary to expected location
    let binary_name = if cfg!(windows) {
        "mcp-proxy-server.exe"
    } else {
        "mcp-proxy-server"
    };
    
    let source = format!("../../target/{}/release/{}", target, binary_name);
    let dest = format!("{}/binaries/{}", out_dir, binary_name);
    
    std::fs::create_dir_all(format!("{}/binaries", out_dir)).unwrap();
    std::fs::copy(source, dest).unwrap();
}
```

### 8. Error Handling & Recovery

#### Backend Crash Recovery
```rust
pub struct BackendMonitor {
    manager: Arc<RwLock<BackendManager>>,
    app_handle: AppHandle,
}

impl BackendMonitor {
    pub async fn start_monitoring(self) {
        loop {
            tokio::time::sleep(Duration::from_secs(5)).await;
            
            if let Ok(status) = self.manager.read().await.get_status().await {
                if !status.running {
                    // Attempt to restart
                    if let Err(e) = self.manager.read().await.start(self.app_handle.clone()).await {
                        // Emit error event to frontend
                        self.app_handle.emit("backend-error", format!("Failed to restart: {}", e)).ok();
                    }
                }
            }
        }
    }
}
```

### 9. Security Considerations

#### Permission Configuration
```json
// tauri-app/src-tauri/capabilities/default.json
{
  "identifier": "default",
  "description": "Default permissions",
  "permissions": [
    "core:default",
    "shell:allow-execute",
    "shell:allow-kill",
    "process:allow-restart",
    "process:allow-exit",
    {
      "identifier": "shell:allow-spawn",
      "allow": [
        {
          "name": "mcp-proxy-server",
          "cmd": "binaries/mcp-proxy-server",
          "args": true
        }
      ]
    }
  ]
}
```

### 10. Testing Strategy

#### Integration Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_backend_lifecycle() {
        let manager = BackendManager::new();
        let app_handle = create_test_app_handle();
        
        // Start backend
        assert!(manager.start(app_handle.clone()).await.is_ok());
        
        // Verify it's running
        let status = manager.get_status().await.unwrap();
        assert!(status.running);
        
        // Stop backend
        assert!(manager.stop().await.is_ok());
        
        // Verify it's stopped
        let status = manager.get_status().await.unwrap();
        assert!(!status.running);
    }
}
```

## Benefits of This Approach

1. **Single Distribution**: Users download one app bundle containing everything
2. **Process Isolation**: Backend runs in separate process for stability
3. **Automatic Lifecycle**: Backend starts/stops with the app
4. **Cross-Platform**: Works on Windows, macOS, and Linux
5. **Resource Efficiency**: Shared binary, no duplication
6. **Easy Updates**: Update entire stack with single app update
7. **Developer Experience**: Standard web development for UI
8. **Production Ready**: Proper error handling and recovery

## Implementation Phases

### Phase 1: Basic Integration (Week 1)
- [ ] Set up sidecar binary packaging
- [ ] Implement basic start/stop commands
- [ ] Create minimal HTTP API communication

### Phase 2: Advanced Features (Week 2)
- [ ] Add WebSocket support for real-time updates
- [ ] Implement configuration management
- [ ] Add backend health monitoring

### Phase 3: Polish & Testing (Week 3)
- [ ] Platform-specific optimizations
- [ ] Comprehensive error handling
- [ ] Integration testing
- [ ] Performance optimization

## Next Steps

1. Update `tauri.conf.json` with sidecar configuration
2. Create `backend_manager.rs` module
3. Implement Tauri commands
4. Update frontend to use backend service
5. Test on all target platforms