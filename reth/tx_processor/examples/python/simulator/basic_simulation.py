#!/usr/bin/env python3
"""
Basic usage example for the Simulator functionality within ethtx

This example demonstrates:
1. Single transaction simulation
2. Sequential simulation (approve + swap)
3. State change analysis
4. Error handling

This example was migrated from reth_tx_simulator/python/examples/basic_usage.py
"""

import ethtx

def main():
    # Initialize simulator
    try:
        sim = ethtx.Simulator()
        print(f"✅ Initialized simulator")
        print(f"📊 Latest block: {sim.get_latest_block()}")
    except Exception as e:
        print(f"❌ Failed to initialize simulator: {e}")
        print("💡 Make sure Reth database is accessible")
        return
    
    # Example 1: Single transaction simulation
    print("\n" + "="*50)
    print("📤 Example 1: Single Transaction Simulation")
    print("="*50)
    
    # Simple ETH transfer
    eth_transfer = {
        "from": "0x742d35Cc6134C0532925a3b8C17ebb6F5E9DFcf4",
        "to": "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045", 
        "value": "1000000000000000000",  # 1 ETH
        "gas": 21000
    }
    
    try:
        result = sim.simulate_transaction(eth_transfer)
        print(f"✅ Success: {result.success}")
        print(f"⛽ Gas used: {result.gas_used}")
        if not result.success:
            print(f"💥 Revert reason: {result.revert_reason}")
    except Exception as e:
        print(f"❌ Simulation failed: {e}")
    
    # Example 2: Sequential simulation (approve + swap)
    print("\n" + "="*50) 
    print("📤 Example 2: Sequential Simulation")
    print("="*50)
    
    # USDC approve + Uniswap swap sequence
    transactions = [
        # 1. Approve USDC to Uniswap Router
        {
            "from": "0x742d35Cc6134C0532925a3b8C17ebb6F5E9DFcf4",
            "to": "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",  # USDC
            "data": "0x095ea7b3000000000000000000000000" +  # approve(address,uint256)
                   "7a250d5630b4cf539739df2c5dacb4c659f2488d" +  # Uniswap Router
                   "00000000000000000000000000000000000000000000000000000000000f4240",  # 1M USDC (6 decimals)
            "gas": 50000
        },
        # 2. Swap USDC for ETH
        {
            "from": "0x742d35Cc6134C0532925a3b8C17ebb6F5E9DFcf4", 
            "to": "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D",  # Uniswap Router
            "data": "0x18cbafe5" +  # swapExactTokensForETH (abbreviated for demo)
                   "00000000000000000000000000000000000000000000000000000000000f4240",  # 1M USDC
            "gas": 200000
        }
    ]
    
    try:
        seq_result = sim.simulate_sequence(transactions)
        
        print(f"📊 Total transactions: {seq_result.total_transactions}")
        print(f"✅ Successful: {seq_result.successful_transactions}")
        print(f"❌ Failed: {seq_result.failed_transactions}")
        print(f"⛽ Total gas used: {seq_result.total_gas_used}")
        print(f"🎯 Sequence success: {seq_result.sequence_success}")
        
        for i, tx_result in enumerate(seq_result.results):
            print(f"  TX {i}: {'✅' if tx_result.success else '❌'} "
                  f"(gas: {tx_result.gas_used})")
            if not tx_result.success:
                print(f"    💥 Revert: {tx_result.revert_reason}")
                
    except Exception as e:
        print(f"❌ Sequential simulation failed: {e}")
    
    # Example 3: State change analysis
    print("\n" + "="*50)
    print("📤 Example 3: State Change Analysis") 
    print("="*50)
    
    try:
        # Build transaction with simulator
        tx = sim.build_transaction(
            from_address="0x742d35Cc6134C0532925a3b8C17ebb6F5E9DFcf4",
            to_address="0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045",
            value="1000000000000000000",  # 1 ETH
            gas_limit=21000
        )
        
        # Simulate with detailed state tracking
        result = sim.simulate_transaction(tx)
        
        print(f"✅ Success: {result.success}")
        
        if hasattr(result, 'state_changes') and result.state_changes:
            print(f"📊 State changes for {len(result.state_changes)} addresses:")
            
            for address, changes in result.state_changes.items():
                print(f"  {address}:")
                if hasattr(changes, 'eth_net') and changes.eth_net != 0:
                    print(f"    💰 ETH: {changes.eth_net:+.6f}")
                if hasattr(changes, 'token_net'):
                    for token, amount in changes.token_net.items():
                        print(f"    🪙 {token}: {amount:+.6f}")
                
    except Exception as e:
        print(f"❌ State change analysis failed: {e}")
    
    # Example 4: Historical simulation
    print("\n" + "="*50)
    print("📤 Example 4: Historical Block Simulation")
    print("="*50)
    
    try:
        # Simulate at block 18500000
        historical_result = sim.simulate_at_block(eth_transfer, block_number=18500000)
        print(f"✅ Historical simulation (block 18500000): {historical_result.success}")
        print(f"⛽ Gas used: {historical_result.gas_used}")
    except NotImplementedError:
        print("⚠️  Historical simulation feature in progress")
    except Exception as e:
        print(f"❌ Historical simulation failed: {e}")
    
    print("\n🎉 Simulator examples completed!")
    print("   All simulator functionality is now part of ethtx module")

if __name__ == "__main__":
    try:
        main()
    except ImportError:
        print("❌ ethtx module not built. Please run:")
        print("   cd /home/nima/code/crypto/rust/tx_processor")
        print("   maturin develop --release --features python")