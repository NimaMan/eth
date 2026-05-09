#!/usr/bin/env python3
"""
Gas Estimation Using PyReth Simulation

Demonstrates how to use transaction simulation for accurate gas estimation
across different transaction types and scenarios.

Algorithm:
1. Initialize PyReth simulator with singleton pattern
2. Define transactions with high gas limits for estimation
3. Execute simulations to determine actual gas usage
4. Compare estimates with actual usage for accuracy analysis
5. Provide gas recommendations with safety margins
"""

from pyreth import simulator as pyreth_simulator
from typing import Dict, Any, List, Tuple, Optional

# Common addresses for testing
VITALIK_ADDRESS = "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045"
USDC_ADDRESS = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
WETH_ADDRESS = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"

def encode_erc20_transfer(to_address: str, amount: int) -> str:
    """Encode ERC20 transfer function call"""
    selector = "a9059cbb"
    address_padded = to_address.replace("0x", "").lower().zfill(64)
    amount_hex = format(amount, '064x')
    return f"0x{selector}{address_padded}{amount_hex}"

def estimate_gas_for_transaction(simulator, tx_dict: Dict[str, Any], operation_name: str) -> Optional[int]:
    """
    Estimate gas for a transaction using simulation
    
    Args:
        simulator: PyReth simulator instance
        tx_dict: Transaction dictionary
        operation_name: Description of the operation
    
    Returns:
        Estimated gas usage, or None if simulation failed
    """
    print(f"\n🔍 Estimating gas for: {operation_name}")
    print("-" * 50)
    
    print(f"Transaction details:")
    print(f"  From: {tx_dict.get('from', 'N/A')}")
    print(f"  To: {tx_dict.get('to', 'N/A')}")
    print(f"  Value: {tx_dict.get('value', 0):,} wei")
    print(f"  Gas limit (for estimation): {tx_dict.get('gas', 0):,}")
    if 'data' in tx_dict and tx_dict['data']:
        print(f"  Data: {tx_dict['data'][:42]}...")
    
    try:
        result = simulator.simulate_transaction(tx_dict, None)
        
        if result.success:
            gas_used = result.gas_used
            gas_limit = tx_dict.get('gas', 0)
            efficiency = (gas_used / gas_limit) * 100 if gas_limit > 0 else 0
            
            print(f"✅ Simulation successful:")
            print(f"   Gas used: {gas_used:,}")
            print(f"   Gas limit: {gas_limit:,}")
            print(f"   Efficiency: {efficiency:.1f}%")
            
            # Recommend gas with safety margin
            recommended_gas = int(gas_used * 1.2)  # 20% safety margin
            print(f"   Recommended gas: {recommended_gas:,} (20% safety margin)")
            
            return gas_used
        else:
            print(f"❌ Transaction would fail:")
            print(f"   Revert reason: {result.revert_reason}")
            print(f"   Gas used before revert: {result.gas_used:,}")
            return None
            
    except Exception as e:
        error_msg = str(e)
        print(f"⚠️  Simulation error: {error_msg[:100]}...")
        
        # Check if it's a validation error vs execution error
        if any(keyword in error_msg.lower() for keyword in ["nonce", "funds", "balance"]):
            print(f"   💡 This is a validation error - transaction params may need adjustment")
        else:
            print(f"   💡 This may be an execution error - check contract interaction")
        
        return None

