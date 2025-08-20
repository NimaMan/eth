#!/usr/bin/env python3
"""
Complete demonstration of the pyreth module

This example shows all functionality:
1. TxProcessor - Process historical transactions
2. Simulator - Simulate unsigned transactions
3. TxBuilder - Build user-friendly transactions

This consolidates examples from:
- reth_tx_simulator/python/examples/
- tx_builder/examples/
- tx_processor/examples/python/
"""

try:
    import pyreth
except ImportError:
    print("❌ pyreth module not built. Please run:")
    print("   cd /home/nima/code/crypto/rust/tx_processor")
    print("   maturin develop --release --features python")
    print("\nNote: Module was renamed from rs_tx_processor to ethtx")
    exit(1)

def demo_tx_processor():
    """Demonstrate transaction processing from Reth database"""
    print("\n" + "="*60)
    print("📊 1. TxProcessor - Historical Transaction Analysis")
    print("="*60)
    
    try:
        processor = pyreth.TxProcessor()
        print(f"✅ TxProcessor initialized")
        print(f"   Database: /home/nima/.local/share/reth/mainnet")
        
        # Process a transaction (example hash)
        test_tx = "0x5c89f223fe19593cf85319dcdd3f6de618e770d958f72ea123d36db2fdf5d46e"
        
        try:
            result = processor.process_transaction(test_tx)
            print(f"✅ Transaction processed:")
            print(f"   Type: {result.txn_type}")
            print(f"   From: {result.from_address[:10]}...")
            print(f"   To: {result.to_address[:10] if result.to_address else 'Contract Creation'}")
            print(f"   Value: {result.value} wei")
            print(f"   ERC20 transfers: {len(result.erc20_transfers)}")
            print(f"   Internal txs: {len(result.internal_transactions)}")
            
            # Show ERC20 transfers
            for transfer in result.erc20_transfers[:3]:
                print(f"   📤 {transfer.token_symbol or 'Unknown'}: "
                      f"{transfer.from_address[:10]}... → {transfer.to_address[:10]}...")
                      
        except NotImplementedError as e:
            print(f"⚠️  TxProcessor placeholder (real implementation in progress)")
            print(f"   {e}")
            
    except Exception as e:
        print(f"❌ TxProcessor error: {e}")

def demo_simulator():
    """Demonstrate transaction simulation"""
    print("\n" + "="*60)
    print("🔬 2. Simulator - Transaction Simulation & State Changes")
    print("="*60)
    
    try:
        simulator = pyreth.Simulator()
        print(f"✅ Simulator initialized")
        print(f"📊 Latest block: {simulator.get_latest_block()}")
        
        # Example 1: Simple ETH transfer
        print("\n📤 Simulating ETH transfer:")
        eth_tx = simulator.build_transaction(
            from_address="0x742d35Cc6134C0532925a3b8C17ebb6F5E9DFcf4",
            to_address="0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045",
            value="1000000000000000000",  # 1 ETH
            gas_limit=21000
        )
        
        result = simulator.simulate_transaction(eth_tx)
        print(f"   Success: {result.success}")
        print(f"   Gas used: {result.gas_used}")
        if result.revert_reason:
            print(f"   Revert: {result.revert_reason}")
            
        # Show state changes
        if result.state_changes:
            print(f"   State changes for {len(result.state_changes)} addresses:")
            for addr, changes in list(result.state_changes.items())[:2]:
                print(f"     {addr[:10]}...")
                if changes.eth_net != 0:
                    print(f"       ETH: {changes.eth_net:+.6f}")
                for token, amount in changes.token_net.items():
                    print(f"       {token}: {amount:+.6f}")
        
        # Example 2: Sequential simulation (approve + swap)
        print("\n📤 Simulating sequential transactions (approve + swap):")
        
        # Build approve transaction
        approve_tx = {
            "from": "0x742d35Cc6134C0532925a3b8C17ebb6F5E9DFcf4",
            "to": "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",  # USDC
            "data": "0x095ea7b3" +  # approve(address,uint256)
                   "0000000000000000000000007a250d5630b4cf539739df2c5dacb4c659f2488d" +  # Uniswap
                   "00000000000000000000000000000000000000000000000000000000000f4240",  # 1M USDC
            "gas": 50000
        }
        
        # Build swap transaction
        swap_tx = {
            "from": "0x742d35Cc6134C0532925a3b8C17ebb6F5E9DFcf4",
            "to": "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D",  # Uniswap Router
            "data": "0x18cbafe5...",  # swapExactTokensForETH (truncated for demo)
            "gas": 200000
        }
        
        seq_result = simulator.simulate_sequence([approve_tx, swap_tx])
        print(f"   Total transactions: {seq_result.total_transactions}")
        print(f"   Successful: {seq_result.successful_transactions}")
        print(f"   Failed: {seq_result.failed_transactions}")
        print(f"   Total gas: {seq_result.total_gas_used}")
        print(f"   Sequence success: {seq_result.sequence_success}")
        
    except Exception as e:
        print(f"❌ Simulator error: {e}")

