#!/usr/bin/env python3
"""Find a recent transaction involving WETH"""

import requests
import json

rpc_url = 'http://127.0.0.1:8545'

# WETH contract address
WETH_ADDRESS = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"

# Get latest block
payload = {
    'jsonrpc': '2.0',
    'method': 'eth_blockNumber',
    'params': [],
    'id': 1
}
response = requests.post(rpc_url, json=payload)
latest_block = int(response.json()['result'], 16)

print(f"Latest block: {latest_block}")

# Search last few blocks for WETH transfers
for block_num in range(latest_block - 5, latest_block + 1):
    print(f"\nChecking block {block_num}...")
    
    # Get block with transactions
    payload = {
        'jsonrpc': '2.0',
        'method': 'eth_getBlockByNumber',
        'params': [hex(block_num), True],
        'id': 1
    }
    response = requests.post(rpc_url, json=payload)
    block = response.json()['result']
    
    if not block or not block.get('transactions'):
        continue
        
    # Check each transaction
    for tx in block['transactions']:
        # Check if transaction involves WETH contract
        if tx.get('to') and tx['to'].lower() == WETH_ADDRESS.lower():
            print(f"\nFound WETH transaction!")
            print(f"Hash: {tx['hash']}")
            print(f"From: {tx['from']}")
            print(f"To: {tx['to']}")
            print(f"Value: {int(tx['value'], 16) / 1e18} ETH")
            
            # Get transaction receipt to see logs
            payload = {
                'jsonrpc': '2.0',
                'method': 'eth_getTransactionReceipt',
                'params': [tx['hash']],
                'id': 1
            }
            receipt_response = requests.post(rpc_url, json=payload)
            receipt = receipt_response.json()['result']
            
            if receipt and receipt.get('logs'):
                print(f"Logs: {len(receipt['logs'])}")
                
            print(f"\nRun with: cargo run --example json_state_validator_no_rpc -- {tx['hash']}")
            exit(0)

print("\nNo WETH transactions found in recent blocks")