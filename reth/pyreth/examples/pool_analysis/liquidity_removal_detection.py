#!/usr/bin/env python3
"""
Liquidity Removal Detection Example

This script demonstrates how to detect liquidity removal from ERC20 pools,
including investigating transactions with various errors and undetected scams.
"""

import pyreth


def investigate_lackoffundfee_error():
    """Investigate a transaction with LackOfFundForMaxFee error"""
    
    # Transaction details
    tx_hash = "0x6e115e08f7832ba8291e507c3b44b529163ef1302cba8aa39a433ba700e2b214"
    account = "0xcC228F9F1428314acc460e7Da110C64415A2F05a"
    
    print("\n1. INVESTIGATING TX WITH LACKOFFUNDFEE ERROR")
    print("-" * 40)
    print(f"TX Hash: {tx_hash}")
    print(f"Account: {account}")
    
    # Create PyReth instance
    pyreth_client = pyreth.PyReth()
    processor = pyreth_client.tx_processor()
    simulator = pyreth_client.simulator()
    chain_query = pyreth_client.chain_query()
    
    try:
        print("\nProcessing actual transaction...")
        tx_processed = processor.process_transaction(tx_hash)
        print(f"  Block: {tx_processed.block_number}")
        print(f"  From: {tx_processed.from_address}")
        print(f"  To: {tx_processed.to_address}")
        print(f"  Value: {tx_processed.value}")
        
        # Get fees information
        fees = tx_processed.fees
        print(f"  Gas Used: {fees['gas_used']}")
        print(f"  Gas Price: {fees['gas_price']}")
        print(f"  Status: {tx_processed.status}")
        
        # Check balance at transaction block
        print(f"\nChecking account balance at block {tx_processed.block_number}...")
        balance_at_tx = chain_query.get_balance(account, tx_processed.block_number)
        print(f"  Balance at TX block: {balance_at_tx} wei")
        
        # Check balance at current state
        print("\nChecking account balance at current state...")
        current_balance = chain_query.get_balance(account)
        print(f"  Current balance: {current_balance} wei")
        
        # Try to simulate at the transaction's block
        print(f"\nSimulating transaction at its original block {tx_processed.block_number}...")
        try:
            # Build transaction for simulation
            sim_tx = {
                "from": tx_processed.from_address,
                "to": tx_processed.to_address,
                "value": str(tx_processed.value),
                "data": tx_processed.input,
                "gas": fees['gas_used'] * 2,  # Use a reasonable gas limit
            }
            
            sim_result = simulator.simulate_transaction(sim_tx, tx_processed.block_number)
            print(f"  Simulation successful!")
            sim_fees = sim_result.fees
            print(f"  Gas used: {sim_fees['gas_used']}")
            print(f"  Status: {sim_result.status}")
        except Exception as e:
            print(f"  Simulation failed: {e}")
            
    except Exception as e:
        print(f"  Error processing transaction: {e}")


