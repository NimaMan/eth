#!/usr/bin/env python3
"""
USDC V2 Trading Analysis

Python equivalent of the Rust example that tests USDC trading on Uniswap V2.
This validates that our Python bindings produce the same results as the Rust implementation.

Tests the buy/approve/sell sequence for USDC on Uniswap V2 pool.
"""
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
    print("  Test Amount: 0.01 ETH")
    print()
    
    print("Analyzing V2 pool...")
    
    try:
        # Create PyReth instance and get trading simulator
        py_reth = pyreth.PyReth()
        trading_sim = py_reth.trading_simulator()
        
        # Use default 0.01 ETH configuration (same default as Rust)
        config = trading_sim.default_config()
        
        # Run the buy/approve/sell analysis using default configuration
        # Use specific block number for consistent comparison with Rust
        target_block = 23205942  # Same as Rust example
        
        result = trading_sim.simulate_with_config(
            prior_tx=None,  # No prior transaction
            token_address=usdc_address,
            pool_address=v2_pool_address,
            config=config,
            block_number=target_block
        )
        
        # Always show transaction status first
        print("\nTransaction Status:")
        buy_success = result.buy_tx.status == "1"
        approve_success = result.approve_tx.status == "1"  
        sell_success = result.sell_tx.status == "1"
        
        # Handle fees as dict
        buy_gas = result.buy_tx.fees.get('gas_used', 0) if isinstance(result.buy_tx.fees, dict) else result.buy_tx.fees.gas_used
        approve_gas = result.approve_tx.fees.get('gas_used', 0) if isinstance(result.approve_tx.fees, dict) else result.approve_tx.fees.gas_used
        sell_gas = result.sell_tx.fees.get('gas_used', 0) if isinstance(result.sell_tx.fees, dict) else result.sell_tx.fees.gas_used
        
        print(f"  Buy: {'✅ Success' if buy_success else '❌ Failed'} (gas: {buy_gas})")
        print(f"  Approve: {'✅ Success' if approve_success else '❌ Failed'} (gas: {approve_gas})")
        print(f"  Sell: {'✅ Success' if sell_success else '❌ Failed'} (gas: {sell_gas})")
        
        # Show actual transaction flow
        print(f"\nTransaction Flow Analysis:")
        print(f"=========================")
        print(f"Block Number: {result.block_number}")
        
        # Analyze buy transaction
        print(f"\n1. BUY Transaction:")
        buy_tx = result.buy_tx
        print(f"   Status: {buy_tx.status}")
        print(f"   Hash: {buy_tx.hash}")
        print(f"   From: {buy_tx.from_address}")
        print(f"   ETH Value: {buy_tx.value} wei")
        
        # Show ERC20 transfers from buy
        tokens_received = 0
        print(f"   ERC20 Transfers: {len(buy_tx.erc20_transfers)}")
        for i, transfer in enumerate(buy_tx.erc20_transfers):
            # Handle transfer as dict
            amount = transfer.get('amount', 0) if isinstance(transfer, dict) else transfer.amount
            from_addr = transfer.get('from_address', 'Unknown') if isinstance(transfer, dict) else transfer.from_address
            to_addr = transfer.get('to_address', 'Unknown') if isinstance(transfer, dict) else transfer.to_address
            token_addr = transfer.get('token_address', 'Unknown') if isinstance(transfer, dict) else transfer.token_address
            
            print(f"     {i+1}. {amount} tokens from {from_addr} to {to_addr}")
            print(f"        Token: {token_addr}")
            if token_addr == usdc_address:
                tokens_received = amount
        
        print(f"   USDC Tokens Received: {tokens_received}")
        
        # Analyze approve transaction  
        print(f"\n2. APPROVE Transaction:")
        approve_tx = result.approve_tx
        print(f"   Status: {approve_tx.status}")
        print(f"   Hash: {approve_tx.hash}")
        print(f"   Approvals: {len(approve_tx.approvals)}")
        for approval in approve_tx.approvals:
            # Handle approval as dict
            amount = approval.get('amount', 0) if isinstance(approval, dict) else approval.amount
            spender = approval.get('spender', 'Unknown') if isinstance(approval, dict) else approval.spender
            print(f"     Approved {amount} tokens for {spender}")
        
        # Analyze sell transaction
        print(f"\n3. SELL Transaction:")
        sell_tx = result.sell_tx  
        print(f"   Status: {sell_tx.status}")
        print(f"   Hash: {sell_tx.hash}")
        print(f"   From: {sell_tx.from_address}")
        print(f"   ETH Value: {sell_tx.value} wei")
        
        # Show ERC20 transfers from sell
        print(f"   ERC20 Transfers: {len(sell_tx.erc20_transfers)}")
        for i, transfer in enumerate(sell_tx.erc20_transfers):
            # Handle transfer as dict
            amount = transfer.get('amount', 0) if isinstance(transfer, dict) else transfer.amount
            from_addr = transfer.get('from_address', 'Unknown') if isinstance(transfer, dict) else transfer.from_address
            to_addr = transfer.get('to_address', 'Unknown') if isinstance(transfer, dict) else transfer.to_address
            token_addr = transfer.get('token_address', 'Unknown') if isinstance(transfer, dict) else transfer.token_address
            
            print(f"     {i+1}. {amount} tokens from {from_addr} to {to_addr}")
            print(f"        Token: {token_addr}")
            
        # Get WETH received back from sell (WETH is an ERC20 token in sell transaction)
        eth_received = 0
        weth_address = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"
        router_address = "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D"
        
        # Look for WETH transfer to router (which then converts to ETH for buyer)
        for transfer in sell_tx.erc20_transfers:
            token_addr = transfer.get('token_address', '') if isinstance(transfer, dict) else transfer.token_address
            to_addr = transfer.get('to_address', '') if isinstance(transfer, dict) else transfer.to_address
            amount = transfer.get('amount', 0) if isinstance(transfer, dict) else transfer.amount
            
            # Check if it's WETH being sent to router (router then unwraps to ETH for buyer)
            if token_addr == weth_address and to_addr == router_address:
                eth_received = amount
                break
        
        print(f"   WETH Received (to router for unwrap): {eth_received} wei")
        
        # Final analysis
        print(f"\nFinal Analysis:")
        print(f"==============")
        print(f"Trading Enabled: {result.trading_enabled}")
        print(f"Buy Tax: {result.buy_tax:.2f}%")
        print(f"Sell Tax: {result.sell_tax:.2f}%")
        
        if int(tokens_received) > 0 and int(eth_received) > 0:
            eth_spent = int(0.01 * 1e18)  # Fixed to 0.01 ETH
            eth_loss_pct = (1 - int(eth_received) / eth_spent) * 100
            print(f"Net ETH Loss: {eth_loss_pct:.2f}% (DEX fees + slippage)")
            print(f"✅ Complete buy/sell cycle executed successfully!")
        elif not sell_success:
            print(f"❌ Sell transaction failed - investigating cause...")
        else:
            print(f"❌ Incomplete transaction data")
        
    except Exception as e:
        print(f"Analysis failed: {e}")
        import traceback
        traceback.print_exc()


if __name__ == "__main__":
    main()