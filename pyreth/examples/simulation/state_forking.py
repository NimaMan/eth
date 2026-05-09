#!/usr/bin/env python3
"""
State Forking Simulation

Demonstrates simulating transactions against different historical blockchain states,
allowing for "what if" analysis at different points in time.

Algorithm:
1. Initialize PyReth simulator with singleton pattern
2. Define transactions to test against different block states
3. Simulate same transaction at different historical blocks
4. Analyze how blockchain state changes affect transaction outcomes
5. Demonstrate time-sensitive contract interactions
"""

from pyreth import simulator as pyreth_simulator
from typing import Dict, Any, List, Optional, Tuple

# Test addresses and contracts
VITALIK_ADDRESS = "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045"
USDC_ADDRESS = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
WETH_ADDRESS = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"
UNI_V2_ROUTER = "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D"

def encode_erc20_transfer(to_address: str, amount: int) -> str:
    """Encode ERC20 transfer function call"""
    selector = "a9059cbb"
    address_padded = to_address.replace("0x", "").lower().zfill(64)
    amount_hex = format(amount, '064x')
    return f"0x{selector}{address_padded}{amount_hex}"

def simulate_at_different_blocks():
    """Simulate the same transaction at different historical blocks"""
    
    print("=" * 80)
    print("HISTORICAL BLOCK SIMULATION")
    print("=" * 80)
    print("Testing how blockchain state affects transaction outcomes")
    
    simulator = pyreth_simulator()
    
    # Define a test transaction
    test_tx = {
        'from': VITALIK_ADDRESS,
        'to': '0x742d35cc6568966c0f99d1b7bd0a99b4c2b4d6b8',
        'value': 100_000_000_000_000_000,  # 0.1 ETH
        'gas': 21000,
        'gas_price': 20_000_000_000,
        'nonce': 100  # Using lower nonce to test historical compatibility
    }
    
    # Test at different block numbers (historical points)
    test_blocks = [
        (None, "Latest Block"),
        (20_000_000, "Block 20M (Recent)"),
        (19_000_000, "Block 19M (Older)"),
        (18_000_000, "Block 18M (Historical)"),
    ]
    
    print(f"Test transaction: {test_tx['value'] / 1e18:.1f} ETH transfer")
    print(f"From: {test_tx['from']}")
    print(f"To: {test_tx['to']}")
    print(f"Nonce: {test_tx['nonce']}")
    
    results = []
    
    for block_number, block_description in test_blocks:
        print(f"\n🔍 Testing at {block_description}")
        print("-" * 40)
        
        try:
            result = simulator.simulate_transaction(test_tx, block_number)
            
            if result.success:
                print(f"✅ Transaction would succeed")
                print(f"   Gas used: {result.gas_used:,}")
                status = 'success'
            else:
                print(f"❌ Transaction would fail")
                print(f"   Revert reason: {result.revert_reason}")
                print(f"   Gas used: {result.gas_used:,}")
                status = 'failed'
            
            results.append({
                'block': block_number,
                'description': block_description,
                'success': result.success,
                'gas_used': result.gas_used,
                'revert_reason': result.revert_reason if not result.success else None
            })
            
        except Exception as e:
            error_msg = str(e)
            print(f"⚠️  Simulation error: {error_msg[:80]}...")
            
            results.append({
                'block': block_number,
                'description': block_description,
                'success': False,
                'error': error_msg
            })
    
    # Analysis
    print(f"\n📊 Historical Analysis Summary:")
    print("-" * 50)
    
    successful_blocks = [r for r in results if r.get('success', False)]
    failed_blocks = [r for r in results if not r.get('success', False)]
    
    print(f"Successful at {len(successful_blocks)} blocks")
    print(f"Failed at {len(failed_blocks)} blocks")
    
    if successful_blocks:
        gas_usage = [r['gas_used'] for r in successful_blocks if 'gas_used' in r]
        if gas_usage:
            print(f"Gas usage consistency: {len(set(gas_usage)) == 1}")
            print(f"Gas range: {min(gas_usage):,} - {max(gas_usage):,}")
    
    return results

