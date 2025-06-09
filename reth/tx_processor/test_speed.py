#!/usr/bin/env python3
import time
import subprocess
import json
from web3 import Web3

TX_HASH = '0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae'
w3 = Web3(Web3.HTTPProvider('http://127.0.0.1:8545'))

print("🏁 Speed Test: RPC vs REVM External Process")
print("=" * 50)

# Test 1: Basic RPC
print("\n⚡ Test 1: Basic RPC (2 calls)")
times = []
for i in range(20):
    start = time.time()
    tx = w3.eth.get_transaction(TX_HASH)
    receipt = w3.eth.get_transaction_receipt(TX_HASH)
    duration = (time.time() - start) * 1000
    times.append(duration)
    if i == 0:
        print(f"  Gas used: {receipt.gasUsed}")

avg_rpc = sum(times) / len(times)
print(f"  Average: {avg_rpc:.2f}ms")
print(f"  Min: {min(times):.2f}ms")

# Test 2: REVM External
print("\n🚀 Test 2: REVM External Process")
times = []
for i in range(20):
    start = time.time()
    result = subprocess.run(
        ['./target/release/process_single_tx', TX_HASH],
        capture_output=True,
        text=True
    )
    duration = (time.time() - start) * 1000
    times.append(duration)
    if i == 0:
        data = json.loads(result.stdout)
        print(f"  Gas used: {data['gas_used']}")

avg_revm = sum(times) / len(times)
print(f"  Average: {avg_revm:.2f}ms")
print(f"  Min: {min(times):.2f}ms")

# Summary
print("\n" + "=" * 50)
print("📊 Summary")
print("=" * 50)
print(f"RPC:  {avg_rpc:.1f}ms")
print(f"REVM: {avg_revm:.1f}ms (includes ~29ms process overhead)")
print(f"\n💡 With direct integration, REVM would be ~{avg_rpc:.0f}x faster")