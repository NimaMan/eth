#!/usr/bin/env python3
"""
Batch Transaction Simulation

Demonstrates simulating multiple transactions in sequence, analyzing
dependencies, and understanding cumulative effects.

Algorithm:
1. Initialize PyReth simulator with singleton pattern
2. Define sequence of related transactions
3. Simulate transactions individually and in batches
4. Analyze state changes and gas consumption
5. Handle inter-transaction dependencies and failures
"""

import pyreth
from typing import Dict, Any, List, Optional, Tuple

# Test addresses and contracts
VITALIK_ADDRESS = "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045"
RECIPIENT_ADDRESS = "0x742d35cc6568966c0f99d1b7bd0a99b4c2b4d6b8"
USDC_ADDRESS = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
WETH_ADDRESS = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"

def encode_erc20_transfer(to_address: str, amount: int) -> str:
    """Encode ERC20 transfer function call"""
    selector = "a9059cbb"
    address_padded = to_address.replace("0x", "").lower().zfill(64)
    amount_hex = format(amount, '064x')
    return f"0x{selector}{address_padded}{amount_hex}"

def encode_erc20_approve(spender_address: str, amount: int) -> str:
    """Encode ERC20 approve function call"""
    selector = "095ea7b3"
    address_padded = spender_address.replace("0x", "").lower().zfill(64)
    amount_hex = format(amount, '064x')
    return f"0x{selector}{address_padded}{amount_hex}"

def create_transaction_batch() -> List[Dict[str, Any]]:
    """Create a batch of related transactions for testing"""
    
    base_nonce = 1500  # Starting nonce for batch
    gas_price = 20_000_000_000  # 20 gwei
    
    batch = [
        # Transaction 1: ETH transfer
        {
            'name': 'ETH Transfer',
            'tx': {
                'from': VITALIK_ADDRESS,
                'to': RECIPIENT_ADDRESS,
                'value': 500_000_000_000_000_000,  # 0.5 ETH
                'gas': 21000,
                'gas_price': gas_price,
                'nonce': base_nonce
            },
            'expected_outcome': 'success',
            'description': 'Transfer 0.5 ETH to recipient'
        },
        
        # Transaction 2: USDC approve
        {
            'name': 'USDC Approve',
            'tx': {
                'from': VITALIK_ADDRESS,
                'to': USDC_ADDRESS,
                'value': 0,
                'data': encode_erc20_approve(RECIPIENT_ADDRESS, 1000 * 10**6),  # Approve 1000 USDC
                'gas': 60000,
                'gas_price': gas_price,
                'nonce': base_nonce + 1
            },
            'expected_outcome': 'success',
            'description': 'Approve recipient to spend 1000 USDC'
        },
        
        # Transaction 3: USDC transfer
        {
            'name': 'USDC Transfer',
            'tx': {
                'from': VITALIK_ADDRESS,
                'to': USDC_ADDRESS,
                'value': 0,
                'data': encode_erc20_transfer(RECIPIENT_ADDRESS, 500 * 10**6),  # Transfer 500 USDC
                'gas': 80000,
                'gas_price': gas_price,
                'nonce': base_nonce + 2
            },
            'expected_outcome': 'depends_on_balance',
            'description': 'Transfer 500 USDC to recipient'
        },
        
        # Transaction 4: Second ETH transfer (might fail due to insufficient funds after previous txs)
        {
            'name': 'Second ETH Transfer',
            'tx': {
                'from': VITALIK_ADDRESS,
                'to': RECIPIENT_ADDRESS,
                'value': 1_000_000_000_000_000_000,  # 1 ETH
                'gas': 21000,
                'gas_price': gas_price,
                'nonce': base_nonce + 3
            },
            'expected_outcome': 'might_fail',
            'description': 'Transfer 1 ETH (might exceed remaining balance)'
        },
        
        # Transaction 5: High gas price transaction
        {
            'name': 'High Gas Price Transfer',
            'tx': {
                'from': VITALIK_ADDRESS,
                'to': RECIPIENT_ADDRESS,
                'value': 100_000_000_000_000_000,  # 0.1 ETH
                'gas': 21000,
                'gas_price': 100_000_000_000,  # 100 gwei (high)
                'nonce': base_nonce + 4
            },
            'expected_outcome': 'success',
            'description': 'Transfer 0.1 ETH with high gas price'
        }
    ]
    
    return batch

