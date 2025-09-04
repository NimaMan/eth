#!/usr/bin/env python3
"""
Error Handling in PyReth Simulation

Comprehensive demonstration of error scenarios and how to handle them
when using PyReth transaction simulation.

Algorithm:
1. Initialize PyReth simulator with singleton pattern
2. Create transactions with various error conditions
3. Simulate and capture different types of errors
4. Categorize errors (validation vs execution errors)
5. Provide guidance on error resolution and debugging
"""

import pyreth
from typing import Dict, Any, List, Optional, Tuple

# Test addresses and contracts
VITALIK_ADDRESS = "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045"
EMPTY_ADDRESS = "0x742d35cc6568966c0f99d1b7bd0a99b4c2b4d6b8"
USDC_ADDRESS = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
NONEXISTENT_ADDRESS = "0x0000000000000000000000000000000000000001"

def encode_erc20_transfer(to_address: str, amount: int) -> str:
    """Encode ERC20 transfer function call"""
    selector = "a9059cbb"
    address_padded = to_address.replace("0x", "").lower().zfill(64)
    amount_hex = format(amount, '064x')
    return f"0x{selector}{address_padded}{amount_hex}"

def simulate_with_error_handling(simulator, tx: Dict[str, Any], scenario_name: str) -> Tuple[bool, Optional[str], Optional[dict]]:
    """
    Simulate transaction with comprehensive error handling
    
    Args:
        simulator: PyReth simulator instance
        tx: Transaction dictionary
        scenario_name: Description of the test scenario
    
    Returns:
        Tuple of (success, error_message, result_data)
    """
    print(f"\n🧪 Testing: {scenario_name}")
    print("-" * 60)
    
    # Display transaction details
    print(f"Transaction details:")
    for key, value in tx.items():
        if key == 'data' and value and len(str(value)) > 50:
            print(f"  {key}: {str(value)[:42]}... ({len(str(value))} chars)")
        else:
            print(f"  {key}: {value}")
    
    try:
        result = simulator.simulate_transaction(tx, None)
        
        if result.success:
            print(f"✅ Simulation successful:")
            print(f"   Gas used: {result.gas_used:,}")
            return True, None, {
                'success': True,
                'gas_used': result.gas_used,
                'result': result
            }
        else:
            print(f"❌ Transaction would revert:")
            print(f"   Gas used before revert: {result.gas_used:,}")
            print(f"   Revert reason: {result.revert_reason}")
            return False, result.revert_reason, {
                'success': False,
                'gas_used': result.gas_used,
                'revert_reason': result.revert_reason,
                'result': result
            }
            
    except Exception as e:
        error_msg = str(e)
        print(f"⚠️  Simulation error: {error_msg}")
        
        # Categorize the error
        error_type = categorize_error(error_msg)
        print(f"   Error type: {error_type}")
        
        # Provide guidance
        guidance = get_error_guidance(error_msg)
        if guidance:
            print(f"   💡 Guidance: {guidance}")
        
        return False, error_msg, {
            'success': False,
            'error': error_msg,
            'error_type': error_type,
            'guidance': guidance
        }

def categorize_error(error_msg: str) -> str:
    """Categorize error messages into types"""
    error_msg_lower = error_msg.lower()
    
    if any(keyword in error_msg_lower for keyword in ["nonce", "too low", "too high"]):
        return "Nonce Error"
    elif any(keyword in error_msg_lower for keyword in ["lack of funds", "insufficient", "balance"]):
        return "Insufficient Funds"
    elif any(keyword in error_msg_lower for keyword in ["invalid", "address", "format"]):
        return "Invalid Address"
    elif any(keyword in error_msg_lower for keyword in ["gas", "limit", "out of gas"]):
        return "Gas Error"
    elif any(keyword in error_msg_lower for keyword in ["revert", "execution reverted"]):
        return "Contract Revert"
    elif any(keyword in error_msg_lower for keyword in ["intrinsic", "data"]):
        return "Invalid Transaction Data"
    else:
        return "Unknown Error"

def get_error_guidance(error_msg: str) -> Optional[str]:
    """Provide guidance based on error message"""
    error_msg_lower = error_msg.lower()
    
    if "nonce" in error_msg_lower:
        if "too low" in error_msg_lower:
            return "Use a higher nonce value - this nonce has already been used"
        elif "too high" in error_msg_lower:
            return "Use a lower nonce value - this nonce is too far in the future"
        else:
            return "Check the current nonce for this address"
    
    elif "lack of funds" in error_msg_lower or "insufficient" in error_msg_lower:
        return "Address doesn't have enough ETH to cover value + gas costs"
    
    elif "invalid" in error_msg_lower and "address" in error_msg_lower:
        return "Check address format - should be 0x followed by 40 hex characters"
    
    elif "gas" in error_msg_lower:
        if "limit" in error_msg_lower:
            return "Increase gas limit or check if transaction is valid"
        else:
            return "Review gas-related parameters (limit, price)"
    
    elif "revert" in error_msg_lower:
        return "Contract rejected the transaction - check contract state and parameters"
    
    return None

