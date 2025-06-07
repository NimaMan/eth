#!/usr/bin/env python3
"""Run Python transaction simulator"""

import sys
import os
sys.path.insert(0, '/home/nima/code/crypto/py/eth_block_processor')

from eth_block_processor.txn.tx_simulator import TxSimulator

# Initialize simulator
tx_simulator = TxSimulator(
    w3_http_url="http://127.0.0.1:8545",
    use_local_ganache=False
)

# Transaction to simulate
tx_hash = "0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae"

print(f"Running Python simulation for: {tx_hash}")
print("-" * 80)

try:
    # Simulate transaction
    result = tx_simulator.simulate_historical_tx(tx_hash)
    
    if result:
        print("\nState changes from Python:")
        import json
        # Convert result to JSON-friendly format
        state_changes = {}
        for addr, changes in result.items():
            if hasattr(changes, 'to_dict'):
                state_changes[addr] = changes.to_dict()
            else:
                state_changes[addr] = str(changes)
        
        print(json.dumps(state_changes, indent=2))
    else:
        print("No result from Python simulator")
        
except Exception as e:
    print(f"Error: {e}")
    import traceback
    traceback.print_exc()