def demo_tx_builder():
    """Demonstrate user-friendly transaction building"""
    print("\n" + "="*60)
    print("🔨 3. TxBuilder - User-Friendly Transaction Construction")
    print("="*60)
    
    try:
        # Initialize builder for mainnet
        builder = pyreth.TxBuilder.mainnet()
        print(f"✅ TxBuilder initialized for Ethereum mainnet")
        
        # Example addresses
        from_addr = "0x742d35Cc6134C0532925a3b8C17ebb6F5E9DFcf4"
        to_addr = "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045"
        
        # Example 1: ETH Transfer
        print("\n💰 Building ETH transfer:")
        eth_tx = builder.eth_transfer(from_addr, to_addr, "1.5")  # 1.5 ETH
        print(f"   From: {eth_tx['from'][:10]}...")
        print(f"   To: {eth_tx['to'][:10]}...")
        print(f"   Value: {eth_tx['value']} (1.5 ETH in wei)")
        print(f"   Gas: {eth_tx.get('gas', 'default')}")
        print(f"   Data: {eth_tx['data']} (empty for ETH transfer)")
        
        # Example 2: ERC20 Transfer using symbol
        print("\n🪙 Building ERC20 transfer (USDC):")
        usdc_tx = builder.erc20_transfer("USDC", from_addr, to_addr, "1000.0")
        print(f"   Token: USDC (resolved to contract)")
        print(f"   To: {usdc_tx['to']} (USDC contract)")
        print(f"   Data: {usdc_tx['data'][:10]}... (encoded transfer)")
        print(f"   Amount: 1000.0 USDC (auto-converted to 6 decimals)")
        
        # Example 3: ERC20 Approval for DeFi
        print("\n✅ Building ERC20 approval:")
        approve_tx = builder.erc20_approve(
            "USDC", 
            from_addr,
            "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D",  # Uniswap Router
            "unlimited"
        )
        print(f"   Token: USDC")
        print(f"   Spender: Uniswap Router V2")
        print(f"   Amount: Unlimited (max uint256)")
        print(f"   Data: {approve_tx['data'][:10]}... (encoded approve)")
        
        # Example 4: Token Information
        print("\n📊 Token information:")
        usdc_info = builder.get_token_info("USDC")
        print(f"   USDC:")
        print(f"     Address: {usdc_info['address']}")
        print(f"     Decimals: {usdc_info['decimals']}")
        
        weth_info = builder.get_token_info("WETH")
        print(f"   WETH:")
        print(f"     Address: {weth_info['address']}")
        print(f"     Decimals: {weth_info['decimals']}")
        
        # Example 5: List available tokens
        print("\n📋 Available tokens:")
        tokens = builder.list_tokens()
        print(f"   Total: {len(tokens)} tokens")
        print(f"   Sample: {', '.join(tokens[:10])}...")
        
        # Example 6: Gas estimation
        print("\n⛽ Gas estimates:")
        eth_gas = builder.estimate_gas("eth_transfer")
        erc20_gas = builder.estimate_gas("erc20_transfer")
        approve_gas = builder.estimate_gas("erc20_approve")
        print(f"   ETH transfer: {eth_gas} gas")
        print(f"   ERC20 transfer: {erc20_gas} gas")
        print(f"   ERC20 approve: {approve_gas} gas")
        
    except Exception as e:
        print(f"❌ TxBuilder error: {e}")

def demo_complete_workflow():
    """Demonstrate complete workflow: Build → Simulate → Process"""
    print("\n" + "="*60)
    print("🔄 4. Complete Workflow: Build → Simulate → Process")
    print("="*60)
    
    try:
        # Step 1: Build transaction
        print("\n📝 Step 1: Build transaction")
        builder = pyreth.TxBuilder.mainnet()
        from_addr = "0x742d35Cc6134C0532925a3b8C17ebb6F5E9DFcf4"
        to_addr = "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045"
        
        tx_params = builder.erc20_transfer("USDC", from_addr, to_addr, "100.0")
        print(f"   Built USDC transfer: 100.0 USDC")
        print(f"   Contract: {tx_params['to']}")
        print(f"   Encoded data: {tx_params['data'][:10]}...")
        
        # Step 2: Simulate transaction
        print("\n🔬 Step 2: Simulate before sending")
        simulator = pyreth.Simulator()
        
        # Convert to simulator format
        sim_tx = {
            "from": tx_params["from"],
            "to": tx_params["to"],
            "data": tx_params["data"],
            "value": tx_params["value"],
            "gas": tx_params.get("gas", 100000)
        }
        
        result = simulator.simulate_transaction(sim_tx)
        print(f"   Simulation: {'✅ Success' if result.success else '❌ Failed'}")
        print(f"   Gas needed: {result.gas_used}")
        
        if result.success:
            print("\n✅ Transaction ready to send!")
            print("   Next: Use web3.py to send to network")
            # In real usage:
            # tx_hash = web3.eth.send_transaction(tx_params)
        else:
            print(f"\n❌ Transaction would fail: {result.revert_reason}")
            print("   Fix issues before sending")
        
        # Step 3: After mining, process transaction
        print("\n📊 Step 3: After mining, analyze transaction")
        print("   (Would use TxProcessor with actual tx_hash)")
        # processor = pyreth.TxProcessor()
        # processed = processor.process_transaction(tx_hash)
        
    except Exception as e:
        print(f"❌ Workflow error: {e}")

def main():
    print("\n" + "="*70)
    print("           🚀 pyreth - Complete Ethereum Transaction Toolkit")
    print("="*70)
    print("\nPreviously: rs_tx_processor (verbose)")
    print("Now: ethtx (clean, professional)")
    
    # Run all demonstrations
    demo_tx_processor()
    demo_simulator()
    demo_tx_builder()
    demo_complete_workflow()
    
    print("\n" + "="*70)
    print("                        🎉 Demo Complete!")
    print("="*70)
    print("\n📚 Summary:")
    print("   • TxProcessor: Analyze historical transactions from Reth DB")
    print("   • Simulator: Test transactions before sending")
    print("   • TxBuilder: Create transactions without ABI knowledge")
    print("   • All integrated in one clean 'ethtx' module")
    print("\n💡 Next steps:")
    print("   1. Use TxBuilder to create user-friendly transactions")
    print("   2. Test with Simulator before sending")
    print("   3. Analyze with TxProcessor after mining")

if __name__ == "__main__":
    main()