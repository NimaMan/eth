#!/usr/bin/env python3
"""
Basic PyReth Transaction Simulation Examples

This demonstrates fundamental simulation capabilities using the refactored
PyReth singleton pattern. Shows ETH transfers, gas estimation, and error handling.

Algorithm:
1. Initialize PyReth singleton for shared database connection
2. Get simulator component from PyReth instance
3. Define transaction parameters with proper type handling
4. Execute simulation and interpret results
5. Handle common error scenarios (insufficient funds, invalid nonce, etc.)
"""

from pyreth import simulator as pyreth_simulator
from typing import Dict, Any, Optional

def create_eth_transfer_tx(
    from_addr: str,
    to_addr: str,
    value_wei: int,
    gas_price_wei: int = 20_000_000_000,  # 20 gwei
    gas_limit: int = 21000,
    nonce: int = 0
) -> Dict[str, Any]:
    """
    Create a basic ETH transfer transaction dictionary
    
    Args:
        from_addr: Sender address (0x...)
        to_addr: Recipient address (0x...)
        value_wei: Amount in wei (int)
        gas_price_wei: Gas price in wei (default: 20 gwei)
        gas_limit: Gas limit (default: 21000 for ETH transfer)
        nonce: Transaction nonce
    
    Returns:
        Transaction dictionary ready for simulation
    """
    return {
        'from': from_addr,
        'to': to_addr,
        'value': value_wei,
        'gas': gas_limit,
        'gas_price': gas_price_wei,
        'nonce': nonce
    }

def simulate_eth_transfer():
    """Demonstrate basic ETH transfer simulation"""
    
    print("=" * 60)
    print("BASIC ETH TRANSFER SIMULATION")
    print("=" * 60)
    
    # Initialize PyReth singleton
    simulator = pyreth_simulator()
    print("✅ PyReth simulator initialized")
    
    # Test 1: Small self-transfer from funded address
    print("\n1. Testing small self-transfer")
    print("-" * 30)
    
    vitalik_addr = "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045"
    
    tx = create_eth_transfer_tx(
        from_addr=vitalik_addr,
        to_addr=vitalik_addr,
        value_wei=1_000_000_000_000_000,  # 0.001 ETH
        nonce=0  # Will fail due to incorrect nonce, but shows validation works
    )
    
    print(f"Transaction: {tx['value'] / 1e18:.3f} ETH self-transfer")
    print(f"From/To: {tx['from']}")
    print(f"Gas limit: {tx['gas']:,}")
    print(f"Gas price: {tx['gas_price'] / 1e9:.1f} gwei")
    
    try:
        result = simulator.simulate_transaction(tx, None)
        print(f"✅ Simulation completed:")
        print(f"   Success: {result.success}")
        print(f"   Gas used: {result.gas_used:,}")
        if result.revert_reason:
            print(f"   Revert reason: {result.revert_reason}")
    except Exception as e:
        print(f"⚠️  Simulation error: {str(e)[:80]}...")
    
    # Test 2: Transfer between different addresses
    print("\n2. Testing transfer to different address")
    print("-" * 30)
    
    tx2 = create_eth_transfer_tx(
        from_addr=vitalik_addr,
        to_addr="0x742d35cc6568966c0f99d1b7bd0a99b4c2b4d6b8",  # Different address
        value_wei=500_000_000_000_000,  # 0.0005 ETH
        nonce=0
    )
    
    print(f"Transaction: {tx2['value'] / 1e18:.4f} ETH transfer")
    print(f"From: {tx2['from']}")
    print(f"To: {tx2['to']}")
    
    try:
        result = simulator.simulate_transaction(tx2, None)
        print(f"✅ Simulation completed:")
        print(f"   Success: {result.success}")
        print(f"   Gas used: {result.gas_used:,}")
        if result.revert_reason:
            print(f"   Revert reason: {result.revert_reason}")
    except Exception as e:
        print(f"⚠️  Simulation error: {str(e)[:80]}...")