def simulate_individual_transactions():
    """Simulate each transaction individually"""
    
    print("=" * 80)
    print("INDIVIDUAL TRANSACTION SIMULATION")
    print("=" * 80)
    print("Simulating each transaction independently")
    
    reth = pyreth.PyReth()
    simulator = reth.simulator()
    
    batch = create_transaction_batch()
    results = []
    
    for i, tx_data in enumerate(batch):
        print(f"\n{i+1}. {tx_data['name']}")
        print("-" * 40)
        print(f"Description: {tx_data['description']}")
        print(f"Expected: {tx_data['expected_outcome']}")
        
        tx = tx_data['tx']
        print(f"Details: {tx['from'][:10]}...→{tx['to'][:10] if tx['to'] else 'CREATE'}... "
              f"({tx.get('value', 0) / 1e18:.3f} ETH, {tx['gas']:,} gas)")
        
        try:
            result = simulator.simulate_transaction(tx, None)
            
            success = result.success
            gas_used = result.gas_used
            
            if success:
                print(f"✅ SUCCESS - Gas used: {gas_used:,}")
                status = 'success'
            else:
                print(f"❌ FAILED - Gas used: {gas_used:,}")
                print(f"   Revert reason: {result.revert_reason}")
                status = 'failed'
            
            results.append({
                'name': tx_data['name'],
                'success': success,
                'gas_used': gas_used,
                'revert_reason': result.revert_reason if not success else None,
                'status': status
            })
            
        except Exception as e:
            error_msg = str(e)
            print(f"⚠️  ERROR: {error_msg[:80]}...")
            results.append({
                'name': tx_data['name'],
                'success': False,
                'error': error_msg,
                'status': 'error'
            })
    
    # Summary
    print(f"\n📊 Individual Simulation Summary:")
    print("-" * 50)
    successful = sum(1 for r in results if r.get('success', False))
    print(f"Total transactions: {len(results)}")
    print(f"Successful: {successful}")
    print(f"Failed: {len(results) - successful}")
    
    total_gas = sum(r.get('gas_used', 0) for r in results if 'gas_used' in r)
    print(f"Total gas used: {total_gas:,}")
    
    return results

def simulate_sequential_batch():
    """Simulate transactions as if they execute in sequence"""
    
    print("\n" + "=" * 80)
    print("SEQUENTIAL BATCH SIMULATION")
    print("=" * 80)
    print("Simulating transactions with sequential nonce management")
    
    reth = pyreth.PyReth()
    simulator = reth.simulator()
    
    # Create modified batch with sequential nonces
    batch = create_transaction_batch()
    
    print(f"\nBatch contains {len(batch)} transactions:")
    for i, tx_data in enumerate(batch):
        tx = tx_data['tx']
        print(f"{i+1}. {tx_data['name']} (nonce: {tx['nonce']})")
    
    # Simulate each transaction
    results = []
    cumulative_gas = 0
    
    for i, tx_data in enumerate(batch):
        print(f"\n--- Transaction {i+1}/{len(batch)}: {tx_data['name']} ---")
        
        tx = tx_data['tx']
        
        try:
            result = simulator.simulate_transaction(tx, None)
            
            if result.success:
                print(f"✅ Transaction {i+1} would succeed")
                print(f"   Gas used: {result.gas_used:,}")
                cumulative_gas += result.gas_used
                status = 'success'
            else:
                print(f"❌ Transaction {i+1} would fail")
                print(f"   Revert reason: {result.revert_reason}")
                print(f"   Gas consumed by failed tx: {result.gas_used:,}")
                cumulative_gas += result.gas_used
                status = 'failed'
                
                # In a real batch, we might want to stop here or skip dependent transactions
                print(f"   💡 Subsequent transactions might be affected")
            
            results.append({
                'index': i + 1,
                'name': tx_data['name'],
                'success': result.success,
                'gas_used': result.gas_used,
                'cumulative_gas': cumulative_gas,
                'revert_reason': result.revert_reason if not result.success else None
            })
            
        except Exception as e:
            error_msg = str(e)
            print(f"⚠️  Transaction {i+1} error: {error_msg[:60]}...")
            
            results.append({
                'index': i + 1,
                'name': tx_data['name'],
                'success': False,
                'error': error_msg,
                'cumulative_gas': cumulative_gas
            })
    
    # Final summary
    print(f"\n📊 Sequential Batch Summary:")
    print("-" * 50)
    successful_count = sum(1 for r in results if r.get('success', False))
    print(f"Successful transactions: {successful_count}/{len(results)}")
    print(f"Total cumulative gas: {cumulative_gas:,}")
    
    avg_gas = cumulative_gas / len(results) if results else 0
    print(f"Average gas per transaction: {avg_gas:,.0f}")
    
    return results

