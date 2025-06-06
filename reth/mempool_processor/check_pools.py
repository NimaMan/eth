#!/usr/bin/env python3
"""Check if specific pools are in the Rust pool cache"""

import zmq
import json

# Pools that Python detected as scams
SCAM_POOLS = [
    '0xe734d96be9c596149aef94a1828152502aea2c67',
    '0x1a11756d4460d846cc0e99ac20bef2f0d6383331',
    '0x1b5a1f42001b890cfadf7b0788641c6707b023e7',
    '0x2f679be2364f45efbb2c7c9f6ccae670312150b3',
    '0x882e614b3a98d39d1f2cd618f3c7735c5aaa4734'
]

def main():
    context = zmq.Context()
    
    # Connect to Python's REP socket to get pool data
    socket = context.socket(zmq.REQ)
    socket.connect("tcp://localhost:5558")
    
    # Request pool data
    socket.send_json({"type": "get_pools"})
    
    # Get response
    response = socket.recv_json()
    pools = response.get('pools', {})
    
    print(f"Total pools in cache: {len(pools)}")
    print("\nChecking scam pools:")
    
    for pool_addr in SCAM_POOLS:
        if pool_addr in pools:
            pool_data = pools[pool_addr]
            print(f"✅ Found {pool_addr}: {pool_data['eth_reserve']:.6f} ETH")
        else:
            print(f"❌ NOT FOUND: {pool_addr}")
    
    socket.close()
    context.term()

if __name__ == "__main__":
    main()