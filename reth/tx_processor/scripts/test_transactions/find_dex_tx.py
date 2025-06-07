#!/usr/bin/env python3
"""Find a recent DEX transaction with WETH"""

import requests
import json

rpc_url = 'http://127.0.0.1:8545'

# Common DEX routers
DEX_ROUTERS = [
    "0x68b3465833fb72A70ecDF485E0e4C7bD8665Fc45",  # Uniswap V3
    "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D",  # Uniswap V2
    "0x1111111254EEB25477B68fb85Ed929f73A960582",  # 1inch v5
]

# Get recent block
payload = {
    'jsonrpc': '2.0',
    'method': 'eth_blockNumber',
    'params': [],
    'id': 1
}
response = requests.post(rpc_url, json=payload)
latest_block = int(response.json()['result'], 16)

# Search last 20 blocks
for block_num in range(latest_block - 20, latest_block + 1):
    print(f"\rChecking block {block_num}...", end='', flush=True)
    
    payload = {
        'jsonrpc': '2.0',
        'method': 'eth_getBlockByNumber',
        'params': [hex(block_num), True],
        'id': 1
    }
    response = requests.post(rpc_url, json=payload)
    block = response.json().get('result')
    
    if not block or not block.get('transactions'):
        continue
        
    for tx in block['transactions']:
        # Check if it's a DEX transaction with ETH value
        if (tx.get('to') and 
            tx['to'].lower() in [addr.lower() for addr in DEX_ROUTERS] and
            int(tx.get('value', '0x0'), 16) > 10**17):  # > 0.1 ETH
            
            print(f"\n\nFound DEX transaction!")
            print(f"Hash: {tx['hash']}")
            print(f"Router: {tx['to']}")
            print(f"Value: {int(tx['value'], 16) / 1e18:.4f} ETH")
            
            # Get receipt to check for WETH events
            payload = {
                'jsonrpc': '2.0',
                'method': 'eth_getTransactionReceipt',
                'params': [tx['hash']],
                'id': 1
            }
            receipt = requests.post(rpc_url, json=payload).json()['result']
            
            if receipt and receipt.get('logs'):
                # Look for WETH Transfer events
                weth_logs = [log for log in receipt['logs'] 
                           if log['address'].lower() == '0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2'.lower()]
                
                if weth_logs:
                    print(f"WETH events found: {len(weth_logs)}")
                    print(f"\nRun: cargo run --example json_state_validator_no_rpc -- {tx['hash']}")
                    exit(0)

print("\nNo suitable DEX transactions found")