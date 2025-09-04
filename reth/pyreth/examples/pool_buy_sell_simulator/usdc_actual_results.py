#!/usr/bin/env python3
"""
USDC Actual Simulation Results

Shows ONLY the real results from the simulation without any hardcoded values or approximations.
"""

import sys
import os
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))

import pyreth


def main():
    print("=" * 80)
    print("USDC 1 ETH Buy/Approve/Sell - ACTUAL RESULTS ONLY")
    print("=" * 80)
    print()
    
    # Configuration
    USDC_ADDRESS = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
    USDC_WETH_V2_POOL = "0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc"
    TEST_AMOUNT_ETH = 1.0
    
    try:
        # Initialize
        reth = pyreth.PyReth()
        simulator = reth.pool_buy_sell_simulator()
        
        # Create config for 1 ETH
        config = pyreth.PoolViabilityConfig()
        config.test_amount_eth = TEST_AMOUNT_ETH
        config.token_decimals = 6  # USDC decimals
        config.buyer_address = "0x0C96c602b1b332B8AB2093E5d72D804a24bd5689"
        
        print(f"Simulating with {TEST_AMOUNT_ETH} ETH on block...")
        print()
        
        # Run simulation - this is the ACTUAL blockchain simulation
        result = simulator.check_uniswap_v2_pool(
            token_address=USDC_ADDRESS,
            pool_address=USDC_WETH_V2_POOL,
            config=config
        )
        
        # Show ONLY actual results from the simulation
        print("ACTUAL SIMULATION RESULTS:")
        print("-" * 40)
        print(f"Block Number: {result.block_number}")
        print(f"Pool Type: {result.pool_type}")
        print()
        
        print("Transaction Status:")
        print(f"  Buy:     {'✅ SUCCESS' if result.can_buy else '❌ FAILED'}")
        print(f"  Approve: {'✅ SUCCESS' if result.can_approve else '❌ FAILED'}")
        print(f"  Sell:    {'✅ SUCCESS' if result.can_sell else '❌ FAILED'}")
        print()
        
        print("Detected Taxes (from actual balance changes):")
        print(f"  Buy Tax:  {result.buy_tax_percentage:.4f}%")
        print(f"  Sell Tax: {result.sell_tax_percentage:.4f}%")
        print()
        
        if result.error_message:
            print(f"Error Message: {result.error_message}")
        
        # The simulation internally tracks:
        # 1. ETH spent (1 ETH = 1000000000000000000 wei)
        # 2. USDC tokens received from buy
        # 3. ETH received back from sell
        # These are the ACTUAL amounts from the simulated blockchain state
        
        print("What the buyer gets:")
        print("-" * 40)
        print("1. Buyer sends 1 ETH to router")
        print("2. Router swaps ETH for USDC in pool")
        print("3. Buyer receives USDC tokens (actual amount from simulation)")
        print("4. Buyer approves router to spend USDC")
        print("5. Router swaps USDC back to ETH")
        print("6. Buyer receives ETH back (minus DEX fees)")
        print()
        
        is_tradeable = result.can_buy and result.can_approve and result.can_sell
        print(f"Final Result: {'✅ TRADEABLE' if is_tradeable else '❌ NOT TRADEABLE'}")
        
    except Exception as e:
        print(f"Error: {e}")
        import traceback
        traceback.print_exc()


if __name__ == "__main__":
    main()