#!/usr/bin/env python3
"""
Basic Pool Check Example

Demonstrates the simplest usage of pool_buy_sell_simulator to check
if a token can be bought and sold on a Uniswap V2 pool.

This example uses USDC as it's a well-known, stable token that should
always be tradeable without taxes.
"""

import sys
import os
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))

import pyreth


def main():
    print("=" * 60)
    print("Basic Pool Buy/Sell Check - USDC on Uniswap V2")
    print("=" * 60)
    print()
    
    # Token and pool addresses
    USDC_ADDRESS = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
    USDC_WETH_V2_POOL = "0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc"
    
    try:
        # Create PyReth instance (singleton pattern)
        reth = pyreth.PyReth()
        print("✅ Connected to Reth database")
        
        # Get the pool buy sell simulator
        simulator = reth.pool_buy_sell_simulator()
        print("✅ Pool Buy Sell Simulator initialized")
        print()
        
        # Check if USDC can be bought and sold on Uniswap V2
        print(f"Testing USDC trading viability:")
        print(f"  Token: {USDC_ADDRESS}")
        print(f"  Pool:  {USDC_WETH_V2_POOL}")
        print(f"  Type:  Uniswap V2")
        print()
        
        print("Running buy → approve → sell simulation...")
        
        # Check the pool (uses default config: 0.01 ETH test amount)
        result = simulator.check_uniswap_v2_pool(
            token_address=USDC_ADDRESS,
            pool_address=USDC_WETH_V2_POOL
        )
        
        # Display results
        print()
        print("Results:")
        print("--------")
        print(f"✅ Can Buy:     {result.can_buy}")
        print(f"✅ Can Approve: {result.can_approve}")
        print(f"✅ Can Sell:    {result.can_sell}")
        print()
        
        print(f"Tax Analysis:")
        print(f"  Buy Tax:  {result.buy_tax_percentage:.2f}%")
        print(f"  Sell Tax: {result.sell_tax_percentage:.2f}%")
        print()
        
        print(f"Block Number: {result.block_number}")
        print(f"Pool Type: {result.pool_type}")
        
        if result.error_message:
            print(f"⚠️  Error: {result.error_message}")
        else:
            print(f"✅ Token is fully tradeable!")
            
    except AttributeError as e:
        if "pool_buy_sell_simulator" in str(e):
            print("❌ Error: pool_buy_sell_simulator method not found")
            print("   Make sure you have the latest PyReth build")
            print("   Try: cd /home/nima/code/crypto/rust/pyreth && maturin develop")
        else:
            raise
    except Exception as e:
        print(f"❌ Error: {e}")
        import traceback
        traceback.print_exc()


if __name__ == "__main__":
    main()