def test_contract_evolution():
    """Test how contract changes affect transaction outcomes over time"""
    
    print("\n" + "=" * 80)
    print("CONTRACT EVOLUTION TESTING")
    print("=" * 80)
    print("Testing contract interactions across different deployment states")
    
    simulator = pyreth_simulator()
    
    # Test USDC transfer at different historical points
    # (USDC was deployed at block ~6082465, became widely used later)
    
    usdc_transfer_tx = {
        'from': VITALIK_ADDRESS,
        'to': USDC_ADDRESS,
        'value': 0,
        'data': encode_erc20_transfer("0x742d35cc6568966c0f99d1b7bd0a99b4c2b4d6b8", 100 * 10**6),  # 100 USDC
        'gas': 80000,
        'gas_price': 20_000_000_000,
        'nonce': 200  # Lower nonce for historical compatibility
    }
    
    # Historical blocks for testing contract evolution
    contract_test_blocks = [
        (None, "Current State"),
        (18_000_000, "Block 18M - USDC Mature"),
        (15_000_000, "Block 15M - USDC Growing"),
        (12_000_000, "Block 12M - USDC Early Days"),
    ]
    
    print(f"Testing USDC transfer evolution:")
    print(f"Contract: {USDC_ADDRESS}")
    print(f"Amount: 100 USDC")
    
    for block_number, block_description in contract_test_blocks:
        print(f"\n🔍 {block_description}")
        print("-" * 30)
        
        try:
            result = simulator.simulate_transaction(usdc_transfer_tx, block_number)
            
            if result.success:
                print(f"✅ USDC transfer would succeed")
                print(f"   Gas used: {result.gas_used:,}")
                
                # At this block, USDC contract exists and functions
                print(f"   Contract fully operational at this state")
            else:
                print(f"❌ USDC transfer would fail")
                print(f"   Revert reason: {result.revert_reason}")
                
                # Analyze why it failed
                if "insufficient" in result.revert_reason.lower():
                    print(f"   💡 Likely insufficient USDC balance at this historical state")
                else:
                    print(f"   💡 Contract may not exist or be functional at this block")
                    
        except Exception as e:
            error_msg = str(e)
            print(f"⚠️  Error: {error_msg[:60]}...")
            
            # Contract might not exist at this block
            if "contract" in error_msg.lower() or "code" in error_msg.lower():
                print(f"   💡 Contract likely not deployed at block {block_number}")

def analyze_defi_state_sensitivity():
    """Analyze how DeFi protocol states change over time"""
    
    print("\n" + "=" * 80)
    print("DEFI STATE SENSITIVITY ANALYSIS")
    print("=" * 80)
    print("Analyzing how DeFi interactions change with blockchain state")
    
    simulator = pyreth_simulator()
    
    # Test different DeFi operations across time
    defi_scenarios = [
        {
            'name': 'WETH Wrapping',
            'tx': {
                'from': VITALIK_ADDRESS,
                'to': WETH_ADDRESS,
                'value': 1_000_000_000_000_000_000,  # 1 ETH
                'data': '0xd0e30db0',  # deposit() function selector
                'gas': 50000,
                'gas_price': 20_000_000_000,
                'nonce': 150
            },
            'expected_behavior': 'Should work consistently across all blocks'
        },
        {
            'name': 'High-Value ETH Transfer',
            'tx': {
                'from': VITALIK_ADDRESS,
                'to': '0x742d35cc6568966c0f99d1b7bd0a99b4c2b4d6b8',
                'value': 10_000_000_000_000_000_000,  # 10 ETH
                'gas': 21000,
                'gas_price': 50_000_000_000,  # 50 gwei
                'nonce': 151
            },
            'expected_behavior': 'May fail if balance was lower historically'
        }
    ]
    
    test_blocks = [
        (None, "Latest"),
        (20_000_000, "Recent"),
        (18_000_000, "Older"),
    ]
    
    for scenario in defi_scenarios:
        print(f"\n📋 Testing: {scenario['name']}")
        print(f"Expected: {scenario['expected_behavior']}")
        print("-" * 50)
        
        scenario_results = []
        
        for block_number, block_desc in test_blocks:
            try:
                result = simulator.simulate_transaction(scenario['tx'], block_number)
                
                success_indicator = "✅" if result.success else "❌"
                print(f"{success_indicator} {block_desc} Block: Gas {result.gas_used:,}")
                
                if not result.success:
                    print(f"   Reason: {result.revert_reason}")
                
                scenario_results.append({
                    'block': block_desc,
                    'success': result.success,
                    'gas_used': result.gas_used
                })
                
            except Exception as e:
                error_msg = str(e)
                print(f"⚠️  {block_desc} Block: Error - {error_msg[:40]}...")
        
        # Analyze consistency
        if scenario_results:
            successes = [r['success'] for r in scenario_results]
            if all(successes):
                print(f"   ✅ Consistent success across all tested blocks")
            elif not any(successes):
                print(f"   ❌ Consistent failure across all tested blocks")
            else:
                print(f"   ⚠️  Mixed results - state-dependent behavior")

