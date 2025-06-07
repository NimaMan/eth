#!/usr/bin/env python3
"""Find a Uniswap transaction with WETH and ETH movements"""

import requests
import json

rpc_url = 'http://127.0.0.1:8545'

# Known Uniswap V3 router that handles WETH
UNISWAP_V3_ROUTER = "0x68b3465833fb72A70ecDF485E0e4C7bD8665Fc45"

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
print(f"Looking for Uniswap V3 transactions with ETH value...")

# Search last few blocks
for block_num in range(latest_block - 10, latest_block + 1):
    print(f"\rChecking block {block_num}...", end='', flush=True)
    
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
        # Check if transaction is to Uniswap router with ETH value
        if (tx.get('to') and 
            tx['to'].lower() == UNISWAP_V3_ROUTER.lower() and 
            int(tx.get('value', '0x0'), 16) > 0):
            
            print(f"\n\nFound Uniswap V3 transaction with ETH!")
            print(f"Hash: {tx['hash']}")
            print(f"From: {tx['from']}")
            print(f"To: {tx['to']}")
            print(f"Value: {int(tx['value'], 16) / 1e18} ETH")
            
            # Get transaction receipt
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
                # Check for WETH events
                weth_logs = [log for log in receipt['logs'] 
                            if log['address'].lower() == '0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2'.lower()]
                if weth_logs:
                    print(f"WETH events: {len(weth_logs)}")
                
            print(f"\nRun with: cargo run --example json_state_validator_no_rpc -- {tx['hash']}")
            exit(0)

print("\n\nNo suitable transactions found")