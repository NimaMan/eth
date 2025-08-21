#!/usr/bin/env python3
"""
ChainQuery Examples - Direct blockchain database queries
"""

import pyreth
import sys
sys.path.append('/home/nima/code/crypto/py')

def main():
    print("=" * 60)
    print("ChainQuery Examples")
    print("=" * 60)
    
    # Initialize ChainQuery
    query = pyreth.ChainQuery()
    print(f"Connected to: {query}")
    
    # Example 1: Get account information
    print("\n1. Account Information")
    print("-" * 40)
    
    addresses = {
        "Vitalik": "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045",
        "Binance": "0xF977814e90dA44bFA03b6295A0616a897441aceC",
        "Coinbase": "0xA9D1e08C7793af67e9d92fe308d5697FB81d3E43"
    }
    
    for name, addr in addresses.items():
        balance_wei = query.get_balance(addr)
        nonce = query.get_nonce(addr)
        balance_eth = int(balance_wei) / 1e18
        
        print(f"{name:10} ({addr[:10]}...)")
        print(f"  Balance: {balance_eth:,.2f} ETH")
        print(f"  Nonce:   {nonce}")
    
    # Example 2: Get token information
    print("\n2. Token Information")
    print("-" * 40)
    
    tokens = {
        "USDC": "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
        "USDT": "0xdAC17F958D2ee523a2206206994597C13D831ec7",
        "DAI":  "0x6B175474E89094C44Da98b954EedeAC495271d0F",
        "WETH": "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"
    }
    
    for name, token_addr in tokens.items():
        try:
            supply = query.get_token_total_supply(token_addr)
            decimals = query.get_token_decimals(token_addr)
            supply_human = int(supply) / (10 ** decimals)
            
            print(f"{name:5} Total Supply: ${supply_human:,.2f}")
        except Exception as e:
            print(f"{name:5} Error: {e}")
    
    # Example 3: Get token balances
    print("\n3. Token Balances")
    print("-" * 40)
    
    # Check Binance's stablecoin holdings
    binance = addresses["Binance"]
    print(f"Binance Stablecoin Holdings:")
    
    for name, token_addr in [("USDC", tokens["USDC"]), ("USDT", tokens["USDT"]), ("DAI", tokens["DAI"])]:
        try:
            balance = query.get_token_balance(token_addr, binance)
            decimals = query.get_token_decimals(token_addr)
            balance_human = int(balance) / (10 ** decimals)
            
            if balance_human > 0:
                print(f"  {name}: ${balance_human:,.2f}")
        except Exception as e:
            print(f"  {name}: Error - {e}")
    
    # Example 4: Get storage values
    print("\n4. Direct Storage Access")
    print("-" * 40)
    
    # Get WETH balance storage slot for an address
    weth = tokens["WETH"]
    slot_index = "0x0000000000000000000000000000000000000000000000000000000000000003"
    
    try:
        storage_value = query.get_storage_at(weth, slot_index)
        print(f"WETH Storage Slot 3: {storage_value}")
    except Exception as e:
        print(f"Storage access error: {e}")
    
    print("\n" + "=" * 60)
    print("✅ All ChainQuery examples completed!")

if __name__ == "__main__":
    main()