def demonstrate_eth_transfer_gas_estimation():
    """Estimate gas for ETH transfers"""
    
    print("=" * 80)
    print("ETH TRANSFER GAS ESTIMATION")
    print("=" * 80)
    
    simulator = pyreth_simulator()
    
    # Test 1: Standard ETH transfer
    eth_transfer = {
        'from': VITALIK_ADDRESS,
        'to': '0x742d35cc6568966c0f99d1b7bd0a99b4c2b4d6b8',
        'value': 1_000_000_000_000_000_000,  # 1 ETH
        'gas': 25000,  # Slightly higher than standard 21000
        'gas_price': 20_000_000_000,
        'nonce': 1500  # Reasonable nonce for this address
    }
    
    gas_used = estimate_gas_for_transaction(simulator, eth_transfer, "Standard ETH Transfer")
    
    if gas_used:
        print(f"\n📊 Analysis:")
        print(f"   Standard ETH transfer: {gas_used:,} gas")
        print(f"   Expected: 21,000 gas")
        if gas_used == 21000:
            print(f"   ✅ Matches expected gas usage exactly")
        else:
            difference = gas_used - 21000
            print(f"   ⚠️  Difference: {difference:+,} gas")
    
    # Test 2: ETH transfer to contract address
    contract_transfer = {
        'from': VITALIK_ADDRESS,
        'to': USDC_ADDRESS,  # Contract address
        'value': 1_000_000_000_000_000,  # 0.001 ETH
        'gas': 50000,  # Higher limit for contract interaction
        'gas_price': 20_000_000_000,
        'nonce': 1500
    }
    
    gas_used = estimate_gas_for_transaction(simulator, contract_transfer, "ETH Transfer to Contract")
    
    if gas_used:
        print(f"\n📊 Contract transfer analysis:")
        print(f"   Gas used: {gas_used:,}")
        print(f"   Extra gas vs standard transfer: {gas_used - 21000:+,}")

def demonstrate_erc20_gas_estimation():
    """Estimate gas for ERC20 operations"""
    
    print("\n" + "=" * 80)
    print("ERC20 OPERATIONS GAS ESTIMATION")
    print("=" * 80)
    
    simulator = pyreth_simulator()
    
    # ERC20 operations to test
    operations = [
        ("USDC Transfer (100 USDC)", encode_erc20_transfer("0x742d35cc6568966c0f99d1b7bd0a99b4c2b4d6b8", 100 * 10**6), 80000),
        ("USDC Transfer (Large Amount)", encode_erc20_transfer("0x742d35cc6568966c0f99d1b7bd0a99b4c2b4d6b8", 1_000_000 * 10**6), 80000),
    ]
    
    gas_results = []
    
    for op_name, call_data, estimated_limit in operations:
        tx = {
            'from': VITALIK_ADDRESS,
            'to': USDC_ADDRESS,
            'value': 0,
            'data': call_data,
            'gas': estimated_limit,
            'gas_price': 20_000_000_000,
            'nonce': 1500
        }
        
        gas_used = estimate_gas_for_transaction(simulator, tx, op_name)
        if gas_used:
            gas_results.append((op_name, gas_used, estimated_limit))
    
    # Compare results
    if gas_results:
        print(f"\n📊 ERC20 Gas Usage Comparison:")
        print("-" * 60)
        print(f"{'Operation':<30} {'Actual':<10} {'Limit':<10} {'Efficiency'}")
        print("-" * 60)
        
        for op_name, actual, limit in gas_results:
            efficiency = (actual / limit) * 100
            print(f"{op_name[:29]:<30} {actual:<10,} {limit:<10,} {efficiency:.1f}%")

def demonstrate_gas_optimization():
    """Show how simulation can help optimize gas usage"""
    
    print("\n" + "=" * 80)
    print("GAS OPTIMIZATION ANALYSIS")
    print("=" * 80)
    
    simulator = pyreth_simulator()
    
    # Test different gas prices to see if they affect gas usage
    print("\n🔍 Testing gas price impact on gas usage")
    print("-" * 45)
    
    base_tx = {
        'from': VITALIK_ADDRESS,
        'to': '0x742d35cc6568966c0f99d1b7bd0a99b4c2b4d6b8',
        'value': 1_000_000_000_000_000,  # 0.001 ETH
        'gas': 25000,
        'nonce': 1500
    }
    
    gas_prices = [
        (10_000_000_000, "10 gwei (low)"),
        (20_000_000_000, "20 gwei (standard)"),
        (50_000_000_000, "50 gwei (high)"),
    ]
    
    for gas_price, description in gas_prices:
        tx = base_tx.copy()
        tx['gas_price'] = gas_price
        
        try:
            result = simulator.simulate_transaction(tx, None)
            if result.success:
                print(f"{description}: {result.gas_used:,} gas used")
            else:
                print(f"{description}: Failed - {result.revert_reason}")
        except Exception as e:
            print(f"{description}: Error - {str(e)[:50]}...")
    
    print(f"\n💡 Gas price doesn't affect gas usage - only affects total cost")

