#!/usr/bin/env python3
"""
Simple MCP client to test communication with the MCP server
"""
import json
import subprocess
import sys

def test_mcp_server():
    """Test the MCP server using subprocess to simulate Claude's communication."""
    
    # Start the MCP server
    cmd = [
        "/home/nima/miniconda3/envs/qw/bin/python",
        "/home/nima/code/crypto/py/eth_data/mcp_server/eth_data_mcp_server.py"
    ]
    
    env = {
        "PYTHONPATH": "/home/nima/code/crypto/py/eth_data",
        "DATABASE_URL": "postgresql://postgres:postgres@localhost:5432/eth_db",
        "ETH_RPC_URL": "http://localhost:8545"
    }
    
    try:
        # Start the server process
        process = subprocess.Popen(
            cmd,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            env=env
        )
        
        # Test 1: Send initialize request first
        init_request = {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        }
        
        # Send the initialize request
        request_json = json.dumps(init_request) + "\n"
        print(f"Sending initialize: {request_json.strip()}")
        
        process.stdin.write(request_json)
        process.stdin.flush()
        
        # Read the initialize response
        response_line = process.stdout.readline()
        if response_line:
            print(f"Init response: {response_line.strip()}")
        else:
            print("✗ No initialize response received")
            return False
        
        # Test 2: Send tools/list request
        list_request = {
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        }
        
        # Send the tools/list request
        request_json = json.dumps(list_request) + "\n"
        print(f"Sending tools/list: {request_json.strip()}")
        
        process.stdin.write(request_json)
        process.stdin.flush()
        
        # Read the response
        response_line = process.stdout.readline()
        if response_line:
            print(f"Tools response: {response_line.strip()}")
            try:
                response = json.loads(response_line)
                if "result" in response and "tools" in response["result"]:
                    tools = response["result"]["tools"]
                    print(f"✓ Found {len(tools)} tools:")
                    for tool in tools:
                        print(f"  - {tool['name']}")
                    return True
                else:
                    print("✗ Invalid response format")
                    return False
            except json.JSONDecodeError as e:
                print(f"✗ JSON decode error: {e}")
                return False
        else:
            print("✗ No response received")
            return False
            
    except Exception as e:
        print(f"✗ Error testing server: {e}")
        return False
    finally:
        try:
            process.terminate()
            process.wait(timeout=2)
        except:
            process.kill()

if __name__ == "__main__":
    success = test_mcp_server()
    sys.exit(0 if success else 1)