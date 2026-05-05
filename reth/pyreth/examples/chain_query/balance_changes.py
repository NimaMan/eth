#!/usr/bin/env python3
"""
Example: Track balance changes between blocks
"""

from pyreth import chain_query as pyreth_chain_query
from eth_token.erc20_token.pools.addresses import require_checksum_address, same_address

def format_wei(wei_str):
    """Format wei to ETH with 4 decimals"""
    return f"{int(wei_str) / 10**18:.4f}"

def format_token_amount(amount_str, decimals=6):
    """Format token amount with proper decimals"""
    return f"{int(amount_str) / 10**decimals:,.2f}"

def main():
    # Create PyReth instance
    query = pyreth_chain_query()
    
    print("=== Chain Query: Balance Changes ===\n")
    
    # Get latest block
    latest_block = query.get_latest_block()
    print(f"Latest block: {latest_block}")
    
    # Set block range (last 100 blocks)
    from_block = latest_block - 100
    to_block = latest_block
    
    print(f"Analyzing blocks {from_block} to {to_block}")
    print()
    
    # Test addresses
    addresses = [
        ("Vitalik", "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045"),
        ("Binance", "0xF977814e90dA44bFA03b6295A0616a897441aceC"),
    ]
    
    # Token addresses
    usdc = require_checksum_address("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48")
    usdt = require_checksum_address("0xdAC17F958D2ee523a2206206994597C13D831ec7")
    
    tokens = [usdc, usdt]
    
    for name, addr in addresses:
        print(f"Balance changes for {name}")
        print("-" * 40)
        
        try:
            # Get balance changes
            changes = query.get_balance_changes(addr, tokens, from_block, to_block)
            
            # Display ETH changes
            print(f"ETH Changes:")
            print(f"  Before: {format_wei(changes.eth_change.before)} ETH")
            print(f"  After:  {format_wei(changes.eth_change.after)} ETH")
            
            if changes.eth_change.is_increase:
                print(f"  Change: +{format_wei(changes.eth_change.difference)} ETH ↑")
            else:
                # For decrease, the difference is already negative in the string
                diff_wei = changes.eth_change.difference.lstrip('-')
                print(f"  Change: -{format_wei(diff_wei)} ETH ↓")
            
            # Display token changes (if any)
            token_changes = changes.token_changes()
            if token_changes:
                print(f"\nToken Changes:")
                for token_addr, change_data in token_changes.items():
                    token_name = "USDC" if same_address(token_addr, usdc) else "USDT"
                    print(f"  {token_name}: {change_data}")
            
        except Exception as e:
            print(f"  Error getting changes: {e}")
        
        print()
    
    # Example: Find significant balance changes
    print("Finding Significant Changes")
    print("-" * 40)
    
    # Check multiple addresses for significant ETH movements
    exchange_addresses = [
        ("Coinbase", "0xA9D1e08C7793af67e9d92fe308d5697FB81d3E43"),
        ("Kraken", "0x53d284357ec70cE289D6D64134DfAc8E511c8a3D"),
    ]
    
    print("Checking exchange balance changes...")
    for name, addr in exchange_addresses:
        try:
            changes = query.get_balance_changes(addr, [], from_block, to_block)
            
            before_eth = int(changes.eth_change.before) / 10**18
            after_eth = int(changes.eth_change.after) / 10**18
            diff_eth = after_eth - before_eth
            
            if abs(diff_eth) > 0.1:  # Significant if > 0.1 ETH
                symbol = "↑" if diff_eth > 0 else "↓"
                print(f"  {name}: {diff_eth:+.4f} ETH {symbol}")
        except Exception as e:
            print(f"  {name}: Error - {e}")
    
    print()
    
    # Example: Portfolio tracking
    print("Portfolio Tracking")
    print("-" * 40)
    
    vitalik_addr = addresses[0][1]
    
    # Get portfolio at two different blocks
    try:
        portfolio_before = query.get_portfolio(vitalik_addr, tokens, from_block)
        portfolio_after = query.get_portfolio(vitalik_addr, tokens, to_block)
        
        print(f"Vitalik's portfolio changes:")
        print(f"  Block {from_block}: {format_wei(portfolio_before.eth_balance)} ETH")
        print(f"  Block {to_block}: {format_wei(portfolio_after.eth_balance)} ETH")
        
        eth_diff = (int(portfolio_after.eth_balance) - int(portfolio_before.eth_balance)) / 10**18
        if eth_diff != 0:
            print(f"  Net change: {eth_diff:+.4f} ETH")
        else:
            print(f"  No change in ETH balance")
            
    except Exception as e:
        print(f"Error tracking portfolio: {e}")
    
    print("\n✅ Balance change tracking complete!")

if __name__ == "__main__":
    main()
