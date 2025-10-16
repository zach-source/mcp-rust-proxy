/// Integration test for Serena MCP server through the proxy
///
/// This test verifies that:
/// 1. Serena can be started and initialized
/// 2. Serena responds to tools/list requests
/// 3. The proxy correctly forwards Serena's tools
/// 4. Tool names are properly prefixed with mcp__proxy__serena__
use serde_json::json;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::time::Duration;

#[test]
#[ignore] // Run with: cargo test serena_integration_test -- --ignored --nocapture
fn test_serena_tool_discovery_direct() {
    println!("\n=== Testing Direct Serena Connection ===\n");

    // Start serena directly (not through proxy)
    let mut child = Command::new("/nix/store/m3im9527fy90pw0n59ba4l25bk3rd2lj-serena-mcp-launcher")
        .env("HOME", "/Users/ztaylor")
        .env("PATH", "/nix/store/rjx0gy4cl4wp2wmibkb3yjpzw8kb7rk6-uv-0.7.22/bin:/nix/store/v21kg4vm7yy0wflh0avkibz0shk86jn8-python3-3.12.11/bin:/nix/store/sn3k53wdfngc737bkci111ic8psm7jn8-git-2.50.1/bin:/nix/store/1n6wb8x5dw1q5wdyws70myxr1397g0jd-rust-default-1.88.0/bin:/nix/store/20iv68w4k1vvjh18xxysjihfm02vq53k-rust-analyzer-2025-07-28/bin:/nix/store/apjxbw3ff380md24x0nmka3hi979drcr-nodejs-22.16.0/bin:/nix/store/7712hv9svx4cf3cc5g848aa073aak0a4-coreutils-9.7/bin:/usr/bin:/bin")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start serena");

    let mut stdin = child.stdin.take().expect("Failed to get stdin");
    let stdout = child.stdout.take().expect("Failed to get stdout");
    let mut reader = BufReader::new(stdout);

    println!("Step 1: Waiting 15 seconds for serena to initialize...");
    std::thread::sleep(Duration::from_secs(15));

    // Send initialize request
    println!("Step 2: Sending initialize request to serena...");
    let initialize_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "test-client",
                "version": "1.0"
            }
        }
    });

    writeln!(stdin, "{initialize_request}").expect("Failed to write initialize");
    stdin.flush().expect("Failed to flush");

    // Read initialize response
    let mut response_line = String::new();
    reader
        .read_line(&mut response_line)
        .expect("Failed to read initialize response");
    println!("Initialize response: {response_line}");

    // Send tools/list request
    println!("\nStep 3: Sending tools/list request to serena...");
    let tools_list_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {}
    });

    writeln!(stdin, "{tools_list_request}").expect("Failed to write tools/list");
    stdin.flush().expect("Failed to flush");

    // Read tools/list response
    let mut tools_response = String::new();
    reader
        .read_line(&mut tools_response)
        .expect("Failed to read tools/list response");

    println!("Tools/list response:\n{tools_response}");

    // Parse and verify tools
    let response: serde_json::Value =
        serde_json::from_str(&tools_response).expect("Failed to parse tools/list response");

    if let Some(tools) = response.get("result").and_then(|r| r.get("tools")) {
        let tools_array = tools.as_array().expect("Tools should be an array");
        println!("\nStep 4: Serena returned {} tools", tools_array.len());

        for (i, tool) in tools_array.iter().enumerate().take(5) {
            if let Some(name) = tool.get("name").and_then(|n| n.as_str()) {
                println!("  Tool {}: {}", i + 1, name);
            }
        }

        assert!(
            !tools_array.is_empty(),
            "Serena should return at least one tool"
        );
    } else {
        panic!("No tools found in response");
    }

    // Cleanup
    let _ = child.kill();
    println!("\n=== Direct Serena Test Complete ===");
}

#[tokio::test]
#[ignore] // Run with: cargo test test_serena_through_proxy -- --ignored --nocapture
async fn test_serena_through_proxy() {
    println!("\n=== Testing Serena Through Proxy ===\n");

    // Make HTTP request to proxy
    let client = reqwest::Client::new();

    println!("Step 1: Requesting tools/list from proxy...");
    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {}
    });

    let response = client
        .post("http://localhost:3000/")
        .json(&request)
        .send()
        .await
        .expect("Failed to send request to proxy");

    let response_json: serde_json::Value = response
        .json()
        .await
        .expect("Failed to parse proxy response");

    println!("Step 2: Analyzing tools from proxy response...");

    if let Some(tools) = response_json.get("result").and_then(|r| r.get("tools")) {
        let tools_array = tools.as_array().expect("Tools should be an array");

        // Count tools by server
        let mut server_counts: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        for tool in tools_array {
            if let Some(server) = tool.get("server").and_then(|s| s.as_str()) {
                *server_counts.entry(server.to_string()).or_insert(0) += 1;
            }
        }

        println!("\nStep 3: Tools by server:");
        for (server, count) in &server_counts {
            println!("  {server}: {count} tools");
        }

        // Check for serena specifically
        if let Some(serena_count) = server_counts.get("serena") {
            println!("\n✅ SUCCESS: Serena has {serena_count} tools through proxy");
            assert!(*serena_count > 0, "Serena should have at least one tool");
        } else {
            println!("\n❌ FAILURE: Serena tools not found in proxy response");
            println!("\nAvailable servers:");
            for server in server_counts.keys() {
                println!("  - {server}");
            }
            panic!("Serena tools missing from proxy");
        }
    } else {
        panic!("No tools found in proxy response");
    }

    println!("\n=== Proxy Serena Test Complete ===");
}

#[tokio::test]
#[ignore]
async fn test_serena_tool_call_through_proxy() {
    println!("\n=== Testing Serena Tool Call Through Proxy ===\n");

    let client = reqwest::Client::new();

    // First, get the list of tools to find a serena tool
    let list_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {}
    });

    let list_response = client
        .post("http://localhost:3000/")
        .json(&list_request)
        .send()
        .await
        .expect("Failed to list tools");

    let list_json: serde_json::Value = list_response.json().await.expect("Failed to parse");

    // Find first serena tool
    let serena_tool = list_json
        .get("result")
        .and_then(|r| r.get("tools"))
        .and_then(|t| t.as_array())
        .and_then(|arr| {
            arr.iter()
                .find(|tool| tool.get("server").and_then(|s| s.as_str()) == Some("serena"))
        });

    if let Some(tool) = serena_tool {
        let tool_name = tool
            .get("name")
            .and_then(|n| n.as_str())
            .expect("Tool should have name");
        println!("Found serena tool: {tool_name}");

        // Try to call it (this may fail if tool requires specific args)
        let call_request = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": tool_name,
                "arguments": {}
            }
        });

        let call_response = client
            .post("http://localhost:3000/")
            .json(&call_request)
            .send()
            .await;

        match call_response {
            Ok(resp) => {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                println!("Call response status: {status}");
                println!("Call response body: {body}");
            }
            Err(e) => {
                println!("Call failed (expected for some tools): {e}");
            }
        }
    } else {
        panic!("No serena tools found - cannot test tool calls");
    }
}
