#!/usr/bin/env python3
"""
Block Range Trading Analysis

Python equivalent of the Rust example that analyzes token trading across multiple blocks.
This helps detect when trading was enabled/disabled or tax changes occurred.

Analyzes the moo token across blocks around the liquidity addition to understand
the trading restriction period.
"""
import pyreth


def analyze_block_range():
    print("Block Range Token Trading Analysis (Python)")
    print("===========================================")
    
    # Create PyReth instance and get components
    py_reth = pyreth.PyReth()
    trading_sim = py_reth.trading_simulator()
    
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
    
    config = trading_sim.default_config()
    config = config.with_buy_amount(0.01)
    
    for block_number in range(start_block, end_block + 1, block_step):
        try:
            # Run simulation at specific block
            result = trading_sim.simulate_with_config(
                prior_tx=None,
                token_address=token_address,
                pool_address=pool_address,
                config=config,
                block_number=block_number
            )
            
            # Determine if trading is enabled
            is_tradeable = result.trading_enabled
            
            if is_tradeable:
                status = "✅ Yes"
                buy_tax = f"{result.buy_tax:.2f}%"
                sell_tax = f"{result.sell_tax:.2f}%"
                
                # Extract tokens received from buy transaction
                tokens_received = 0
                for transfer in result.buy_tx.erc20_transfers:
                    if transfer['token_address'] == token_address:
                        if transfer['to_address'] == result.buy_tx.from_address:
                            tokens_received = int(transfer['amount'])
                            break
                
                # Extract ETH received from sell transaction (as WETH)
                eth_received = 0
                weth_address = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"
                for transfer in result.sell_tx.erc20_transfers:
                    if transfer['token_address'] == weth_address:
                        eth_received = int(transfer['amount'])
                        break
                
                tokens_str = str(tokens_received) if tokens_received > 0 else "-"
                eth_str = f"{eth_received / 1e18:.6f} ETH" if eth_received > 0 else "-"
            else:
                status = "❌ No"
                buy_tax = "-"
                sell_tax = "-"
                tokens_str = "-"
                eth_str = "-"
            
            print(f"{block_number:<10} {status:<12} {buy_tax:<10} {sell_tax:<10} {tokens_str:<20} {eth_str:<20}")
            
            # Log failure details for non-tradeable tokens
            if not is_tradeable:
                # Check which transaction failed
                buy_success = result.buy_tx.status == '1'
                approve_success = result.approve_tx.status == '1'
                sell_success = result.sell_tx.status == '1'
                
                buy_status = "✅" if buy_success else "❌"
                approve_status = "✅" if approve_success else "❌"
                sell_status = "✅" if sell_success else "❌"
                print(f"           📊 Transaction status: Buy {buy_status} | Approve {approve_status} | Sell {sell_status}")
                
                # The TradingSequenceResult doesn't have failure_reason - that's in other result types
                # We'll extract failure details from the individual transactions
                
                # Also show transaction-specific details
                if not buy_success:
                    # Show buy transaction failure details
                    if result.buy_tx.txn_type:
                        print(f"           ⚠️  Buy transaction type: {result.buy_tx.txn_type}")
                    if result.buy_tx.actions:
                        # Actions contain the failure reason
                        for action in result.buy_tx.actions:
                            if action:
                                print(f"           ⚠️  Buy details: {action}")
                
                if not sell_success:
                    # Show sell transaction failure details
                    if result.sell_tx.txn_type:
                        print(f"           ⚠️  Sell transaction type: {result.sell_tx.txn_type}")
                    if result.sell_tx.actions:
                        # Actions contain the failure reason
                        for action in result.sell_tx.actions:
                            if action:
                                print(f"           ⚠️  Sell details: {action}")
            
            # Track state changes
            if is_tradeable:
                if trading_enabled_block is None:
                    trading_enabled_block = block_number
                
                # Check for tax changes
                if last_buy_tax is not None:
                    if abs(result.buy_tax - last_buy_tax) > 0.01:
                        tax_changes.append((block_number, "buy", last_buy_tax, result.buy_tax))
                if last_sell_tax is not None:
                    if abs(result.sell_tax - last_sell_tax) > 0.01:
                        tax_changes.append((block_number, "sell", last_sell_tax, result.sell_tax))
                
                last_buy_tax = result.buy_tax
                last_sell_tax = result.sell_tax
            elif trading_enabled_block is not None and trading_disabled_block is None:
                trading_disabled_block = block_number
            
            results.append((block_number, {
                'is_tradeable': is_tradeable,
                'buy_tax': result.buy_tax if is_tradeable else 0,
                'sell_tax': result.sell_tax if is_tradeable else 0,
                'tokens_received': tokens_received if is_tradeable else 0
            }))
            
        except Exception as e:
            # Show the actual error from the simulation
            error_msg = str(e)
            # Truncate very long error messages
            if len(error_msg) > 100:
                error_msg = error_msg[:100] + "..."
            print(f"{block_number:<10} ❌ Error: {error_msg}")
            results.append((block_number, {
                'is_tradeable': False,
                'buy_tax': 0,
                'sell_tax': 0,
                'tokens_received': 0
            }))
    
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
                key=lambda x: x[1]['buy_tax'] + x[1]['sell_tax'])
            
            print(f"\n🏆 Best Trading Block: {best_block}")
            print(f"  Combined tax: {best_result['buy_tax'] + best_result['sell_tax']:.2f}%")
            print(f"  Buy tax: {best_result['buy_tax']:.2f}%, Sell tax: {best_result['sell_tax']:.2f}%")
    
    # Compare with Rust results
    print("\n📊 Expected Results (from Rust):")
    print("  Trading pattern: Fails until block 23196239, then succeeds")
    print("  Failure reason: Shows actual EVM revert reasons")
    print("  Key insight: 40-block sell restriction after liquidity addition")
    print("  Tax pattern: 0% throughout when tradeable")


def main():
    analyze_block_range()


if __name__ == "__main__":
    main()