def analyze_gas_optimization():
    """Analyze gas usage patterns and optimization opportunities"""
    
    print("\n" + "=" * 80)
    print("GAS OPTIMIZATION ANALYSIS")
    print("=" * 80)
    
    reth = pyreth.PyReth()
    simulator = reth.simulator()
    
    # Test different gas price strategies
    print("\n🔍 Testing Gas Price Impact")
    print("-" * 35)
    
    base_tx = {
        'from': VITALIK_ADDRESS,
        'to': RECIPIENT_ADDRESS,
        'value': 100_000_000_000_000_000,  # 0.1 ETH
        'gas': 21000,
        'nonce': 1500
    }
    
    gas_prices = [
        (10_000_000_000, "10 gwei (low)"),
        (20_000_000_000, "20 gwei (standard)"),
        (50_000_000_000, "50 gwei (high)"),
        (100_000_000_000, "100 gwei (urgent)"),
    ]
    
    gas_results = []
    
    for gas_price, description in gas_prices:
        tx = base_tx.copy()
        tx['gas_price'] = gas_price
        
        try:
            result = simulator.simulate_transaction(tx, None)
            if result.success:
                total_cost = result.gas_used * gas_price
                gas_results.append((description, gas_price, result.gas_used, total_cost))
                print(f"{description}: {result.gas_used:,} gas, {total_cost / 1e18:.6f} ETH cost")
            else:
                print(f"{description}: Failed - {result.revert_reason}")
        except Exception as e:
            print(f"{description}: Error - {str(e)[:40]}...")
    
    # Gas optimization recommendations
    print(f"\n💡 Gas Optimization Insights:")
    print("-" * 40)
    if gas_results:
        # All should use same gas, different costs
        unique_gas_usage = set(gas_used for _, _, gas_used, _ in gas_results)
        if len(unique_gas_usage) == 1:
            print("✅ Gas usage is consistent across different gas prices")
        else:
            print("⚠️  Gas usage varies with gas price (unexpected)")
        
        lowest_cost = min(total_cost for _, _, _, total_cost in gas_results)
        highest_cost = max(total_cost for _, _, _, total_cost in gas_results)
        cost_difference = (highest_cost - lowest_cost) / 1e18
        
        print(f"Cost range: {lowest_cost/1e18:.6f} to {highest_cost/1e18:.6f} ETH")
        print(f"Maximum savings: {cost_difference:.6f} ETH per transaction")

