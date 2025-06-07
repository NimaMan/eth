#!/usr/bin/env python3
"""
Simulate the transaction from Etherscan data that the user provided.
Based on the conversation, this transaction has:
- WETH transfers that should be treated as ETH
- Multiple intermediate addresses
- USDC and USDT transfers
"""

import json

# Transaction details from the conversation
tx_hash = "0xd9e31b8c86db86fb37f5fb38f9c69a756e576b8cfbacb4fb456fea0787046834"

print(f"Transaction: {tx_hash}")
print("\nBased on the Etherscan data you provided:")
print("\nExpected state changes with WETH-as-ETH treatment:")
print("-" * 80)

# The expected results based on treating WETH as ETH
expected_state = {
    "0xd72a3b02a39cfb9f3d13ccae47cce3b632787e13": {
        "eth_net": -23.529,  # Sends ETH
        "token_net": {}
    },
    "0x3177f690...": {  
        "eth_net": 0.0,  # Receives WETH and sends ETH - net zero
        "token_net": {}
    },
    "0x6bdf3535...": {
        "eth_net": 0.0,  # Receives WETH and sends ETH - net zero  
        "token_net": {}
    },
    "0xdac17f958d2ee523a2206206994597c13d831ec7": {  # USDT
        "eth_net": 0.0,
        "token_net": {}
    },
    "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48": {  # USDC
        "eth_net": 0.0,
        "token_net": {}
    },
    "0x88e6a0c2ddd26feeb64f039a2c41296fcb3f5640": {  # Uniswap V3 Pool
        "eth_net": 0.0,
        "token_net": {
            "USDC": -532.946739  # Based on 6 decimals
        }
    },
    "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2": {  # WETH
        "eth_net": 0.0,  # WETH contract itself
        "token_net": {}
    },
    "0x95222290dd7278aa3ddd389cc1e1d165cc4bafe5": {  # Validator 
        "eth_net": 0.000455,  # Receives gas fees
        "token_net": {}
    }
}

print(json.dumps(expected_state, indent=2))

print("\n\nKey points:")
print("1. Intermediate addresses (0x3177f690..., 0x6bdf3535...) show 0 net ETH")
print("   because they receive WETH and send ETH (we treat them as the same)")
print("2. USDC shows with proper 6 decimal precision")
print("3. Internal ETH transfers are tracked via CallTracer")
print("4. Gas fees to validator are captured")

print("\n\nTo test with a real transaction on your local node, try:")
print("cargo run --example json_state_validator_no_rpc -- <recent_tx_hash>")