def create_gas_estimation_report():
    """Generate a comprehensive gas estimation report"""
    
    print("\n" + "=" * 80)
    print("COMPREHENSIVE GAS ESTIMATION REPORT")
    print("=" * 80)
    
    simulator = pyreth_simulator()
    
    # Test scenarios with expected gas ranges
    test_scenarios = [
        {
            'name': 'ETH Transfer',
            'tx': {
                'from': VITALIK_ADDRESS,
                'to': '0x742d35cc6568966c0f99d1b7bd0a99b4c2b4d6b8',
                'value': 1000000000000000000,
                'gas': 25000,
                'gas_price': 20000000000,
                'nonce': 1500
            },
            'expected_range': (21000, 21000),
            'category': 'Basic Transfer'
        },
        {
            'name': 'ERC20 Transfer',
            'tx': {
                'from': VITALIK_ADDRESS,
                'to': USDC_ADDRESS,
                'value': 0,
                'data': encode_erc20_transfer("0x742d35cc6568966c0f99d1b7bd0a99b4c2b4d6b8", 100 * 10**6),
                'gas': 100000,
                'gas_price': 20000000000,
                'nonce': 1500
            },
            'expected_range': (50000, 80000),
            'category': 'ERC20 Operation'
        },
    ]
    
    print(f"\n📋 Gas Estimation Summary:")
    print("-" * 80)
    print(f"{'Operation':<20} {'Category':<15} {'Actual':<10} {'Expected':<15} {'Status'}")
    print("-" * 80)
    
    total_tests = 0
    successful_tests = 0
    
    for scenario in test_scenarios:
        total_tests += 1
        
        try:
            result = simulator.simulate_transaction(scenario['tx'], None)
            
            if result.success:
                actual_gas = result.gas_used
                min_expected, max_expected = scenario['expected_range']
                
                if min_expected <= actual_gas <= max_expected:
                    status = "✅ In Range"
                    successful_tests += 1
                else:
                    status = "⚠️  Out of Range"
                
                expected_str = f"{min_expected:,}-{max_expected:,}" if min_expected != max_expected else f"{min_expected:,}"
                
                print(f"{scenario['name']:<20} {scenario['category']:<15} {actual_gas:<10,} {expected_str:<15} {status}")
            else:
                print(f"{scenario['name']:<20} {scenario['category']:<15} {'Failed':<10} {'-':<15} ❌ Reverted")
                
        except Exception as e:
            print(f"{scenario['name']:<20} {scenario['category']:<15} {'Error':<10} {'-':<15} ❌ Exception")
    
    print("-" * 80)
    print(f"Summary: {successful_tests}/{total_tests} estimations within expected ranges")
    
    if successful_tests == total_tests:
        print("🎉 All gas estimations are accurate!")
    else:
        print("⚠️  Some estimations may need adjustment")

def main():
    """Run all gas estimation demonstrations"""
    
    print("🧪 PyReth Gas Estimation Examples")
    print("⛽ Using simulation for accurate gas estimation")
    print("🔗 Singleton pattern for efficient database access")
    
    try:
        # Basic ETH transfer estimation
        demonstrate_eth_transfer_gas_estimation()
        
        # ERC20 operation estimation
        demonstrate_erc20_gas_estimation()
        
        # Gas optimization analysis
        demonstrate_gas_optimization()
        
        # Comprehensive report
        create_gas_estimation_report()
        
        print("\n" + "=" * 80)
        print("✅ GAS ESTIMATION EXAMPLES COMPLETED")
        print("=" * 80)
        print("\n🎯 Key Takeaways:")
        print("  • Simulation provides accurate gas estimation")
        print("  • ETH transfers consistently use 21,000 gas")
        print("  • ERC20 operations vary based on contract implementation")
        print("  • Add 10-20% safety margin to simulation results")
        print("  • Gas price affects cost but not gas usage")
        print("  • Failed transactions still consume gas up to revert point")
        
    except Exception as e:
        print(f"\n❌ Gas estimation examples failed: {e}")
        raise

if __name__ == "__main__":
    main()