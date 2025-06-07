#!/usr/bin/env python3
"""Compare Python and Rust implementations for the given transaction"""

import subprocess
import json
import sys

tx_hash = "0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae"

print(f"Transaction: {tx_hash}")
print("=" * 100)

# Run Python implementation
print("\n## PYTHON IMPLEMENTATION")
print("-" * 50)
python_cmd = [
    "python", "-m", "eth_tx_manager.simulation.eth_tx_simulation", 
    "--tx_hash", tx_hash,
    "--eth_rpc_url", "http://127.0.0.1:8545"
]

try:
    # Change to Python directory
    import os
    os.chdir("/home/nima/code/crypto/py/eth_tx_manager")
    
    result = subprocess.run(python_cmd, capture_output=True, text=True)
    if result.stdout:
        try:
            python_data = json.loads(result.stdout)
            print(json.dumps(python_data, indent=2))
        except:
            print(result.stdout)
    if result.stderr:
        print("STDERR:", result.stderr)
except Exception as e:
    print(f"Error running Python: {e}")

# Summary of Rust results (already captured above)
print("\n## RUST IMPLEMENTATION (Summary)")
print("-" * 50)
rust_summary = """
Key addresses and their net ETH changes:
- 0x5b43453f... (sender): -0.000000000022646153 ETH (transaction value)
- 0xfbd4cdb4... (router): -23.055 ETH (sent out WETH)
- 0x6bdf3535... (intermediate): +10.829 ETH (received WETH from unwrap)
- 0x3177f690... (intermediate): +5.496 ETH (received WETH from unwrap)
- 0x00000000... (Uniswap V4 Pool): +16.325 ETH (received ETH from both intermediates)
- 0x11b815ef... (Uniswap V3 Pool): +6.730 ETH (received WETH)
- 0xc02aaa39... (WETH contract): -16.325 ETH (unwrapped to ETH)

Token movements:
- USDC: 27,158.42 moved through the system
- USDT: 16,865.02 + 13,772.16 = 30,637.18 total USDT moved
"""
print(rust_summary)

print("\n## KEY DIFFERENCES TO NOTE")
print("-" * 50)
print("""
1. Python filters out addresses with ETH changes < 0.0005 ETH
2. Rust shows ALL state changes, including gas fees to validators
3. Both treat WETH transfers as ETH movements
4. Internal transfers are now captured by CallTracer in Rust
5. Token decimals are properly handled (USDC/USDT with 6 decimals)
""")