def test_validation_errors():
    """Test various validation errors that occur before execution"""
    
    print("=" * 80)
    print("VALIDATION ERRORS")
    print("=" * 80)
    print("Testing errors that occur during transaction validation")
    
    reth = pyreth.PyReth()
    simulator = reth.simulator()
    
    validation_tests = [
        {
            'name': 'Insufficient Funds',
            'tx': {
                'from': EMPTY_ADDRESS,
                'to': VITALIK_ADDRESS,
                'value': 1_000_000_000_000_000_000,  # 1 ETH
                'gas': 21000,
                'gas_price': 20_000_000_000,
                'nonce': 0
            }
        },
        {
            'name': 'Invalid From Address',
            'tx': {
                'from': 'invalid_address',
                'to': VITALIK_ADDRESS,
                'value': 1000,
                'gas': 21000,
                'gas_price': 20_000_000_000,
                'nonce': 0
            }
        },
        {
            'name': 'Invalid To Address',
            'tx': {
                'from': VITALIK_ADDRESS,
                'to': 'invalid_to_address',
                'value': 1000,
                'gas': 21000,
                'gas_price': 20_000_000_000,
                'nonce': 0
            }
        },
        {
            'name': 'Nonce Too Low',
            'tx': {
                'from': VITALIK_ADDRESS,
                'to': EMPTY_ADDRESS,
                'value': 1000,
                'gas': 21000,
                'gas_price': 20_000_000_000,
                'nonce': 0  # Too low for Vitalik's account
            }
        },
        {
            'name': 'Gas Limit Too Low',
            'tx': {
                'from': VITALIK_ADDRESS,
                'to': EMPTY_ADDRESS,
                'value': 1000,
                'gas': 10000,  # Less than 21000 required for ETH transfer
                'gas_price': 20_000_000_000,
                'nonce': 1500
            }
        }
    ]
    
    results = []
    for test in validation_tests:
        success, error, data = simulate_with_error_handling(simulator, test['tx'], test['name'])
        results.append((test['name'], success, error, data))
    
    # Summary
    print(f"\n📊 Validation Error Summary:")
    print("-" * 50)
    failed_count = sum(1 for _, success, _, _ in results if not success)
    print(f"Total tests: {len(results)}")
    print(f"Expected failures: {failed_count}")
    print(f"All tests behaved as expected: {'✅' if failed_count == len(results) else '❌'}")

def test_contract_execution_errors():
    """Test errors that occur during contract execution"""
    
    print("\n" + "=" * 80)
    print("CONTRACT EXECUTION ERRORS")
    print("=" * 80)
    print("Testing errors that occur during contract execution")
    
    reth = pyreth.PyReth()
    simulator = reth.simulator()
    
    execution_tests = [
        {
            'name': 'ERC20 Transfer Exceeding Balance',
            'tx': {
                'from': VITALIK_ADDRESS,
                'to': USDC_ADDRESS,
                'value': 0,
                'data': encode_erc20_transfer(EMPTY_ADDRESS, 1_000_000_000 * 10**6),  # 1B USDC
                'gas': 100000,
                'gas_price': 20_000_000_000,
                'nonce': 1500
            }
        },
        {
            'name': 'Call to Non-Contract Address',
            'tx': {
                'from': VITALIK_ADDRESS,
                'to': EMPTY_ADDRESS,
                'value': 0,
                'data': encode_erc20_transfer(VITALIK_ADDRESS, 100 * 10**6),
                'gas': 100000,
                'gas_price': 20_000_000_000,
                'nonce': 1500
            }
        },
        {
            'name': 'Invalid Function Selector',
            'tx': {
                'from': VITALIK_ADDRESS,
                'to': USDC_ADDRESS,
                'value': 0,
                'data': '0xdeadbeef' + '0' * 128,  # Invalid function selector
                'gas': 100000,
                'gas_price': 20_000_000_000,
                'nonce': 1500
            }
        },
        {
            'name': 'Contract Call with ETH to Non-Payable',
            'tx': {
                'from': VITALIK_ADDRESS,
                'to': USDC_ADDRESS,
                'value': 1_000_000_000_000_000,  # 0.001 ETH
                'data': encode_erc20_transfer(EMPTY_ADDRESS, 100 * 10**6),
                'gas': 100000,
                'gas_price': 20_000_000_000,
                'nonce': 1500
            }
        }
    ]
    
    results = []
    for test in execution_tests:
        success, error, data = simulate_with_error_handling(simulator, test['tx'], test['name'])
        results.append((test['name'], success, error, data))
    
    # Analyze results
    print(f"\n📊 Contract Execution Error Summary:")
    print("-" * 60)
    for name, success, error, data in results:
        status = "✅ Handled" if not success else "⚠️  Unexpected Success"
        print(f"{name}: {status}")
        if not success and data and 'error_type' in data:
            print(f"   Type: {data['error_type']}")

