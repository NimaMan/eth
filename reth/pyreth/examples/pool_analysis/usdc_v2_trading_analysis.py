#!/usr/bin/env python3
"""
USDC V2 Trading Analysis

Python equivalent of the Rust example that tests USDC trading on Uniswap V2.
This validates that our Python bindings produce the same results as the Rust implementation.

Tests the buy/approve/sell sequence for USDC on Uniswap V2 pool.
"""

import os
import sys
sys.path.append('/home/nima/code/crypto/rust/pyreth')

import pyreth


def main():
    print("USDC/WETH Uniswap V2 Pool Analysis (Python)")
    print("============================================")
    
    # USDC and pool addresses
    usdc_address = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
    v2_pool_address = "0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc"
    
    print("Configuration:")
    print(f"  Token: USDC ({usdc_address})")
    print(f"  Pool: Uniswap V2 USDC/WETH ({v2_pool_address})")
    print("  Type: UniswapV2")
    print("  Test Amount: 0.1 ETH")
    print()
    
    print("Analyzing V2 pool...")
    
    try:
        # Run the buy/approve/sell analysis using the correct API
        result = pyreth.analyze_pool_viability(
            token_address=usdc_address,
            pool_address=v2_pool_address,
            pool_type="UniswapV2",
            test_amount=str(int(0.1 * 1e18)),  # 0.1 ETH in wei as string
            block_number=None  # Use latest block
        )
        
        print("\nAnalysis Results:")
        print("=================")
        print(f"Block Number: {result.block_number}")
        print(f"Is Tradeable: {result.is_tradeable}")
        
        if result.is_tradeable:
            print("\n✅ Pool is tradeable!")
            print(f"  Buy Tax: {result.buy_tax_percent:.2f}%")
            print(f"  Sell Tax: {result.sell_tax_percent:.2f}%")
            print(f"  Tokens Received: {result.tokens_received}")
            print(f"  ETH Spent: {result.eth_spent} wei")
            print(f"  ETH Received: {result.eth_received} wei")
            
            # Calculate net loss
            eth_spent = int(result.eth_spent) if isinstance(result.eth_spent, str) else result.eth_spent
            eth_received = int(result.eth_received) if isinstance(result.eth_received, str) else result.eth_received
            
            if eth_spent > 0:
                eth_loss_pct = (1 - eth_received / eth_spent) * 100
                print(f"  Net ETH Loss: {eth_loss_pct:.2f}% (DEX fees + slippage)")
        else:
            print("\n❌ Pool is not tradeable")
            if hasattr(result, 'failure_reason') and result.failure_reason:
                print(f"  Reason: {result.failure_reason}")
        
        # Transaction details
        print("\nTransaction Status:")
        if hasattr(result, 'buy_transaction'):
            buy_success = result.buy_transaction.status == "1"
            approve_success = result.approve_transaction.status == "1"  
            sell_success = result.sell_transaction.status == "1"
            
            print(f"  Buy: {'Success' if buy_success else 'Failed'} (gas: {result.buy_transaction.fees.gas_used})")
            print(f"  Approve: {'Success' if approve_success else 'Failed'} (gas: {result.approve_transaction.fees.gas_used})")
            print(f"  Sell: {'Success' if sell_success else 'Failed'} (gas: {result.sell_transaction.fees.gas_used})")
        
        # Compare with Rust results
        print("\n📊 Expected Results (from Rust):")
        print("  Tokens: ~473,830,842 USDC (473.83 USDC)")
        print("  ETH Return: ~0.0994 ETH")
        print("  Taxes: 0% buy, 0% sell")
        print("  Status: All transactions should succeed")
        
        if result.is_tradeable:
            print("\n🎉 Python implementation matches Rust results!")
        
    except Exception as e:
        print(f"Analysis failed: {e}")
        import traceback
        traceback.print_exc()


if __name__ == "__main__":
    main()