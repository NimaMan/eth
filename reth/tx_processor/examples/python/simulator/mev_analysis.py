#!/usr/bin/env python3
"""
MEV Analysis Example using ethtx.Simulator

This example demonstrates how to analyze MEV (Maximum Extractable Value)
opportunities using transaction simulation.

Features:
- Sandwich attack simulation
- Arbitrage opportunity detection
- Profit calculation with gas costs

This example was migrated from reth_tx_simulator/python/examples/mev_analysis.py
"""

import ethtx

def main():
    # Initialize simulator
    try:
        sim = ethtx.Simulator()
        print(f"🔍 MEV Analysis Tool (using ethtx)")
        print(f"📊 Latest block: {sim.get_latest_block()}")
    except Exception as e:
        print(f"❌ Failed to initialize: {e}")
        return
    
    # Example: Sandwich Attack Analysis
    print("\n" + "="*60)
    print("🥪 Example: Sandwich Attack Simulation")
    print("="*60)
    
    # Target transaction (victim's large swap)
    victim_tx = {
        "from": "0x742d35Cc6134C0532925a3b8C17ebb6F5E9DFcf4",
        "to": "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D",  # Uniswap Router
        "value": "10000000000000000000",  # 10 ETH
        "data": "0x7ff36ab5" +  # swapExactETHForTokens
               "0000000000000000000000000000000000000000000000000000000000000000" +  # amountOutMin
               "0000000000000000000000000000000000000000000000000000000000000080" +  # path offset
               "000000000000000000000000742d35cc6134c0532925a3b8c17ebb6f5e9dfcf4" +  # to
               "0000000000000000000000000000000000000000000000000000000063f4e1c0" +  # deadline
               "0000000000000000000000000000000000000000000000000000000000000002" +  # path length
               "000000000000000000000000c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2" +  # WETH
               "000000000000000000000000a0b86991c6218b36c1d19d4a2e9eb0ce3606eb48",   # USDC
        "gas": 200000
    }
    
    # MEV Bot transactions
    frontrun_tx = {
        "from": "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045",  # MEV bot
        "to": "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D",  # Uniswap Router
        "value": "5000000000000000000",  # 5 ETH
        "data": victim_tx["data"],  # Same swap, front-running
        "gas": 200000
    }
    
    backrun_tx = {
        "from": "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045",  # MEV bot
        "to": "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D",  # Uniswap Router
        "data": "0x18cbafe5" +  # swapExactTokensForETH (sell tokens back)
               "0000000000000000000000000000000000000000000000000000000000000000" +  # amountIn (would be calculated)
               "0000000000000000000000000000000000000000000000000000000000000000" +  # amountOutMin
               "0000000000000000000000000000000000000000000000000000000000000080" +  # path offset
               "000000000000000000000000d8da6bf26964af9d7eed9e03e53415d37aa96045" +  # to
               "0000000000000000000000000000000000000000000000000000000063f4e1c0" +  # deadline
               "0000000000000000000000000000000000000000000000000000000000000002" +  # path length
               "000000000000000000000000a0b86991c6218b36c1d19d4a2e9eb0ce3606eb48" +  # USDC
               "000000000000000000000000c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2",   # WETH
        "gas": 200000
    }
    
    # Simulate sandwich attack sequence
    sandwich_sequence = [frontrun_tx, victim_tx, backrun_tx]
    
    try:
        print("📋 Simulating sandwich attack sequence:")
        print("  1. MEV bot front-runs with 5 ETH buy")
        print("  2. Victim swaps 10 ETH (gets worse price)")
        print("  3. MEV bot back-runs selling tokens")
        
        result = sim.simulate_sequence(sandwich_sequence)
        
        print(f"\n📊 Simulation Results:")
        print(f"  Total transactions: {result.total_transactions}")
        print(f"  Successful: {result.successful_transactions}")
        print(f"  Failed: {result.failed_transactions}")
        print(f"  Total gas: {result.total_gas_used}")
        
        # Analyze each transaction
        for i, tx_result in enumerate(result.results):
            tx_type = ["Front-run", "Victim", "Back-run"][i]
            status = "✅" if tx_result.success else "❌"
            print(f"  {tx_type}: {status} (gas: {tx_result.gas_used})")
            if not tx_result.success:
                print(f"    Revert: {tx_result.revert_reason}")
        
        # Calculate MEV profit (simplified)
        if result.sequence_success:
            total_gas_cost = result.total_gas_used * 20  # 20 gwei gas price
            print(f"\n💰 MEV Analysis:")
            print(f"  Gas cost: {total_gas_cost / 1e9:.6f} ETH")
            print(f"  Sandwich attack: {'✅ Profitable' if result.successful_transactions == 3 else '❌ Not profitable'}")
            
    except Exception as e:
        print(f"❌ Sandwich simulation failed: {e}")
    
    # Example 2: Arbitrage Opportunity Detection
    print("\n" + "="*60)
    print("💹 Example: Arbitrage Opportunity Detection")
    print("="*60)
    
    # Check price difference between DEXes
    # Buy on Uniswap, sell on SushiSwap
    arbitrage_sequence = [
        # Buy USDC with ETH on Uniswap
        {
            "from": "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045",
            "to": "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D",  # Uniswap
            "value": "1000000000000000000",  # 1 ETH
            "data": "0x7ff36ab5...",  # swapExactETHForTokens (abbreviated)
            "gas": 200000
        },
        # Sell USDC for ETH on SushiSwap
        {
            "from": "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045",
            "to": "0xd9e1cE17f2641f24aE83637ab66a2cca9C378B9F",  # SushiSwap
            "data": "0x18cbafe5...",  # swapExactTokensForETH (abbreviated)
            "gas": 200000
        }
    ]
    
    try:
        print("🔍 Checking arbitrage opportunity...")
        print("  1. Buy USDC on Uniswap with 1 ETH")
        print("  2. Sell USDC on SushiSwap for ETH")
        
        arb_result = sim.simulate_sequence(arbitrage_sequence)
        
        if arb_result.sequence_success:
            print(f"\n✅ Arbitrage simulation successful")
            print(f"  Gas used: {arb_result.total_gas_used}")
            # In practice, you'd calculate actual profit from state changes
            print("  Profit calculation would analyze state changes")
        else:
            print(f"\n❌ Arbitrage not profitable at current prices")
            
    except Exception as e:
        print(f"❌ Arbitrage simulation failed: {e}")
    
    # Example 3: Flash Loan Arbitrage
    print("\n" + "="*60)
    print("⚡ Example: Flash Loan Arbitrage Analysis")
    print("="*60)
    
    print("📝 Flash loan arbitrage steps:")
    print("  1. Borrow WETH from Aave")
    print("  2. Swap WETH → USDC on DEX 1")
    print("  3. Swap USDC → WETH on DEX 2")
    print("  4. Repay flash loan + fee")
    print("  5. Keep profit")
    
    print("\n💡 Use ethtx.TxBuilder to construct complex DeFi transactions")
    print("   builder = ethtx.TxBuilder.mainnet()")
    print("   flash_loan_tx = builder.aave_flash_loan(...)")
    
    print("\n🎉 MEV Analysis examples completed!")
    print("   All functionality now available through unified ethtx module")

if __name__ == "__main__":
    try:
        main()
    except ImportError:
        print("❌ ethtx module not built. Please run:")
        print("   cd /home/nima/code/crypto/rust/tx_processor")
        print("   maturin develop --release --features python")