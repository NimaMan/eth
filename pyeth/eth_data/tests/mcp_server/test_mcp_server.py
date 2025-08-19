#!/usr/bin/env python3
"""
Test script for ETH Data MCP Server
====================================

Tests the MCP server functionality by simulating MCP protocol messages.
"""

import json
import asyncio
import sys
from typing import Dict, Any

# Add parent directory to path
sys.path.insert(0, '/home/nima/code/crypto/py/eth_data')
sys.path.insert(0, '/home/nima/code/crypto/py/eth_data/mcp_server')

from eth_data_mcp_server import EthDataMCPServer


async def test_server():
    """Test the MCP server with sample requests."""
    
    print("Testing ETH Data MCP Server")
    print("=" * 50)
    
    # Initialize server
    server = EthDataMCPServer()
    
    # Test 1: List tools
    print("\n1. Testing tools/list...")
    list_request = {
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {}
    }
    
    response = await server.handle_request(list_request)
    tools = response["result"]["tools"]
    print(f"   Found {len(tools)} tools:")
    for tool in tools:
        print(f"   - {tool['name']}: {tool['description']}")
    
    # Test 2: Get processed transaction from hash
    print("\n2. Testing get_processed_tx_from_hash...")
    # Use a known transaction hash (you can replace with a recent one)
    tx_request = {
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "get_processed_tx_from_hash",
            "arguments": {
                "tx_hash": "0x5c89f223fe19593cf85319dcdd3f6de618e770d958f72ea123d36db2fdf5d46e"
            }
        }
    }
    
    print("   Investigating transaction: 0x5c89f223fe19593cf85319dcdd3f6de618e770d958f72ea123d36db2fdf5d46e")
    response = await server.handle_request(tx_request)
    
    if "error" in response:
        print(f"   Error: {response['error']['message']}")
    else:
        result = json.loads(response["result"]["content"][0]["text"])
        if "error" not in result:
            print(f"   From: {result['from_address'][:10]}...")
            print(f"   To: {result['to_address'][:10]}...")
            print(f"   Value: {result['value']} wei")
            print(f"   Block: {result['block_number']}")
            print(f"   Status: {result['status']}")
            print(f"   Events: {len(result.get('events', []))} events")
    
    # Test 3: Get processed transactions for address (Vitalik's address as example)
    print("\n3. Testing get_processed_txs_for_address...")
    address_request = {
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "get_processed_txs_for_address",
            "arguments": {
                "address": "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045",
                "limit": 5
            }
        }
    }
    
    print("   Investigating address: 0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045 (Vitalik)")
    response = await server.handle_request(address_request)
    
    if "error" in response:
        print(f"   Error: {response['error']['message']}")
    else:
        result = json.loads(response["result"]["content"][0]["text"])
        print(f"   Found {result['transaction_count']} transactions (limited to 5)")
        if result.get('summary'):
            print(f"   Total incoming: {result['summary']['incoming']}")
            print(f"   Total outgoing: {result['summary']['outgoing']}")
    
    # Test 4: Process a block
    print("\n4. Testing process_block...")
    block_request = {
        "jsonrpc": "2.0",
        "id": 4,
        "method": "tools/call",
        "params": {
            "name": "process_block",
            "arguments": {
                "block_number": 20000000
            }
        }
    }
    
    print("   Investigating block: 20000000")
    response = await server.handle_request(block_request)
    
    if "error" in response:
        print(f"   Error: {response['error']['message']}")
    else:
        result = json.loads(response["result"]["content"][0]["text"])
        if "error" not in result:
            print(f"   Transactions: {result['transaction_count']}")
            print(f"   Total value: {result['total_value']} wei")
            print(f"   Average gas: {result.get('average_gas', 0)}")
    
    # Test 5: Trace fund flow
    print("\n5. Testing trace_fund_flow...")
    flow_request = {
        "jsonrpc": "2.0",
        "id": 5,
        "method": "tools/call",
        "params": {
            "name": "trace_fund_flow",
            "arguments": {
                "tx_hash": "0x5c89f223fe19593cf85319dcdd3f6de618e770d958f72ea123d36db2fdf5d46e",
                "depth": 2
            }
        }
    }
    
    print("   Analyzing fund flow with depth 2")
    response = await server.handle_request(flow_request)
    
    if "error" in response:
        print(f"   Error: {response['error']['message']}")
    else:
        result = json.loads(response["result"]["content"][0]["text"])
        if "error" not in result:
            print(f"   Flows found: {len(result.get('flows', []))}")
            print(f"   Unique addresses: {result.get('unique_addresses', 0)}")
            print(f"   Total value moved: {result.get('total_value_moved', '0')} wei")
    
    print("\n" + "=" * 50)
    print("All tests completed!")
    
    # Cleanup
    server.provider.close()


if __name__ == "__main__":
    asyncio.run(test_server())