def demonstrate_error_scenarios():
    """Show common simulation error scenarios"""
    
    print("\n" + "=" * 60)
    print("ERROR SCENARIO DEMONSTRATIONS")
    print("=" * 60)
    
    simulator = pyreth_simulator()
    
    # Error 1: Insufficient funds
    print("\n1. Insufficient funds scenario")
    print("-" * 30)
    
    empty_addr = "0x742d35cc6568966c0f99d1b7bd0a99b4c2b4d6b8"
    
    tx = create_eth_transfer_tx(
        from_addr=empty_addr,
        to_addr="0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045",
        value_wei=1_000_000_000_000_000_000,  # 1 ETH
        nonce=0
    )
    
    print(f"Attempting to send 1 ETH from likely empty address")
    print(f"From: {tx['from']}")
    
    try:
        result = simulator.simulate_transaction(tx, None)
        print(f"Unexpected success: {result.success}")
    except Exception as e:
        error_msg = str(e)
        if "lack of funds" in error_msg.lower():
            print(f"✅ Expected error: Insufficient funds")
            print(f"   Details: {error_msg[:100]}...")
        else:
            print(f"⚠️  Unexpected error: {error_msg[:80]}...")
    
    # Error 2: Invalid address format
    print("\n2. Invalid address format")
    print("-" * 30)
    
    try:
        tx_invalid = create_eth_transfer_tx(
            from_addr="invalid_address",
            to_addr="0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045",
            value_wei=1000,
            nonce=0
        )
        result = simulator.simulate_transaction(tx_invalid, None)
        print(f"Unexpected success with invalid address")
    except Exception as e:
        error_msg = str(e)
        if "invalid" in error_msg.lower() or "address" in error_msg.lower():
            print(f"✅ Expected error: Invalid address format")
            print(f"   Details: {error_msg[:100]}...")
        else:
            print(f"⚠️  Unexpected error: {error_msg[:80]}...")
    
    # Error 3: Gas limit too low
    print("\n3. Gas limit too low")
    print("-" * 30)
    
    tx_low_gas = create_eth_transfer_tx(
        from_addr="0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045",
        to_addr="0x742d35cc6568966c0f99d1b7bd0a99b4c2b4d6b8",
        value_wei=1000,
        gas_limit=1000,  # Too low for ETH transfer (needs 21000)
        nonce=0
    )
    
    print(f"Attempting ETH transfer with gas limit: {tx_low_gas['gas']:,}")
    print("(Standard ETH transfer needs 21,000 gas)")
    
    try:
        result = simulator.simulate_transaction(tx_low_gas, None)
        print(f"✅ Simulation completed:")
        print(f"   Success: {result.success}")
        print(f"   Gas used: {result.gas_used:,}")
        if not result.success:
            print(f"   Revert reason: {result.revert_reason}")
    except Exception as e:
        print(f"⚠️  Simulation error: {str(e)[:80]}...")

def demonstrate_flexible_types():
    """Show that simulation accepts both string and integer values"""
    
    print("\n" + "=" * 60)
    print("FLEXIBLE TYPE HANDLING DEMONSTRATION")
    print("=" * 60)
    
    simulator = pyreth_simulator()
    
    # Test with integer values
    print("\n1. Using integer values")
    print("-" * 30)
    
    tx_int = {
        'from': '0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045',
        'to': '0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045',
        'value': 1000000000000000,  # Integer
        'gas': 21000,              # Integer
        'gas_price': 20000000000,  # Integer
        'nonce': 0
    }
    
    print("Transaction with integer values:")
    print(f"  value: {tx_int['value']} (int)")
    print(f"  gas: {tx_int['gas']} (int)")
    print(f"  gas_price: {tx_int['gas_price']} (int)")
    
    try:
        result = simulator.simulate_transaction(tx_int, None)
        print(f"✅ Integer types accepted - Gas used: {result.gas_used:,}")
    except Exception as e:
        print(f"❌ Integer type error: {str(e)[:80]}...")
    
    # Test with string values
    print("\n2. Using string values")
    print("-" * 30)
    
    tx_str = {
        'from': '0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045',
        'to': '0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045',
        'value': '1000000000000000',  # String
        'gas': '21000',              # String
        'gas_price': '20000000000',  # String
        'nonce': 0
    }
    
    print("Transaction with string values:")
    print(f"  value: '{tx_str['value']}' (str)")
    print(f"  gas: '{tx_str['gas']}' (str)")
    print(f"  gas_price: '{tx_str['gas_price']}' (str)")
    
    try:
        result = simulator.simulate_transaction(tx_str, None)
        print(f"✅ String types accepted - Gas used: {result.gas_used:,}")
    except Exception as e:
        print(f"❌ String type error: {str(e)[:80]}...")
    
    # Test with mixed types
    print("\n3. Using mixed types")
    print("-" * 30)
    
    tx_mixed = {
        'from': '0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045',
        'to': '0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045',
        'value': '1000000000000000',  # String
        'gas': 21000,                # Integer
        'gas_price': '20000000000',  # String
        'nonce': 0
    }
    
    print("Transaction with mixed types:")
    print(f"  value: '{tx_mixed['value']}' (str)")
    print(f"  gas: {tx_mixed['gas']} (int)")
    print(f"  gas_price: '{tx_mixed['gas_price']}' (str)")
    
    try:
        result = simulator.simulate_transaction(tx_mixed, None)
        print(f"✅ Mixed types accepted - Gas used: {result.gas_used:,}")
    except Exception as e:
        print(f"❌ Mixed type error: {str(e)[:80]}...")

def main():
    """Run all basic simulation demonstrations"""
    
    print("🧪 PyReth Basic Simulation Examples")
    print("🔗 Using singleton pattern for shared database connection")
    print("📚 Demonstrates ETH transfers, error handling, and type flexibility")
    
    try:
        # Basic simulations
        simulate_eth_transfer()
        
        # Error scenarios
        demonstrate_error_scenarios()
        
        # Type flexibility
        demonstrate_flexible_types()
        
        print("\n" + "=" * 60)
        print("✅ BASIC SIMULATION EXAMPLES COMPLETED")
        print("=" * 60)
        print("\n🎯 Key Takeaways:")
        print("  • simulator() provides robust transaction simulation")
        print("  • Singleton pattern ensures efficient database usage")
        print("  • Supports both integer and string parameter types")
        print("  • Provides detailed error messages for debugging")
        print("  • Validates transactions before execution")
        
    except Exception as e:
        print(f"\n❌ Example failed with error: {e}")
        raise

if __name__ == "__main__":
    main()
