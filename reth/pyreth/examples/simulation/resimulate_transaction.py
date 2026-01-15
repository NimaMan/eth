#!/usr/bin/env python3
"""
Transaction Re-simulation Using PyReth

Function to re-simulate a ProcessedTransaction at different blocks using
the refactored PyReth singleton pattern for efficient database access.

Algorithm:
1. Initialize PyReth singleton for shared database connection
2. Process original transaction to get transaction details
3. Convert ProcessedTransaction to simulation parameters
4. Re-simulate at original or different block numbers
5. Compare results and analyze differences

This allows us to:
1. Test if simulation matches actual execution
2. Debug transactions at different states  
3. Perform "what if" analysis across time
4. Validate transaction logic against historical states
"""

import pyreth
from typing import Optional

def resimulate_transaction(
    processed_tx,
    simulator,
    block_number: Optional[int] = None,
    gas_multiplier: float = 2.0
):
    """
    Re-simulate a processed transaction at a different block
    
    Args:
        processed_tx: The original ProcessedTransaction object
        simulator: PySimulator instance from PyReth
        block_number: Block to simulate at (default: original block)
        gas_multiplier: Multiplier for gas limit (default: 2.0 for safety)
    
    Returns:
        New ProcessedTransaction from simulation
    """
    # Extract fees information
    fees = processed_tx.fees
    
    # Build simulation dictionary from processed transaction
    sim_tx = {
        "from": processed_tx.from_address,
        "to": processed_tx.to_address if processed_tx.to_address else None,
        "value": str(processed_tx.value),
        "data": processed_tx.input,
        "gas": int(fees['gas_used'] * gas_multiplier),  # Use safe gas limit
        "nonce": processed_tx.nonce,
    }
    
    # Handle gas price (could be legacy or EIP-1559)
    if 'max_fee_per_gas' in fees:
        sim_tx["max_fee_per_gas"] = int(fees['max_fee_per_gas'])
        if 'max_priority_fee' in fees:
            sim_tx["max_priority_fee_per_gas"] = int(fees['max_priority_fee'])
    else:
        sim_tx["gas_price"] = int(fees['gas_price'])
    
    # Use specified block or original block
    target_block = block_number if block_number is not None else processed_tx.block_number
    
    print(f"Re-simulating transaction at block {target_block}...")
    print(f"  From: {sim_tx['from']}")
    print(f"  To: {sim_tx['to']}")
    print(f"  Value: {sim_tx['value']}")
    print(f"  Gas Limit: {sim_tx['gas']}")
    print(f"  Data Length: {len(sim_tx['data'])} chars")
    
    # Simulate and return new ProcessedTransaction
    return simulator.simulate_transaction(sim_tx, target_block)


def test_resimulation():
    """Test the resimulation function with real transactions"""
    
    print("=" * 80)
    print("TRANSACTION RE-SIMULATION TEST")
    print("=" * 80)
    
    # Initialize components using singleton pattern
    reth = pyreth.PyReth()
    processor = reth.tx_processor()
    simulator = reth.simulator()
    
    # Test Case 1: The liquidity removal transaction
    tx_hash = "0xb20e91c60b35647725b1878b60e2ccf6543fc17983983227656cf98bebb22966"
    
    print(f"\n1. Processing original transaction: {tx_hash}")
    print("-" * 40)
    
    try:
        # Get the original processed transaction
        original_tx = processor.process_transaction(tx_hash)
        
        print(f"Original transaction:")
        print(f"  Block: {original_tx.block_number}")
        print(f"  Status: {original_tx.status}")
        print(f"  Type: {original_tx.tx_type}")
        fees = original_tx.fees
        print(f"  Gas Used: {fees['gas_used']}")
        print(f"  ERC20 Transfers: {len(original_tx.erc20_transfers)}")
        print(f"  Internal Txs: {len(original_tx.internal_transactions)}")
        
        # Re-simulate at the same block
        print(f"\n2. Re-simulating at original block {original_tx.block_number}")
        print("-" * 40)
        
        resimulated_tx = resimulate_transaction(original_tx, simulator)
        
        print(f"Re-simulated transaction:")
        print(f"  Block: {resimulated_tx.block_number}")
        print(f"  Status: {resimulated_tx.status}")
        print(f"  Type: {resimulated_tx.tx_type}")
        resim_fees = resimulated_tx.fees
        print(f"  Gas Used: {resim_fees['gas_used']}")
        print(f"  ERC20 Transfers: {len(resimulated_tx.erc20_transfers)}")
        print(f"  Internal Txs: {len(resimulated_tx.internal_transactions)}")
        
        # Compare results
        print(f"\n3. Comparison")
        print("-" * 40)
        
        gas_match = fees['gas_used'] == resim_fees['gas_used']
        print(f"  Gas Used Match: {gas_match} ({fees['gas_used']} vs {resim_fees['gas_used']})")
        
        erc20_match = len(original_tx.erc20_transfers) == len(resimulated_tx.erc20_transfers)
        print(f"  ERC20 Count Match: {erc20_match} ({len(original_tx.erc20_transfers)} vs {len(resimulated_tx.erc20_transfers)})")
        
        internal_match = len(original_tx.internal_transactions) == len(resimulated_tx.internal_transactions)
        print(f"  Internal Tx Match: {internal_match} ({len(original_tx.internal_transactions)} vs {len(resimulated_tx.internal_transactions)})")
        
        # Try simulating at a different block (10 blocks later)
        print(f"\n4. Re-simulating at different block")
        print("-" * 40)
        
        different_block = original_tx.block_number + 10
        print(f"Simulating at block {different_block} (10 blocks after original)")
        
        try:
            later_sim = resimulate_transaction(original_tx, simulator, different_block)
            print(f"  Success! Gas used: {later_sim.fees['gas_used']}")
        except Exception as e:
            print(f"  Failed: {e}")
        
    except Exception as e:
        print(f"Error processing transaction: {e}")
    
    # Test Case 2: The transaction with LackOfFundForMaxFee error
    print("\n" + "=" * 80)
    print("5. Testing problematic transaction")
    print("-" * 40)
    
    problem_tx_hash = "0x6e115e08f7832ba8291e507c3b44b529163ef1302cba8aa39a433ba700e2b214"
    
    try:
        problem_tx = processor.process_transaction(problem_tx_hash)
        print(f"Original block: {problem_tx.block_number}")
        
        # Try to simulate at current state (will likely fail due to lack of funds)
        print("Attempting to simulate at latest block...")
        try:
            # Get latest block using chain_query
            chain_query = reth.chain_query()
            latest_block = chain_query.get_latest_block()
            current_sim = resimulate_transaction(problem_tx, simulator, latest_block)
            print(f"  Unexpected success at block {latest_block}")
        except Exception as e:
            print(f"  Expected failure: {str(e)[:100]}...")
            
    except Exception as e:
        print(f"Error: {e}")
    
    print("\n" + "=" * 80)
    print("TEST COMPLETE")
    print("=" * 80)


if __name__ == "__main__":
    test_resimulation()