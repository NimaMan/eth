#!/usr/bin/env python3
"""
Simple test of the get_processed_tx_from_hash method
"""

import asyncio
import json
from eth_data_mcp_server import EthDataMCPServer

async def test_tx():
    server = EthDataMCPServer()
    
    try:
        result = await server.get_processed_tx_from_hash({"tx_hash": "0x5c89f223fe19593cf85319dcdd3f6de618e770d958f72ea123d36db2fdf5d46e"})
        
        # Try to serialize to JSON to check if it works
        json_str = json.dumps(result, indent=2)
        print("✓ Successfully serialized to JSON")
        print(f"Hash: {result.get('hash', 'N/A')}")
        print(f"Block: {result.get('block_number', 'N/A')}")
        print(f"Type: {result.get('tx_type', 'N/A')}")
        print(f"ERC20 transfers: {len(result.get('erc20_transfers', []))}")
        print(f"Unique addresses: {len(result.get('unique_addresses', []))}")
        
    except Exception as e:
        print(f"✗ Error: {e}")
    finally:
        server.provider.close()

if __name__ == "__main__":
    asyncio.run(test_tx())