def investigate_undetected_scam():
    """Investigate an undetected scam transaction with liquidity removal"""
    
    # Transaction details
    tx_hash = "0xb20e91c60b35647725b1878b60e2ccf6543fc17983983227656cf98bebb22966"
    pool = "0xd5113065d0dA0CD94F8c0Ba7B2Fa61d8A48AE404"
    creator = "0x7D804D810147e0297b97f11b6f017147DFaE2fc2"
    
    print("\n\n2. INVESTIGATING UNDETECTED SCAM TRANSACTION")
    print("-" * 40)
    print(f"TX Hash: {tx_hash}")
    print(f"Pool: {pool}")
    print(f"Creator: {creator}")
    
    # Create PyReth instance
    pyreth_client = pyreth.PyReth()
    processor = pyreth_client.tx_processor()
    chain_query = pyreth_client.chain_query()
    
    try:
        print("\nProcessing actual transaction...")
        tx_processed = processor.process_transaction(tx_hash)
        print(f"  Block: {tx_processed.block_number}")
        print(f"  From: {tx_processed.from_address}")
        print(f"  To: {tx_processed.to_address}")
        fees = tx_processed.fees
        print(f"  Gas Used: {fees['gas_used']}")
        print(f"  Status: {tx_processed.status}")
        
        # Analyze events
        print(f"\nTransaction Events:")
        print(f"  ERC20 Transfers: {len(tx_processed.erc20_transfers)}")
        print(f"  Uniswap V2 Events: {len(tx_processed.uniswap_v2_swaps)}")
        print(f"  Internal Transactions: {len(tx_processed.internal_transactions)}")
        
        # Look for liquidity removal
        for event in tx_processed.uniswap_v2_swaps:
            if event.get('type') == 'burn':
                print(f"\n  LIQUIDITY REMOVAL DETECTED:")
                print(f"    Pool: {event.get('pair_address')}")
                print(f"    Amount0: {event.get('amount0')}")
                print(f"    Amount1: {event.get('amount1')}")
        
        # Check pool state before and after
        print(f"\nChecking pool ETH balance...")
        print(f"  At TX block {tx_processed.block_number}:")
        pool_balance_at_tx = chain_query.get_balance(pool, tx_processed.block_number)
        print(f"    Pool balance: {int(pool_balance_at_tx) / 10**18:.6f} ETH")
        
        # Check one block after
        pool_balance_after = chain_query.get_balance(pool, tx_processed.block_number + 1)
        print(f"  After TX (block {tx_processed.block_number + 1}):")
        print(f"    Pool balance: {int(pool_balance_after) / 10**18:.6f} ETH")
        
        balance_change = (int(pool_balance_at_tx) - int(pool_balance_after)) / 10**18
        print(f"  Balance removed: {balance_change:.6f} ETH")
        
    except Exception as e:
        print(f"  Error processing transaction: {e}")


def test_trading_simulation():
    """Test trading simulation with a token"""
    
    print("\n\n3. TESTING TRADING SIMULATION")
    print("-" * 40)
    
    # Create PyReth instance
    pyreth_client = pyreth.PyReth()
    processor = pyreth_client.tx_processor()
    trading_sim = pyreth_client.trading_simulator()
    
    try:
        # Process a transaction to use as prior_tx
        print("Processing enable trading transaction...")
        tx_processed = processor.process_transaction(
            "0x57acab49778bb8c38efb5261e2b3473feca6c018856f0557f9a3f4d976355745"
        )
        print(f"  Processed tx from: {tx_processed.from_address}")
        
        # Simulate trading sequence
        print("\nSimulating trading sequence...")
        print("  Token: PEPE (0x6982508145454Ce325dDbE47a25d4ec3d2311933)")
        print("  Pool: PEPE/WETH (0xa43fe16908251ee70ef74718545e4fe6c5ccec9f)")
        
        result = trading_sim.simulate_tx_with_buy_sell_seq(
            prior_tx=tx_processed,  # Optional ProcessedTransaction
            token_address="0x6982508145454Ce325dDbE47a25d4ec3d2311933",  # PEPE
            pool_address="0xa43fe16908251ee70ef74718545e4fe6c5ccec9f",   # PEPE/WETH
            block_number=None  # Use latest block
        )
        
        # Access results
        print(f"\nResults:")
        print(f"  Trading enabled: {result.trading_enabled}")
        print(f"  Buy tax: {result.buy_tax}%")
        print(f"  Sell tax: {result.sell_tax}%")
        
        if result.buy_tx:
            print(f"  Buy TX status: {result.buy_tx.status}")
        if result.sell_tx:
            print(f"  Sell TX status: {result.sell_tx.status}")
            
    except Exception as e:
        print(f"  Trading simulation failed: {e}")
        if "LackOfFundForMaxFee" in str(e):
            print("  Note: This error indicates insufficient funds for gas fees")


def main():
    """Run all investigations"""
    
    print("=" * 80)
    print("LIQUIDITY REMOVAL DETECTION EXAMPLES")
    print("=" * 80)
    
    # Get latest block for reference
    try:
        pyreth_client = pyreth.PyReth()
        simulator = pyreth_client.simulator()
        latest_block = simulator.get_latest_block()
        print(f"\nLatest block in database: {latest_block}")
    except Exception as e:
        print(f"\nCould not get latest block: {e}")
    
    # Run investigations
    investigate_lackoffundfee_error()
    investigate_undetected_scam()
    test_trading_simulation()
    
    print("\n" + "=" * 80)
    print("INVESTIGATION COMPLETE")
    print("=" * 80)


if __name__ == "__main__":
    main()