def test_edge_cases():
    """Test edge cases and unusual scenarios"""
    
    print("\n" + "=" * 80)
    print("EDGE CASES AND UNUSUAL SCENARIOS")
    print("=" * 80)
    
    reth = pyreth.PyReth()
    simulator = reth.simulator()
    
    edge_cases = [
        {
            'name': 'Zero Value Transaction',
            'tx': {
                'from': VITALIK_ADDRESS,
                'to': EMPTY_ADDRESS,
                'value': 0,  # Zero ETH
                'gas': 21000,
                'gas_price': 20_000_000_000,
                'nonce': 1500
            }
        },
        {
            'name': 'Very High Gas Limit',
            'tx': {
                'from': VITALIK_ADDRESS,
                'to': EMPTY_ADDRESS,
                'value': 1000,
                'gas': 10_000_000,  # Very high gas limit
                'gas_price': 20_000_000_000,
                'nonce': 1500
            }
        },
        {
            'name': 'Contract Creation with Empty Bytecode',
            'tx': {
                'from': VITALIK_ADDRESS,
                'to': None,  # Contract creation
                'value': 0,
                'data': '0x',  # Empty bytecode
                'gas': 100000,
                'gas_price': 20_000_000_000,
                'nonce': 1500
            }
        },
        {
            'name': 'Self-Transfer',
            'tx': {
                'from': VITALIK_ADDRESS,
                'to': VITALIK_ADDRESS,  # Send to self
                'value': 1_000_000_000_000_000,  # 0.001 ETH
                'gas': 21000,
                'gas_price': 20_000_000_000,
                'nonce': 1500
            }
        }
    ]
    
    successful_cases = 0
    for test in edge_cases:
        success, error, data = simulate_with_error_handling(simulator, test['tx'], test['name'])
        if success:
            successful_cases += 1
    
    print(f"\n📊 Edge Case Summary:")
    print(f"Successful simulations: {successful_cases}/{len(edge_cases)}")

def create_error_handling_guide():
    """Create a comprehensive error handling guide"""
    
    print("\n" + "=" * 80)
    print("ERROR HANDLING GUIDE")
    print("=" * 80)
    
    error_guide = {
        "Nonce Errors": {
            "description": "Transaction nonce is incorrect",
            "common_causes": [
                "Using a nonce that's already been used",
                "Nonce too far in the future",
                "Not accounting for pending transactions"
            ],
            "solutions": [
                "Query current nonce from blockchain",
                "Account for pending transactions in mempool",
                "Use sequential nonces"
            ]
        },
        "Insufficient Funds": {
            "description": "Not enough ETH to cover transaction costs",
            "common_causes": [
                "Account balance too low",
                "Gas price * gas limit exceeds balance",
                "Total cost (value + gas) exceeds balance"
            ],
            "solutions": [
                "Check account balance before simulation",
                "Reduce gas price or gas limit",
                "Reduce transaction value"
            ]
        },
        "Contract Reverts": {
            "description": "Smart contract rejected the transaction",
            "common_causes": [
                "Contract preconditions not met",
                "Insufficient token balance for transfers",
                "Function called with wrong parameters"
            ],
            "solutions": [
                "Check contract state before calling",
                "Validate function parameters",
                "Read contract documentation for requirements"
            ]
        },
        "Invalid Addresses": {
            "description": "Address format is incorrect",
            "common_causes": [
                "Missing 0x prefix",
                "Wrong length (not 40 hex characters)",
                "Invalid hex characters"
            ],
            "solutions": [
                "Use checksum addresses",
                "Validate address format before use",
                "Use address validation libraries"
            ]
        }
    }
    
    for error_type, info in error_guide.items():
        print(f"\n🔍 {error_type}")
        print("-" * 30)
        print(f"Description: {info['description']}")
        print("Common causes:")
        for cause in info['common_causes']:
            print(f"  • {cause}")
        print("Solutions:")
        for solution in info['solutions']:
            print(f"  • {solution}")
    
    print(f"\n💡 General Tips:")
    print("  • Always use try-catch blocks around simulation calls")
    print("  • Parse error messages to categorize error types")
    print("  • Test with realistic transaction parameters")
    print("  • Use proper nonce management for sequential transactions")
    print("  • Validate inputs before simulation to catch obvious errors")

def main():
    """Run all error handling demonstrations"""
    
    print("🧪 PyReth Error Handling Examples")
    print("🚨 Comprehensive error scenario testing")
    print("🔗 Using singleton pattern for efficient database access")
    
    try:
        # Test validation errors
        test_validation_errors()
        
        # Test contract execution errors
        test_contract_execution_errors()
        
        # Test edge cases
        test_edge_cases()
        
        # Error handling guide
        create_error_handling_guide()
        
        print("\n" + "=" * 80)
        print("✅ ERROR HANDLING EXAMPLES COMPLETED")
        print("=" * 80)
        print("\n🎯 Key Takeaways:")
        print("  • Always wrap simulation calls in try-catch blocks")
        print("  • Error messages provide specific guidance for resolution")
        print("  • Validation errors occur before execution (cheaper)")
        print("  • Contract reverts can still consume gas")
        print("  • Proper error handling improves user experience")
        print("  • Test edge cases to ensure robust error handling")
        
    except Exception as e:
        print(f"\n❌ Error handling examples failed: {e}")
        raise

if __name__ == "__main__":
    main()