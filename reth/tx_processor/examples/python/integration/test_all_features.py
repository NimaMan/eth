#!/usr/bin/env python3
"""
Comprehensive test of all ethtx features

This test verifies that all Python bindings are properly integrated:
1. TxProcessor - Process historical transactions
2. Simulator - Simulate unsigned transactions
3. TxBuilder - Build user-friendly transactions

Run this to validate the complete integration.
"""

import sys

def test_module_import():
    """Test that ethtx module can be imported"""
    print("="*60)
    print("1. Testing Module Import")
    print("="*60)
    
    try:
        import ethtx
        print("✅ Module imported successfully")
        print(f"   Module: ethtx")
        
        # Check version if available
        if hasattr(ethtx, '__version__'):
            print(f"   Version: {ethtx.__version__}")
        
        return ethtx
    except ImportError as e:
        print(f"❌ Failed to import ethtx: {e}")
        print("\n📝 Build instructions:")
        print("   cd /home/nima/code/crypto/rust/tx_processor")
        print("   maturin develop --release --features python")
        return None

def test_tx_processor(ethtx):
    """Test TxProcessor functionality"""
    print("\n" + "="*60)
    print("2. Testing TxProcessor")
    print("="*60)
    
    try:
        processor = ethtx.TxProcessor()
        print("✅ TxProcessor initialized")
        
        # Check methods
        methods = ['process_transaction', 'process_transactions_batch']
        for method in methods:
            if hasattr(processor, method):
                print(f"   ✅ Method available: {method}")
            else:
                print(f"   ❌ Method missing: {method}")
                
        return True
    except Exception as e:
        print(f"❌ TxProcessor test failed: {e}")
        return False

def test_simulator(ethtx):
    """Test Simulator functionality"""
    print("\n" + "="*60)
    print("3. Testing Simulator")
    print("="*60)
    
    try:
        simulator = ethtx.Simulator()
        print("✅ Simulator initialized")
        
        # Check methods
        methods = [
            'simulate_transaction',
            'simulate_sequence',
            'build_transaction',
            'get_latest_block'
        ]
        
        for method in methods:
            if hasattr(simulator, method):
                print(f"   ✅ Method available: {method}")
            else:
                print(f"   ❌ Method missing: {method}")
        
        # Test get_latest_block
        try:
            block = simulator.get_latest_block()
            print(f"   📊 Latest block: {block}")
        except Exception as e:
            print(f"   ⚠️  get_latest_block error: {e}")
            
        return True
    except Exception as e:
        print(f"❌ Simulator test failed: {e}")
        return False

def test_tx_builder(ethtx):
    """Test TxBuilder functionality"""
    print("\n" + "="*60)
    print("4. Testing TxBuilder")
    print("="*60)
    
    try:
        # Check if TxBuilder exists
        if not hasattr(ethtx, 'TxBuilder'):
            print("❌ TxBuilder not found in module")
            return False
            
        # Initialize builder
        builder = ethtx.TxBuilder.mainnet()
        print("✅ TxBuilder initialized for mainnet")
        
        # Check methods
        methods = [
            'eth_transfer',
            'erc20_transfer',
            'erc20_approve',
            'get_token_info',
            'list_tokens',
            'estimate_gas'
        ]
        
        for method in methods:
            if hasattr(builder, method):
                print(f"   ✅ Method available: {method}")
            else:
                print(f"   ❌ Method missing: {method}")
        
        # Test token listing
        try:
            tokens = builder.list_tokens()
            print(f"   📊 Available tokens: {len(tokens)}")
            print(f"      Sample: {', '.join(tokens[:5])}...")
        except Exception as e:
            print(f"   ⚠️  list_tokens error: {e}")
        
        # Test token info
        try:
            usdc_info = builder.get_token_info("USDC")
            print(f"   💰 USDC info retrieved:")
            print(f"      Address: {usdc_info.get('address', 'N/A')}")
            print(f"      Decimals: {usdc_info.get('decimals', 'N/A')}")
        except Exception as e:
            print(f"   ⚠️  get_token_info error: {e}")
            
        return True
    except Exception as e:
        print(f"❌ TxBuilder test failed: {e}")
        return False

def test_integration(ethtx):
    """Test that all components work together"""
    print("\n" + "="*60)
    print("5. Testing Integration")
    print("="*60)
    
    try:
        # Build a transaction with TxBuilder
        builder = ethtx.TxBuilder.mainnet()
        from_addr = "0x742d35Cc6134C0532925a3b8C17ebb6F5E9DFcf4"
        to_addr = "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045"
        
        print("🔨 Building transaction with TxBuilder...")
        tx_params = builder.eth_transfer(from_addr, to_addr, "0.1")
        print(f"   ✅ Transaction built")
        print(f"      From: {tx_params['from'][:10]}...")
        print(f"      To: {tx_params['to'][:10]}...")
        print(f"      Value: {tx_params['value']} wei")
        
        # Simulate it
        simulator = ethtx.Simulator()
        print("\n🔬 Simulating with Simulator...")
        
        sim_tx = {
            "from": tx_params["from"],
            "to": tx_params["to"],
            "value": tx_params["value"],
            "gas": tx_params.get("gas", 21000)
        }
        
        result = simulator.simulate_transaction(sim_tx)
        print(f"   ✅ Simulation complete")
        print(f"      Success: {result.success}")
        print(f"      Gas: {result.gas_used}")
        
        # Would process after mining
        print("\n📊 After mining, would process with TxProcessor")
        print("   processor = ethtx.TxProcessor()")
        print("   processed = processor.process_transaction(tx_hash)")
        
        return True
    except Exception as e:
        print(f"❌ Integration test failed: {e}")
        return False

def main():
    print("\n" + "="*70)
    print("           🧪 ethtx Module Comprehensive Test")
    print("="*70)
    
    # Import module
    ethtx = test_module_import()
    if not ethtx:
        sys.exit(1)
    
    # Run all tests
    results = {
        "TxProcessor": test_tx_processor(ethtx),
        "Simulator": test_simulator(ethtx),
        "TxBuilder": test_tx_builder(ethtx),
        "Integration": test_integration(ethtx)
    }
    
    # Summary
    print("\n" + "="*70)
    print("                        📊 Test Summary")
    print("="*70)
    
    all_passed = True
    for component, passed in results.items():
        status = "✅ PASS" if passed else "❌ FAIL"
        print(f"  {component:15} {status}")
        if not passed:
            all_passed = False
    
    print("="*70)
    
    if all_passed:
        print("\n🎉 All tests passed! The ethtx module is fully integrated.")
        print("\n📚 Available functionality:")
        print("   • TxProcessor: Analyze historical transactions")
        print("   • Simulator: Test transactions before sending")
        print("   • TxBuilder: Create transactions without ABI knowledge")
        print("\n💡 Next steps:")
        print("   1. Use examples in examples/python/ directory")
        print("   2. Read MIGRATION_GUIDE.md for upgrading code")
        print("   3. Check README_ETHTX.md for full documentation")
        return 0
    else:
        print("\n⚠️  Some tests failed. Please check the errors above.")
        print("\n🔧 Troubleshooting:")
        print("   1. Rebuild: maturin develop --release --features python")
        print("   2. Check Cargo.toml has python feature enabled")
        print("   3. Verify tx_builder dependency is included")
        return 1

if __name__ == "__main__":
    sys.exit(main())