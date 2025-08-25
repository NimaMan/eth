#!/usr/bin/env python3
"""
moo Token Trading Analysis

Python equivalent of the Rust example that tests the moo token from a specific transaction.
Validates simulation against real transaction: 0xea83d46948ebd529ad26370a5f46341a6a84f3c2f1b07caf7cf94aa232283115

This example simulates buy/approve/sell at the same block where we know
the token was tradeable to verify our simulation matches reality.
"""

import pyreth


def main():
    print("moo Token Trading Viability Analysis (Python)")
    print("==============================================")
    print("Reference TX: 0xea83d46948ebd529ad26370a5f46341a6a84f3c2f1b07caf7cf94aa232283115")
    print("Block: 23196488")
    print("Original swap: 0.1 ETH -> 575,446,178 moo tokens")
    print()
    
    # Create PyReth instance and get trading simulator
    py_reth = pyreth.PyReth()
    trading_sim = py_reth.trading_simulator()
    
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
        # Use config with 0.1 ETH (not default 0.01 ETH)
        config = trading_sim.default_config()
        config = config.with_buy_amount(0.1)  # 0.1 ETH
        
        # Run the buy/approve/sell simulation
        result = trading_sim.simulate_with_config(
            prior_tx=None,  # No prior transaction
            token_address=moo_token_address,
            pool_address=moo_pool_address,
            config=config,
            block_number=target_block
        )
        
        print("\nAnalysis Results:")
        print("=================")
        print(f"Block Number: {result.block_number}")
        print(f"Trading Enabled: {result.trading_enabled}")
        
        if result.trading_enabled:
            print("\n✅ Pool is tradeable!")
            print(f"  Buy Tax: {result.buy_tax:.2f}%")
            print(f"  Sell Tax: {result.sell_tax:.2f}%")
            
            # Extract tokens received from buy transaction
            buy_tx = result.buy_tx
            tokens_received = 0
            for transfer in buy_tx.erc20_transfers:
                # Look for transfer to buyer
                if transfer['token_address'] == moo_token_address:
                    if transfer['to_address'] == buy_tx.from_address:
                        tokens_received = int(transfer['amount'])
                        break
            
            print(f"  Tokens Received: {tokens_received}")
            
            # Compare with original transaction
            original_tokens = 575_446_178_536_175_301
            
            if original_tokens > 0 and tokens_received > 0:
                difference_pct = abs(original_tokens - tokens_received) / original_tokens * 100
                
                print("\n📊 Comparison with original transaction:")
                print(f"  Original tokens received: {original_tokens:,}")
                print(f"  Simulated tokens received: {tokens_received:,}")
                print(f"  Difference: {difference_pct:.2f}%")
                
                if difference_pct < 5.0:
                    print("  ✅ Simulation closely matches actual transaction!")
                else:
                    print("  ⚠️ Some difference detected (could be due to MEV, slippage, or block timing)")
        else:
            print("\n❌ Pool is not tradeable")
        
        # Transaction details
        print("\nTransaction Status:")
        print(f"  Buy: {'Success' if result.buy_tx.status == '1' else 'Failed'} (gas: {result.buy_tx.fees['gas_used']})")
        print(f"  Approve: {'Success' if result.approve_tx.status == '1' else 'Failed'} (gas: {result.approve_tx.fees['gas_used']})")
        print(f"  Sell: {'Success' if result.sell_tx.status == '1' else 'Failed'} (gas: {result.sell_tx.fees['gas_used']})")
        
        # Expected results from Rust
        print("\n📊 Expected Results (from Rust):")
        print("  Tokens: ~497,409,164 moo tokens")
        print("  Buy Tax: 0.00%")
        print("  Sell Tax: 0.00%")
        print("  Note: Difference from original is due to different routing/execution")
        
    except Exception as e:
        print(f"\n❌ Analysis failed: {e}")
        import traceback
        traceback.print_exc()


if __name__ == "__main__":
    main()