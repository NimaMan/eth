#!/usr/bin/env python3
"""
Investigate liquidity removal transactions using pyreth

This script investigates two specific transactions:
1. TX with LackOfFundForMaxFee error during simulation
2. Original undetected scam transaction
"""

import pyreth
import json

def investigate_transactions():
    """Investigate the problematic transactions"""
    
    # Initialize pyreth components using shared instance
    reth = pyreth.PyReth()
    processor = reth.tx_processor()
    simulator = reth.simulator()
    chain_query = reth.chain_query()
    
    print("=" * 80)
    print("TRANSACTION INVESTIGATION USING PYRETH")
    print("=" * 80)
    
    # Transaction 1: The one with LackOfFundForMaxFee error
    tx1_hash = "0x6e115e08f7832ba8291e507c3b44b529163ef1302cba8aa39a433ba700e2b214"
    tx1_account = "0xcC228F9F1428314acc460e7Da110C64415A2F05a"
    
    print("\n1. INVESTIGATING TX WITH LACKOFFUNDFEE ERROR")
    print("-" * 40)
    print(f"TX Hash: {tx1_hash}")
    print(f"Account: {tx1_account}")
    
    # Process the actual transaction
    try:
        print("\nProcessing actual transaction...")
        tx1_processed = processor.process_transaction_from_hash_with_simulation(tx1_hash)
        print(f"  Block: {tx1_processed.block_number}")
        print(f"  From: {tx1_processed.from_address}")
        print(f"  To: {tx1_processed.to_address}")
        print(f"  Value: {tx1_processed.value}")
        
        # Get fees information
        fees = tx1_processed.fees  # It's a property, not a method
        print(f"  Gas Used: {fees['gas_used']}")
        print(f"  Gas Price: {fees['gas_price']}")
        print(f"  Status: {tx1_processed.status}")
        
        # Check balance at transaction block
        print(f"\nChecking account balance at block {tx1_processed.block_number}...")
        balance_at_tx = chain_query.get_balance(tx1_account, tx1_processed.block_number)
        print(f"  Balance at TX block: {balance_at_tx} wei")
        
        # Check balance at current state
        print("\nChecking account balance at current state...")
        current_balance = chain_query.get_balance(tx1_account)
        print(f"  Current balance: {current_balance} wei")
        
        # Try to simulate at the transaction's block
        print(f"\nSimulating transaction at its original block {tx1_processed.block_number}...")
        try:
            # Build transaction for simulation
            sim_tx = {
                "from": tx1_processed.from_address,
                "to": tx1_processed.to_address,
                "value": str(tx1_processed.value),
                "data": tx1_processed.input,
                "gas": fees['gas_used'] * 2,  # Use a reasonable gas limit
            }
            
            sim_result = simulator.simulate_transaction(sim_tx, tx1_processed.block_number)
            print(f"  Simulation successful!")
            sim_fees = sim_result.fees
            print(f"  Gas used: {sim_fees['gas_used']}")
            print(f"  Status: {sim_result.status}")
        except Exception as e:
            print(f"  Simulation failed: {e}")
            
    except Exception as e:
        print(f"  Error processing transaction: {e}")
    
    # Transaction 2: The undetected scam
    tx2_hash = "0xb20e91c60b35647725b1878b60e2ccf6543fc17983983227656cf98bebb22966"
    tx2_pool = "0xd5113065d0dA0CD94F8c0Ba7B2Fa61d8A48AE404"
    tx2_creator = "0x7D804D810147e0297b97f11b6f017147DFaE2fc2"
    
    print("\n\n2. INVESTIGATING UNDETECTED SCAM TRANSACTION")
    print("-" * 40)
    print(f"TX Hash: {tx2_hash}")
    print(f"Pool: {tx2_pool}")
    print(f"Creator: {tx2_creator}")
    
    try:
        print("\nProcessing actual transaction...")
        tx2_processed = processor.process_transaction_from_hash_with_simulation(tx2_hash)
        print(f"  Block: {tx2_processed.block_number}")
        print(f"  From: {tx2_processed.from_address}")
        print(f"  To: {tx2_processed.to_address}")
        fees2 = tx2_processed.fees
        print(f"  Gas Used: {fees2['gas_used']}")
        print(f"  Status: {tx2_processed.status}")
        
        # Analyze events
        print(f"\nTransaction Events:")
        print(f"  ERC20 Transfers: {len(tx2_processed.erc20_transfers)}")
        print(f"  Uniswap V2 Events: {len(tx2_processed.uniswap_v2_swaps)}")
        print(f"  Internal Transactions: {len(tx2_processed.internal_transactions)}")
        
        # Look for liquidity removal
        for event in tx2_processed.uniswap_v2_swaps:
            if event.get('type') == 'burn':
                print(f"\n  LIQUIDITY REMOVAL DETECTED:")
                print(f"    Pool: {event.get('pair_address')}")
                print(f"    Amount0: {event.get('amount0')}")
                print(f"    Amount1: {event.get('amount1')}")
        
        # Check pool state before and after
        print(f"\nChecking pool ETH balance...")
        print(f"  At TX block {tx2_processed.block_number}:")
        pool_balance_at_tx = chain_query.get_balance(tx2_pool, tx2_processed.block_number)
        print(f"    Pool balance: {int(pool_balance_at_tx) / 10**18:.6f} ETH")
        
        # Check one block after
        pool_balance_after = chain_query.get_balance(tx2_pool, tx2_processed.block_number + 1)
        print(f"  After TX (block {tx2_processed.block_number + 1}):")
        print(f"    Pool balance: {int(pool_balance_after) / 10**18:.6f} ETH")
        
        balance_change = (int(pool_balance_at_tx) - int(pool_balance_after)) / 10**18
        print(f"  Balance removed: {balance_change:.6f} ETH")
        
    except Exception as e:
        print(f"  Error processing transaction: {e}")
    
    # Get latest block for reference
    try:
        latest_block = simulator.get_latest_block()
        print(f"\n\nLatest block in database: {latest_block}")
    except Exception as e:
        print(f"\n\nCould not get latest block: {e}")
    
    print("\n" + "=" * 80)
    print("INVESTIGATION COMPLETE")
    print("=" * 80)

if __name__ == "__main__":
    investigate_transactions()
