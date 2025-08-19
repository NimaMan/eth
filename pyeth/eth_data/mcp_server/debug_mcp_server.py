#!/usr/bin/env python3
"""
Debug version of the MCP server to help troubleshoot connection issues
"""

import sys
import json
import traceback
import os

def debug_log(message):
    """Write debug messages to stderr"""
    print(f"DEBUG: {message}", file=sys.stderr, flush=True)

def main():
    """Simple debug MCP server"""
    debug_log("Starting debug MCP server")
    debug_log(f"Working directory: {os.getcwd()}")
    debug_log(f"Python path: {sys.path}")
    debug_log(f"Environment PYTHONPATH: {os.environ.get('PYTHONPATH', 'Not set')}")
    debug_log(f"Environment DATABASE_URL: {os.environ.get('DATABASE_URL', 'Not set')}")
    
    try:
        # Try to import the required modules
        sys.path.insert(0, '/home/nima/code/crypto/py/eth_data')
        from eth_data.tx_provider.processed_transaction_provider import RustProcessedTransactionProvider
        debug_log("✓ Successfully imported RustProcessedTransactionProvider")
    except Exception as e:
        debug_log(f"✗ Failed to import: {e}")
        debug_log(f"Traceback: {traceback.format_exc()}")
        return 1
    
    debug_log("Waiting for stdin...")
    
    # Simple MCP protocol handler
    while True:
        try:
            line = sys.stdin.readline()
            debug_log(f"Received: {line.strip()}")
            
            if not line:
                debug_log("EOF received, exiting")
                break
                
            if not line.strip():
                continue
                
            try:
                request = json.loads(line.strip())
                debug_log(f"Parsed request: {request}")
                
                if request.get("method") == "initialize":
                    response = {
                        "jsonrpc": "2.0",
                        "id": request.get("id"),
                        "result": {
                            "protocolVersion": "0.1.0",
                            "capabilities": {"tools": {}},
                            "serverInfo": {"name": "debug-mcp-server", "version": "1.0.0"}
                        }
                    }
                elif request.get("method") == "tools/list":
                    response = {
                        "jsonrpc": "2.0",
                        "id": request.get("id"),
                        "result": {
                            "tools": [{
                                "name": "test_tool",
                                "description": "A test tool",
                                "inputSchema": {"type": "object", "properties": {}}
                            }]
                        }
                    }
                else:
                    response = {
                        "jsonrpc": "2.0",
                        "id": request.get("id"),
                        "error": {"code": -32601, "message": f"Method not found: {request.get('method')}"}
                    }
                
                response_json = json.dumps(response)
                debug_log(f"Sending response: {response_json}")
                print(response_json, flush=True)
                
            except json.JSONDecodeError as e:
                debug_log(f"JSON decode error: {e}")
                error_response = {
                    "jsonrpc": "2.0",
                    "error": {"code": -32700, "message": "Parse error"}
                }
                print(json.dumps(error_response), flush=True)
                
        except KeyboardInterrupt:
            debug_log("Keyboard interrupt received")
            break
        except Exception as e:
            debug_log(f"Unexpected error: {e}")
            debug_log(f"Traceback: {traceback.format_exc()}")
            break
    
    debug_log("Debug MCP server shutting down")
    return 0

if __name__ == "__main__":
    sys.exit(main())