def simulate_complex_batch_scenario():
    """Simulate a complex real-world batch scenario"""
    
    print("\n" + "=" * 80)
    print("COMPLEX BATCH SCENARIO")
    print("=" * 80)
    print("Simulating a realistic DeFi interaction sequence")
    
    reth = pyreth.PyReth()
    simulator = reth.simulator()
    
    # Complex scenario: User wants to perform multiple DeFi operations
    complex_batch = [
        {
            'name': 'ETH to WETH Wrap',
            'description': 'Wrap ETH to WETH for DeFi protocols',
            'tx': {
                'from': VITALIK_ADDRESS,
                'to': WETH_ADDRESS,
                'value': 2_000_000_000_000_000_000,  # 2 ETH
                'data': '0xd0e30db0',  # deposit() function
                'gas': 50000,
                'gas_price': 25_000_000_000,
                'nonce': 1500
            }
        },
        {
            'name': 'USDC Approval for DEX',
            'description': 'Approve DEX to spend USDC',
            'tx': {
                'from': VITALIK_ADDRESS,
                'to': USDC_ADDRESS,
                'value': 0,
                'data': encode_erc20_approve("0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D", 5000 * 10**6),  # Uniswap V2 Router
                'gas': 60000,
                'gas_price': 25_000_000_000,
                'nonce': 1501
            }
        },
        {
            'name': 'Token Transfer to DEX',
            'description': 'Transfer tokens for trading',
            'tx': {
                'from': VITALIK_ADDRESS,
                'to': USDC_ADDRESS,
                'value': 0,
                'data': encode_erc20_transfer("0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D", 1000 * 10**6),
                'gas': 80000,
                'gas_price': 25_000_000_000,
                'nonce': 1502
            }
        }
    ]
    
    print(f"\nComplex scenario: DeFi interaction sequence")
    print(f"Total transactions: {len(complex_batch)}")
    
    total_gas_estimate = 0
    all_successful = True
    
    for i, tx_data in enumerate(complex_batch):
        print(f"\n{i+1}. {tx_data['name']}")
        print(f"   {tx_data['description']}")
        
        try:
            result = simulator.simulate_transaction(tx_data['tx'], None)
            
            if result.success:
                print(f"   ✅ Estimated gas: {result.gas_used:,}")
                total_gas_estimate += result.gas_used
            else:
                print(f"   ❌ Would fail: {result.revert_reason}")
                print(f"   Gas before revert: {result.gas_used:,}")
                all_successful = False
                
        except Exception as e:
            error_msg = str(e)
            print(f"   ⚠️  Error: {error_msg[:60]}...")
            all_successful = False
    
    # Summary
    print(f"\n📊 Complex Batch Analysis:")
    print("-" * 35)
    print(f"All transactions viable: {'✅' if all_successful else '❌'}")
    print(f"Total estimated gas: {total_gas_estimate:,}")
    
    if total_gas_estimate > 0:
        avg_gas_price = 25_000_000_000  # 25 gwei
        total_cost = total_gas_estimate * avg_gas_price
        print(f"Estimated total cost: {total_cost / 1e18:.6f} ETH")
        print(f"Average cost per transaction: {total_cost / len(complex_batch) / 1e18:.6f} ETH")

def main():
    """Run all batch simulation demonstrations"""
    
    print("🧪 PyReth Batch Simulation Examples")
    print("📦 Multiple transaction simulation and analysis")
    print("🔗 Using singleton pattern for efficient database access")
    
    try:
        # Individual transaction simulation
        individual_results = simulate_individual_transactions()
        
        # Sequential batch simulation
        sequential_results = simulate_sequential_batch()
        
        # Gas optimization analysis
        analyze_gas_optimization()
        
        # Complex batch scenario
        simulate_complex_batch_scenario()
        
        print("\n" + "=" * 80)
        print("✅ BATCH SIMULATION EXAMPLES COMPLETED")
        print("=" * 80)
        print("\n🎯 Key Takeaways:")
        print("  • Batch simulations help analyze transaction sequences")
        print("  • Nonce management is crucial for sequential transactions")
        print("  • Failed transactions still consume gas up to revert point")
        print("  • Gas price affects total cost but not gas usage")
        print("  • Complex DeFi scenarios can be pre-validated with simulation")
        print("  • Singleton pattern maintains efficiency across multiple simulations")
        
    except Exception as e:
        print(f"\n❌ Batch simulation examples failed: {e}")
        raise

if __name__ == "__main__":
    main()