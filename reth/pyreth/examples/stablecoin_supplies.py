#!/usr/bin/env python3
"""
Example: Get current stablecoin supplies using pyreth ChainQuery

This demonstrates using the ChainQuery module to get on-chain data
without needing RPC calls - directly from the Reth database.
"""

import pyreth
import sys
sys.path.append('/home/nima/code/crypto/py')
from eth_data.chain_utils.common_addresses.stablecoin_addresses import STABLECOINS_ADDRESS_BY_NAME as STABLECOINS

def main():
    # Initialize ChainQuery
    query = pyreth.ChainQuery()
    print("Connected to Reth database")
    print("=" * 60)
    
    # Get total supplies for all major stablecoins
    print("Current Stablecoin Supplies:")
    print("-" * 60)
    
    total_market_cap = 0
    
    for name, address in STABLECOINS.items():
        try:
            # Get total supply and decimals
            total_supply_str = query.get_token_total_supply(address)
            decimals = query.get_token_decimals(address)
            
            # Convert to human-readable format
            total_supply = int(total_supply_str)
            supply_formatted = total_supply / (10 ** decimals)
            
            print(f"{name:10} ${supply_formatted:>20,.2f}")
            total_market_cap += supply_formatted
            
        except Exception as e:
            print(f"{name:10} Error: {e}")
    
    print("-" * 60)
    print(f"{'Total':10} ${total_market_cap:>20,.2f}")
    print("=" * 60)
    
    # Example: Get specific account balance
    print("\nExample: Get account balance")
    print("-" * 60)
    
    # Binance hot wallet
    binance_address = "0xF977814e90dA44bFA03b6295A0616a897441aceC"
    
    # Get ETH balance
    eth_balance_wei = query.get_balance(binance_address)
    eth_balance = int(eth_balance_wei) / 1e18
    print(f"Binance ETH Balance: {eth_balance:,.4f} ETH")
    
    # Get USDT balance  
    usdt_address = STABLECOINS["USDT"]
    usdt_balance_str = query.get_token_balance(usdt_address, binance_address)
    usdt_decimals = query.get_token_decimals(usdt_address)
    usdt_balance = int(usdt_balance_str) / (10 ** usdt_decimals)
    print(f"Binance USDT Balance: ${usdt_balance:,.2f}")

if __name__ == "__main__":
    main()