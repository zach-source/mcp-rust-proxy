#!/usr/bin/env python3
import subprocess
import json
import sys

# Test direct communication with MCP server
def test_mcp_server(command, args):
    print(f"Testing MCP server: {command}")
    
    # Start the process
    proc = subprocess.Popen(
        [command] + args,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True
    )
    
    # Send initialize
    initialize_request = {
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "0.1.0",
            "capabilities": {},
            "clientInfo": {
                "name": "test-client",
                "version": "0.1.0"
            }
        }
    }
    
    print("Sending initialize...")
    proc.stdin.write(json.dumps(initialize_request) + "\n")
    proc.stdin.flush()
    
    # Read response
    response = proc.stdout.readline()
    print(f"Initialize response: {response.strip()}")
    
    # Send initialized notification
    initialized_notification = {
        "jsonrpc": "2.0",
        "method": "initialized",
        "params": {}
    }
    
    print("Sending initialized notification...")
    proc.stdin.write(json.dumps(initialized_notification) + "\n")
    proc.stdin.flush()
    
    # Send ping
    ping_request = {
        "jsonrpc": "2.0",
        "id": 2,
        "method": "ping",
        "params": {}
    }
    
    print("Sending ping...")
    proc.stdin.write(json.dumps(ping_request) + "\n")
    proc.stdin.flush()
    
    # Read response
    response = proc.stdout.readline()
    print(f"Ping response: {response.strip()}")
    
    # Close
    proc.stdin.close()
    proc.terminate()
    proc.wait()

if __name__ == "__main__":
    # Test the time server
    test_mcp_server(
        "/nix/store/blkyfy6pa9xkfzvqkpkhvhgk59wzbmrs-mcp-server-time-2025.7.1/bin/mcp-server-time",
        []
    )