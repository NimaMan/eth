#!/usr/bin/env python3
"""
Account State Query Example

Demonstrates how to query account state including:
- ETH balance
- Account nonce
- ERC20 token balances
"""

import ethtx

def main():
    # Initialize simulator
    sim = ethtx.Simulator()
    
    # Example addresses
    VITALIK = "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045"  # vitalik.eth
    USDC = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
    USDT = "0xdAC17F958D2ee523a2206206994597C13D831ec7"
    DAI = "0x6B175474E89094C44Da98b954EedeAC495271d0F"
    
    # Get latest block number
    latest_block = sim.get_latest_block()
    print(f"Latest block: {latest_block}")
    
    # Get base fee
    base_fee = sim.get_latest_base_fee()
    print(f"Current base fee: {base_fee / 1e9:.2f} gwei")
    
    # Query at a specific block (recent block)
    query_block = latest_block - 10  # 10 blocks ago
    print(f"\nQuerying at block: {query_block}")
    
    # Get ETH balance
    balance_wei = sim.get_balance(VITALIK, query_block)
    balance_eth = int(balance_wei) / 1e18
    print(f"\n{VITALIK} (vitalik.eth)")
    print(f"  ETH Balance: {balance_eth:.4f} ETH")
    
    # Get account nonce
    nonce = sim.get_nonce(VITALIK, query_block)
    print(f"  Nonce: {nonce}")
    
    # Get ERC20 token balances
    print(f"\n  Token Balances:")
    
    # USDC (6 decimals)
    usdc_balance = sim.get_token_balance(USDC, VITALIK, query_block)
    if int(usdc_balance) > 0:
        usdc_formatted = int(usdc_balance) / 1e6
        print(f"    USDC: {usdc_formatted:,.2f}")
    else:
        print(f"    USDC: 0")
    
    # USDT (6 decimals)
    usdt_balance = sim.get_token_balance(USDT, VITALIK, query_block)
    if int(usdt_balance) > 0:
        usdt_formatted = int(usdt_balance) / 1e6
        print(f"    USDT: {usdt_formatted:,.2f}")
    else:
        print(f"    USDT: 0")
    
    # DAI (18 decimals)
    dai_balance = sim.get_token_balance(DAI, VITALIK, query_block)
    if int(dai_balance) > 0:
        dai_formatted = int(dai_balance) / 1e18
        print(f"    DAI: {dai_formatted:,.2f}")
    else:
        print(f"    DAI: 0")
    
    # Example: Check balance before simulating a transaction
    print("\n" + "="*50)
    print("Pre-simulation Balance Check Example")
    print("="*50)
    
    sender = "0x0C96c602b1b332B8AB2093E5d72D804a24bd5689"
    
    # Get current state
    sender_balance = sim.get_balance(sender)
    sender_nonce = sim.get_nonce(sender)
    
    print(f"\nSender: {sender}")
    print(f"  Current Balance: {int(sender_balance) / 1e18:.6f} ETH")
    print(f"  Current Nonce: {sender_nonce}")
    
    # Build a transaction
    tx = sim.build_transaction(
        from_address=sender,
        to_address=VITALIK,
        value="1000000000000000",  # 0.001 ETH
        gas_limit=21000
    )
    
    # Check if sender has enough balance
    value = int(tx["value"])
    gas_limit = int(tx.get("gas", tx.get("gas_limit", 21000)))  # Check both keys
    gas_price = int(tx.get("gas_price", 20000000000))
    gas_cost = gas_limit * gas_price
    total_cost = value + gas_cost
    
    print(f"\nTransaction Cost Analysis:")
    print(f"  Value: {value / 1e18:.6f} ETH")
    print(f"  Max Gas Cost: {gas_cost / 1e18:.6f} ETH")
    print(f"  Total Cost: {total_cost / 1e18:.6f} ETH")
    
    if int(sender_balance) >= total_cost:
        print(f"  ✓ Sufficient balance")
    else:
        print(f"  ✗ Insufficient balance (need {(total_cost - int(sender_balance)) / 1e18:.6f} more ETH)")

if __name__ == "__main__":
    main()