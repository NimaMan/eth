#!/usr/bin/env python3
"""
moo Token Trading Analysis

Python equivalent of the Rust example that tests the moo token from a specific transaction.
Validates simulation against real transaction: 0xea83d46948ebd529ad26370a5f46341a6a84f3c2f1b07caf7cf94aa232283115

This example simulates buy/approve/sell at the same block where we know
the token was tradeable to verify our simulation matches reality.
"""

import os
import sys
sys.path.append('/home/nima/code/crypto/rust/pyreth')

import pyreth
from pyreth import ChainQuery, TradingSimulator, PoolType


def main():
    print("moo Token Trading Viability Analysis (Python)")
    print("==============================================")
    print("Reference TX: 0xea83d46948ebd529ad26370a5f46341a6a84f3c2f1b07caf7cf94aa232283115")
    print("Block: 23196488")
    print("Original swap: 0.1 ETH -> 575,446,178 moo tokens")
    print()
    
    # Create components using Python bindings
    chain_query = ChainQuery()
    simulator = chain_query.get_simulator()
    
    # moo token details from the transaction
    moo_token_address = "0xDF6010eF80142D379eA0324ac100Dd3Cf50901b2"
    moo_pool_address = "0xFc099D07b32D52D61d2f5Dd6De2614d26474eCf7"
    target_block = 23196488
    
    print("Configuration:")
    print(f"  Token: moo ({moo_token_address})")
    print(f"  Pool: Uniswap V2 moo/WETH ({moo_pool_address})")
    print("  Type: UniswapV2")
    print("  Test Amount: 0.1 ETH")
    print(f"  Block: {target_block} (same as reference transaction)")
    print()
    
    print(f"Analyzing moo token at block {target_block}...")
    
    try:
        # Create trading simulator
        trading_sim = TradingSimulator(simulator)
        
        # Configure for moo token V2 trading at specific block
        config = {
            'token_address': moo_token_address,
            'pool_address': moo_pool_address,
            'pool_type': PoolType.UniswapV2,
            'test_amount': int(0.1 * 1e18),  # 0.1 ETH in wei
            'buyer_address': "0x0C96c602b1b332B8AB2093E5d72D804a24bd5689",  # Default buyer
            'block_number': target_block
        }
        
        # Run the buy/approve/sell analysis
        result = trading_sim.analyze_token_trading_viability(config)
        
        print("\nAnalysis Results:")
        print("=================")
        print(f"Block Number: {result['block_number']}")
        print(f"Is Tradeable: {result['is_tradeable']}")
        
        if result['is_tradeable']:
            print("\n✅ Pool is tradeable!")
            print(f"  Buy Tax: {result['buy_tax_percent']:.2f}%")
            print(f"  Sell Tax: {result['sell_tax_percent']:.2f}%")
            print(f"  Tokens Received: {result['tokens_received']}")
            print(f"  ETH Spent: {result['eth_spent']} wei")
            print(f"  ETH Received: {result['eth_received']} wei")
            
            # Compare with original transaction
            original_tokens = 575_446_178_536_175_301
            simulated_tokens = result['tokens_received']
            
            if original_tokens > 0:
                difference_pct = abs(original_tokens - simulated_tokens) / original_tokens * 100
                
                print("\n📊 Comparison with original transaction:")
                print(f"  Original tokens received: {original_tokens}")
                print(f"  Simulated tokens received: {simulated_tokens}")
                print(f"  Difference: {difference_pct:.2f}%")
                
                if difference_pct < 1.0:
                    print("  ✅ Simulation closely matches actual transaction!")
                else:
                    print("  ⚠️ Some difference detected (could be due to MEV, slippage, or fees)")
        else:
            print("\n❌ Pool is not tradeable")
            if 'failure_reason' in result:
                print(f"  Reason: {result['failure_reason']}")
        
        # Transaction details
        print("\nTransaction Status:")
        if 'transactions' in result:
            txs = result['transactions']
            print(f"  Buy: {'Success' if txs['buy']['success'] else 'Failed'} (gas: {txs['buy'].get('gas_used', 'N/A')})")
            print(f"  Approve: {'Success' if txs['approve']['success'] else 'Failed'} (gas: {txs['approve'].get('gas_used', 'N/A')})")
            print(f"  Sell: {'Success' if txs['sell']['success'] else 'Failed'} (gas: {txs['sell'].get('gas_used', 'N/A')})")
        
        # Expected results from Rust
        print("\n📊 Expected Results (from Rust):")
        print("  Tokens: ~497,409,164 moo tokens")
        print("  ETH Return: ~0.0994 ETH")
        print("  Taxes: 0% buy, 0% sell")
        print("  Status: All transactions should succeed")
        print("  Difference from real tx: ~13.56%")
        
    except Exception as e:
        print(f"Analysis failed: {e}")
        import traceback
        traceback.print_exc()


if __name__ == "__main__":
    main()