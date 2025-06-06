#!/usr/bin/env python3
"""
Find the actual transaction hashes that Python detected as scams
by querying the blocks around the time of detection.
"""

import requests
import json
from datetime import datetime
import time

# Scam detection timestamps and details from Python logs
SCAM_DETECTIONS = [
    {
        "timestamp": "2025-06-05 15:40:49",
        "token": "0x6426b6C2A9108Fa815bcccA3a3232301E1895742",
        "pool": "0x548Db8fC431Dd7c39817BF0a59638B2bCA2eAcD5",
        "current_eth": 0.559,
        "simulated_eth": 0.289,
        "block_around": 22638767
    },
    {
        "timestamp": "2025-06-05 15:41:49", 
        "token": "0x8B77fE013C078ea92260589De96c1C5EE464da02",
        "pool": "0xB987d8E7A1F8594aD000B55c40b3deD8d8dfE93A",
        "current_eth": 6.811687,
        "simulated_eth": 0.0,
        "block_around": 22638772
    }
]

def rpc_call(method, params=None):
    """Make RPC call to local Reth node."""
    payload = {
        "jsonrpc": "2.0",
        "method": method,
        "params": params or [],
        "id": 1
    }
    
    response = requests.post("http://localhost:8545", 
                           headers={"Content-Type": "application/json"},
                           json=payload)
    
    if response.status_code == 200:
        result = response.json()
        if "result" in result:
            return result["result"]
        else:
            print(f"RPC Error: {result.get('error', 'Unknown error')}")
            return None
    else:
        print(f"HTTP Error {response.status_code}: {response.text}")
        return None

def get_block_with_transactions(block_number):
    """Get block with all transaction details."""
    block_hex = hex(block_number)
    return rpc_call("eth_getBlockByNumber", [block_hex, True])

def analyze_transaction_for_pool(tx, pool_address):
    """Check if transaction interacts with the specified pool."""
    if not tx:
        return False, None
    
    # Direct interaction (to address)
    to_address = tx.get("to")
    if to_address and to_address.lower() == pool_address.lower():
        return True, "direct_to"
    
    # Check if transaction input contains the pool address
    input_data = tx.get("input", "")
    if input_data and pool_address.lower()[2:] in input_data.lower():  # Remove 0x prefix
        return True, "input_data"
    
    return False, None

def find_transactions_for_scam(scam_info):
    """Find potential scam transactions around the detection time."""
    print(f"\n🔍 Searching for scam transactions:")
    print(f"   Time: {scam_info['timestamp']}")
    print(f"   Pool: {scam_info['pool']}")
    print(f"   ETH change: {scam_info['current_eth']} → {scam_info['simulated_eth']}")
    
    # Search a few blocks around the detection
    start_block = scam_info['block_around'] - 2
    end_block = scam_info['block_around'] + 2
    
    candidates = []
    
    for block_num in range(start_block, end_block + 1):
        print(f"   📦 Checking block {block_num}...")
        
        block = get_block_with_transactions(block_num)
        if not block:
            continue
            
        block_timestamp = int(block['timestamp'], 16)
        block_time = datetime.fromtimestamp(block_timestamp)
        
        print(f"      Block time: {block_time}")
        print(f"      Transactions: {len(block.get('transactions', []))}")
        
        for tx in block.get('transactions', []):
            is_pool_tx, interaction_type = analyze_transaction_for_pool(tx, scam_info['pool'])
            
            if is_pool_tx:
                candidates.append({
                    'hash': tx['hash'],
                    'block': block_num,
                    'from': tx['from'],
                    'to': tx.get('to'),
                    'value': int(tx['value'], 16) / 1e18 if tx['value'] != '0x0' else 0,
                    'interaction_type': interaction_type,
                    'gas': int(tx['gas'], 16),
                    'gas_price': int(tx.get('gasPrice', '0x0'), 16) / 1e9,  # Convert to gwei
                    'block_time': block_time
                })
                
                print(f"      ✅ Found pool interaction: {tx['hash']}")
                print(f"         Type: {interaction_type}")
                print(f"         Value: {candidates[-1]['value']:.6f} ETH")
    
    return candidates

def main():
    print("🔬 PYTHON SCAM TRANSACTION HASH FINDER")
    print("======================================")
    print()
    print("Searching for the actual transaction hashes that Python detected as scams...")
    print()
    
    all_candidates = {}
    
    for i, scam in enumerate(SCAM_DETECTIONS, 1):
        print(f"\n🚨 SCAM CASE {i}")
        print("=" * 50)
        
        candidates = find_transactions_for_scam(scam)
        all_candidates[f"scam_{i}"] = {
            "scam_info": scam,
            "candidates": candidates
        }
        
        if candidates:
            print(f"\n📊 Found {len(candidates)} candidate transactions:")
            for j, candidate in enumerate(candidates, 1):
                print(f"   {j}. {candidate['hash']}")
                print(f"      Block: {candidate['block']}")
                print(f"      Value: {candidate['value']:.6f} ETH")
                print(f"      Interaction: {candidate['interaction_type']}")
                print(f"      Time: {candidate['block_time']}")
                print()
        else:
            print("❌ No candidate transactions found")
            print("   This could mean:")
            print("   - The scam was detected in mempool (not yet mined)")
            print("   - Different block range needed")
            print("   - Complex DEX router interaction")
    
    # Output results for Rust testing
    print("\n🔧 RUST BENCHMARK INPUT")
    print("=" * 30)
    print("Copy these transaction hashes to test in Rust:")
    print()
    
    for scam_id, data in all_candidates.items():
        if data['candidates']:
            print(f"// {scam_id.upper()}: {data['scam_info']['pool']}")
            for candidate in data['candidates']:
                print(f'"{candidate["hash"]}",  // Block {candidate["block"]}, {candidate["value"]:.6f} ETH')
            print()
    
    # Save detailed results
    with open('/tmp/scam_candidates.json', 'w') as f:
        json.dump(all_candidates, f, indent=2, default=str)
    
    print(f"💾 Detailed results saved to /tmp/scam_candidates.json")

if __name__ == "__main__":
    main()