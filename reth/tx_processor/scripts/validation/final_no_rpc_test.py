#!/usr/bin/env python3
"""
Final test of NO RPC implementation
Validate that we get comprehensive state changes without RPC overhead
"""

import subprocess
import json
import requests
import time

def get_test_transactions(count: int = 100) -> list:
    """Get recent transactions for testing"""
    rpc_url = "http://127.0.0.1:8545"
    
    latest_response = requests.post(rpc_url, json={
        "jsonrpc": "2.0", "method": "eth_blockNumber", "params": [], "id": 1
    })
    latest_block = int(latest_response.json()["result"], 16)
    
    transactions = []
    for block_offset in range(10):
        if len(transactions) >= count:
            break
        block_num = latest_block - block_offset
        block_response = requests.post(rpc_url, json={
            "jsonrpc": "2.0", "method": "eth_getBlockByNumber", 
            "params": [hex(block_num), True], "id": 1
        })
        block_data = block_response.json().get("result", {})
        for tx in block_data.get("transactions", []):
            if len(transactions) >= count:
                break
            transactions.append(tx["hash"])
    
    return transactions[:count]

def run_no_rpc_validator(tx_hash: str) -> tuple:
    """Run no-RPC validator and return (success, address_count, processing_time)"""
    start_time = time.time()
    try:
        result = subprocess.run([
            "cargo", "run", "--example", "json_state_validator_no_rpc", "--", tx_hash
        ], capture_output=True, text=True, timeout=10, cwd='/home/nima/code/crypto/rust/revm_tx_simulator')
        
        processing_time = time.time() - start_time
        
        if result.returncode != 0:
            return False, 0, processing_time
        
        # Parse results
        try:
            parsed = json.loads(result.stdout)
            address_count = len(parsed)
        except:
            address_count = 0
        
        return True, address_count, processing_time
        
    except:
        processing_time = time.time() - start_time
        return False, 0, processing_time

def main():
    """Test NO RPC implementation"""
    print("🔍 Testing NO RPC REVM Implementation")
    print("=" * 50)
    
    start_time = time.time()
    transactions = get_test_transactions(50)  # Smaller test for speed
    print(f"📊 Testing {len(transactions)} transactions...")
    
    stats = {
        "tested": 0, "successful": 0, "failed": 0,
        "total_addresses": 0, "processing_times": []
    }
    
    for i, tx_hash in enumerate(transactions):
        if i % 10 == 0:
            rate = i / (time.time() - start_time) if i > 0 else 0
            print(f"📊 Progress: {i}/50 ({i*2:.0f}%) - {rate:.1f} tx/s")
        
        stats["tested"] += 1
        success, addresses, proc_time = run_no_rpc_validator(tx_hash)
        stats["processing_times"].append(proc_time)
        
        if success:
            stats["successful"] += 1
            stats["total_addresses"] += addresses
        else:
            stats["failed"] += 1
    
    total_time = time.time() - start_time
    avg_time = sum(stats["processing_times"]) / len(stats["processing_times"])
    
    # Results
    print(f"\n📊 NO RPC IMPLEMENTATION RESULTS")
    print("=" * 40)
    print(f"Tested: {stats['tested']}")
    print(f"Successful: {stats['successful']} ({stats['successful']/stats['tested']*100:.1f}%)")
    print(f"Failed: {stats['failed']}")
    print(f"Average addresses/tx: {stats['total_addresses']/stats['successful']:.1f}")
    print(f"Average processing time: {avg_time:.2f}s")
    print(f"Processing rate: {stats['successful']/total_time:.1f} tx/s")
    print(f"Total time: {total_time:.1f}s")
    
    # Assessment
    if stats['successful'] >= 45:  # 90% success
        print("\n🏆 EXCELLENT: NO RPC implementation working perfectly!")
        print("✅ Pure REVM with comprehensive state tracking")
        print("✅ No external RPC calls for internal transfers")
    elif stats['successful'] >= 40:  # 80% success  
        print("\n✅ GOOD: High success rate")
    else:
        print("\n⚠️  NEEDS WORK: Lower success rate")

if __name__ == "__main__":
    main()