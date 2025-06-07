#!/usr/bin/env python3
"""Find a transaction with internal transfers for testing"""

import json
import subprocess

# Known transaction with internal transfers from Etherscan
# This is a UniswapV3 router transaction with multiple internal transfers
tx_hash = "0xb5c8bd9430b6cc87a0e2fe110ece6bf527fa4f170a4bc8cd032f768fc5219838"

# Run the validator
cmd = [
    "cargo", "run", "--example", "json_state_validator_no_rpc", "--", tx_hash
]

print(f"Testing transaction: {tx_hash}")
print("This transaction has multiple internal ETH transfers through UniswapV3")
print("-" * 80)

result = subprocess.run(cmd, capture_output=True, text=True)
print(result.stderr)
if result.stdout:
    try:
        data = json.loads(result.stdout)
        print(json.dumps(data, indent=2))
    except:
        print(result.stdout)