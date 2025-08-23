#!/usr/bin/env python3
"""
Block Range Trading Analysis

Python equivalent of the Rust example that analyzes token trading across multiple blocks.
This helps detect when trading was enabled/disabled or tax changes occurred.

Analyzes the moo token across blocks around the liquidity addition to understand
the trading restriction period.
"""

import os
import sys
sys.path.append('/home/nima/code/crypto/rust/pyreth')

import pyreth
from pyreth import ChainQuery, TradingSimulator, PoolType


def analyze_block_range():
    print("Block Range Token Trading Analysis (Python)")
    print("===========================================")
    
    # Create components using Python bindings
    chain_query = ChainQuery()
    simulator = chain_query.get_simulator()
    
    # Configure token to analyze - using moo token
    token_address = "0xDF6010eF80142D379eA0324ac100Dd3Cf50901b2"  # moo token
    pool_address = "0xFc099D07b32D52D61d2f5Dd6De2614d26474eCf7"   # moo/WETH V2 pool
    
    # Define block range - around liquidity addition and trading events
    start_block = 23196189  # 10 blocks before liquidity addition
    end_block = 23196499    # 10 blocks after the successful swap
    block_step = 10         # Test every 10th block
    
    print("Configuration:")
    print(f"  Token: moo ({token_address})")
    print("  Pool: Uniswap V2 moo/WETH")
    print(f"  Block Range: {start_block} to {end_block}")
    print("  Key Events:")
    print("    - Block 23196199: Liquidity addition (1 ETH + 89B moo tokens)")
    print("    - Block 23196488: Successful swap (0.1 ETH -> 575M moo tokens)")
    print(f"  Step: Every {block_step} blocks")
    print("  Test Amount: 0.1 ETH")
    print()
    
    print("Starting block range analysis...\n")
    print(f"{'Block':<10} {'Tradeable':<12} {'Buy Tax':<10} {'Sell Tax':<10} {'Tokens Received':<20} {'ETH Received':<20}")
    print("=" * 92)
    
    results = []
    trading_enabled_block = None
    trading_disabled_block = None
    tax_changes = []
    last_buy_tax = None
    last_sell_tax = None
    
    try:
        # Create trading simulator
        trading_sim = TradingSimulator(simulator)
        
        for block_number in range(start_block, end_block + 1, block_step):
            # Configure for this specific block
            config = {
                'token_address': token_address,
                'pool_address': pool_address,
                'pool_type': PoolType.UniswapV2,
                'test_amount': int(0.1 * 1e18),  # 0.1 ETH in wei
                'buyer_address': "0x0C96c602b1b332B8AB2093E5d72D804a24bd5689",
                'block_number': block_number
            }
            
            try:
                result = trading_sim.analyze_token_trading_viability(config)
                
                status = "✅ Yes" if result['is_tradeable'] else "❌ No"
                buy_tax = f"{result['buy_tax_percent']:.2f}%" if result['is_tradeable'] else "-"
                sell_tax = f"{result['sell_tax_percent']:.2f}%" if result['is_tradeable'] else "-"
                tokens = str(result['tokens_received']) if result['is_tradeable'] else "-"
                eth_received = f"{result['eth_received'] / 1e18:.6f} ETH" if result['is_tradeable'] else "-"
                
                print(f"{block_number:<10} {status:<12} {buy_tax:<10} {sell_tax:<10} {tokens:<20} {eth_received:<20}")
                
                # Log failure details for non-tradeable tokens
                if not result['is_tradeable']:
                    if 'failure_reason' in result:
                        print(f"           🔍 Failure reason: {result['failure_reason']}")
                    
                    # Log individual transaction statuses for debugging
                    if 'transactions' in result:
                        txs = result['transactions']
                        buy_status = "✅" if txs['buy']['success'] else "❌"
                        approve_status = "✅" if txs['approve']['success'] else "❌"
                        sell_status = "✅" if txs['sell']['success'] else "❌"
                        
                        print(f"           📊 Transaction status: Buy {buy_status} | Approve {approve_status} | Sell {sell_status}")
                
                # Track state changes
                if result['is_tradeable']:
                    if trading_enabled_block is None:
                        trading_enabled_block = block_number
                    
                    # Check for tax changes
                    if last_buy_tax is not None:
                        if abs(result['buy_tax_percent'] - last_buy_tax) > 0.01:
                            tax_changes.append((block_number, "buy", last_buy_tax, result['buy_tax_percent']))
                    if last_sell_tax is not None:
                        if abs(result['sell_tax_percent'] - last_sell_tax) > 0.01:
                            tax_changes.append((block_number, "sell", last_sell_tax, result['sell_tax_percent']))
                    
                    last_buy_tax = result['buy_tax_percent']
                    last_sell_tax = result['sell_tax_percent']
                elif trading_enabled_block is not None and trading_disabled_block is None:
                    trading_disabled_block = block_number
                
                results.append((block_number, result))
                
            except Exception as e:
                print(f"{block_number:<10} ❌ Error: {str(e)}")
    
    except Exception as e:
        print(f"Analysis setup failed: {e}")
        import traceback
        traceback.print_exc()
        return
    
    # Summary
    print("\n📊 Analysis Summary:")
    print("===================")
    
    tradeable_count = sum(1 for _, r in results if r['is_tradeable'])
    total_count = len(results)
    
    print(f"  Total blocks analyzed: {total_count}")
    if total_count > 0:
        print(f"  Tradeable blocks: {tradeable_count} ({tradeable_count / total_count * 100:.1f}%)")
    
    if trading_enabled_block:
        print(f"  Trading first detected at block: {trading_enabled_block}")
    
    if trading_disabled_block:
        print(f"  Trading disabled at block: {trading_disabled_block}")
    
    if tax_changes:
        print("\n📈 Tax Changes Detected:")
        for block, tax_type, old_tax, new_tax in tax_changes:
            print(f"  Block {block}: {tax_type} tax changed from {old_tax:.2f}% to {new_tax:.2f}%")
    
    # Find best trading block (lowest combined tax)
    if tradeable_count > 0:
        tradeable_results = [(block, result) for block, result in results if result['is_tradeable']]
        if tradeable_results:
            best_block, best_result = min(tradeable_results, 
                key=lambda x: x[1]['buy_tax_percent'] + x[1]['sell_tax_percent'])
            
            print(f"\n🏆 Best Trading Block: {best_block}")
            print(f"  Combined tax: {best_result['buy_tax_percent'] + best_result['sell_tax_percent']:.2f}%")
            print(f"  Buy tax: {best_result['buy_tax_percent']:.2f}%, Sell tax: {best_result['sell_tax_percent']:.2f}%")
    
    # Compare with Rust results
    print("\n📊 Expected Results (from Rust):")
    print("  Trading pattern: Fails until block 23196239, then succeeds")
    print("  Failure reason: 'Sell transaction failed - token may prevent selling or have cooldown period'")
    print("  Key insight: 40-block sell restriction after liquidity addition")
    print("  Tax pattern: 0% throughout when tradeable")


def main():
    analyze_block_range()


if __name__ == "__main__":
    main()