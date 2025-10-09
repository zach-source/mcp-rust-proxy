use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct ConfigHelperProps {
    pub show: bool,
    pub on_close: Callback<()>,
}

#[function_component(ConfigHelper)]
pub fn config_helper(props: &ConfigHelperProps) -> Html {
    let example_config = r#"# Example MCP Server Configuration
servers:
  # Context7 - Library documentation
  context7:
    command: "npx"
    args: ["-y", "@context7/mcp-server"]
    transport:
      type: stdio
    restartOnFailure: true
    maxRestarts: 3

  # Filesystem access
  filesystem:
    command: "npx"
    args: 
      - "-y"
      - "@modelcontextprotocol/server-filesystem"
      - "/path/to/allowed/directory"
    transport:
      type: stdio

  # GitHub integration
  github:
    command: "github-mcp-server"
    args: ["stdio"]
    env:
      GITHUB_TOKEN: "${GITHUB_TOKEN}"
    transport:
      type: stdio

# Proxy settings
proxy:
  port: 3000
  host: "0.0.0.0"
  connectionPoolSize: 10
  requestTimeoutMs: 30000

# Web UI settings  
webUi:
  enabled: true
  port: 3001
  host: "0.0.0.0"

# Health check defaults
healthCheck:
  enabled: true
  intervalSeconds: 60
  timeoutSeconds: 5"#;

    let on_copy = {
        let config = example_config.to_string();
        Callback::from(move |_| {
            // For now, just log the action. Clipboard API requires more complex setup
            web_sys::console::log_1(
                &"Copy configuration feature - config logged to console".into(),
            );
            web_sys::console::log_1(&config.clone().into());
        })
    };

    if !props.show {
        return html! {};
    }

    html! {
        <div class="modal">
            <div class="modal-content config-helper-modal">
                <div class="modal-header">
                    <h3>{"MCP Server Configuration Helper"}</h3>
                    <button class="close-btn" onclick={props.on_close.reform(|_| ())}>{"×"}</button>
                </div>

                <div class="config-helper-body">
                    <div class="helper-section">
                        <h4>{"Quick Start"}</h4>
                        <p>{"To configure MCP servers for the proxy:"}</p>
                        <ol>
                            <li>{"Create a YAML configuration file (e.g., mcp-proxy-config.yaml)"}</li>
                            <li>{"Define your MCP servers with their commands and transport settings"}</li>
                            <li>{"Start the proxy with: "}<code>{"mcp-rust-proxy --config your-config.yaml"}</code></li>
                        </ol>
                    </div>

                    <div class="helper-section">
                        <h4>{"Transport Types"}</h4>
                        <div class="transport-types">
                            <div class="transport-type">
                                <strong>{"stdio"}</strong>
                                <p>{"Standard input/output communication. Most common for local MCP servers."}</p>
                            </div>
                            <div class="transport-type">
                                <strong>{"httpSse"}</strong>
                                <p>{"HTTP Server-Sent Events. Used for servers that expose an HTTP endpoint."}</p>
                            </div>
                            <div class="transport-type">
                                <strong>{"websocket"}</strong>
                                <p>{"WebSocket connection. For real-time bidirectional communication."}</p>
                            </div>
                        </div>
                    </div>

                    <div class="helper-section">
                        <h4>{"Common MCP Servers"}</h4>
                        <table class="server-table">
                            <thead>
                                <tr>
                                    <th>{"Server"}</th>
                                    <th>{"Purpose"}</th>
                                    <th>{"Installation"}</th>
                                </tr>
                            </thead>
                            <tbody>
                                <tr>
                                    <td>{"filesystem"}</td>
                                    <td>{"File system access"}</td>
                                    <td><code>{"npm install -g @modelcontextprotocol/server-filesystem"}</code></td>
                                </tr>
                                <tr>
                                    <td>{"github"}</td>
                                    <td>{"GitHub API operations"}</td>
                                    <td><code>{"npm install -g @modelcontextprotocol/server-github"}</code></td>
                                </tr>
                                <tr>
                                    <td>{"context7"}</td>
                                    <td>{"Library documentation"}</td>
                                    <td><code>{"npm install -g @context7/mcp-server"}</code></td>
                                </tr>
                                <tr>
                                    <td>{"playwright"}</td>
                                    <td>{"Browser automation"}</td>
                                    <td><code>{"npm install -g @modelcontextprotocol/server-playwright"}</code></td>
                                </tr>
                                <tr>
                                    <td>{"memory"}</td>
                                    <td>{"Persistent memory storage"}</td>
                                    <td><code>{"npm install -g @modelcontextprotocol/server-memory"}</code></td>
                                </tr>
                            </tbody>
                        </table>
                    </div>

                    <div class="helper-section">
                        <h4>{"Example Configuration"}</h4>
                        <div class="config-example">
                            <button class="btn btn-secondary copy-btn" onclick={on_copy}>
                                {"Copy Example"}
                            </button>
                            <pre><code>{example_config}</code></pre>
                        </div>
                    </div>

                    <div class="helper-section">
                        <h4>{"Environment Variables"}</h4>
                        <p>{"You can use environment variables in your configuration:"}</p>
                        <ul>
                            <li><code>{"${VARIABLE_NAME}"}</code>{" - References an environment variable"}</li>
                            <li>{"Set variables before starting the proxy or in the env section"}</li>
                            <li>{"Common variables: GITHUB_TOKEN, API_KEY, HOME, etc."}</li>
                        </ul>
                    </div>

                    <div class="helper-section">
                        <h4>{"Tips"}</h4>
                        <ul>
                            <li>{"Enable health checks to monitor server status"}</li>
                            <li>{"Set restartOnFailure: true for automatic recovery"}</li>
                            <li>{"Use maxRestarts to prevent infinite restart loops"}</li>
                            <li>{"Check server logs in ~/.mcp-proxy/logs/ for debugging"}</li>
                            <li>{"Disable health checks for servers that don't support ping"}</li>
                        </ul>
                    </div>
                </div>

                <div class="modal-buttons">
                    <button class="btn btn-primary" onclick={props.on_close.reform(|_| ())}>
                        {"Close"}
                    </button>
                </div>
            </div>
        </div>
    }
}