def demonstrate_time_travel_debugging():
    """Show how to use historical simulation for debugging"""
    
    print("\n" + "=" * 80)
    print("TIME-TRAVEL DEBUGGING")
    print("=" * 80)
    print("Using historical simulation to debug transaction issues")
    
    simulator = pyreth_simulator()
    
    # Simulate debugging a failed transaction by testing at different blocks
    problematic_tx = {
        'from': VITALIK_ADDRESS,
        'to': USDC_ADDRESS,
        'value': 0,
        'data': encode_erc20_transfer("0x742d35cc6568966c0f99d1b7bd0a99b4c2b4d6b8", 1_000_000 * 10**6),  # 1M USDC
        'gas': 100000,
        'gas_price': 30_000_000_000,
        'nonce': 300
    }
    
    print("🔍 Debugging large USDC transfer:")
    print(f"Amount: {1_000_000:,} USDC")
    print("This might fail due to insufficient balance - let's check different times")
    
    debug_blocks = [
        (None, "Current State", "Does it work now?"),
        (19_500_000, "Block 19.5M", "Did it work recently?"),
        (19_000_000, "Block 19M", "Did it work a while ago?"),
        (18_500_000, "Block 18.5M", "Did it work even earlier?"),
    ]
    
    working_blocks = []
    failing_blocks = []
    
    for block_number, block_desc, question in debug_blocks:
        print(f"\n❓ {question} ({block_desc})")
        print("-" * 40)
        
        try:
            result = simulator.simulate_transaction(problematic_tx, block_number)
            
            if result.success:
                print(f"✅ SUCCESS at {block_desc}")
                print(f"   Gas used: {result.gas_used:,}")
                print(f"   💡 Transaction would have worked at this block")
                working_blocks.append((block_number, block_desc))
            else:
                print(f"❌ FAILED at {block_desc}")
                print(f"   Revert reason: {result.revert_reason}")
                print(f"   💡 Same issue exists at this historical point")
                failing_blocks.append((block_number, block_desc, result.revert_reason))
                
        except Exception as e:
            error_msg = str(e)
            print(f"⚠️  ERROR at {block_desc}: {error_msg[:50]}...")
            failing_blocks.append((block_number, block_desc, error_msg))
    
    # Debugging conclusions
    print(f"\n🔬 Debugging Analysis:")
    print("-" * 30)
    
    if working_blocks:
        print(f"✅ Transaction worked at {len(working_blocks)} block(s):")
        for block_num, desc in working_blocks:
            print(f"   • {desc}")
    
    if failing_blocks:
        print(f"❌ Transaction failed at {len(failing_blocks)} block(s):")
        for block_num, desc, reason in failing_blocks:
            print(f"   • {desc}: {reason[:50]}...")
    
    if len(working_blocks) == 0 and len(failing_blocks) > 0:
        print("💡 Issue is persistent across time - likely a fundamental problem")
    elif len(working_blocks) > 0 and len(failing_blocks) > 0:
        print("💡 Issue is state-dependent - balance or contract state changed")
    else:
        print("💡 Unable to determine issue pattern from this sample")

def main():
    """Run all state forking simulation demonstrations"""
    
    print("🧪 PyReth State Forking Examples")
    print("⏰ Time-travel simulation for historical analysis")
    print("🔗 Using singleton pattern for efficient database access")
    
    try:
        # Historical block simulation
        historical_results = simulate_at_different_blocks()
        
        # Contract evolution testing
        test_contract_evolution()
        
        # DeFi state sensitivity
        analyze_defi_state_sensitivity()
        
        # Time-travel debugging
        demonstrate_time_travel_debugging()
        
        print("\n" + "=" * 80)
        print("✅ STATE FORKING EXAMPLES COMPLETED")
        print("=" * 80)
        print("\n🎯 Key Takeaways:")
        print("  • Historical simulation enables 'what if' analysis")
        print("  • Contract deployments and upgrades affect transaction outcomes")
        print("  • Account balances change over time affecting transaction viability")
        print("  • Time-travel debugging helps understand transaction failures")
        print("  • Same transaction may succeed/fail at different blockchain states")
        print("  • Historical testing validates transaction logic across time")
        
    except Exception as e:
        print(f"\n❌ State forking examples failed: {e}")
        raise

if __name__ == "__main__":
    main()