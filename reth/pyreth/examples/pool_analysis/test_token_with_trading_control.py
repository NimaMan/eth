#!/usr/bin/env python3
"""
Test InfinityHub Trading Simulation

This example demonstrates using real transactions from InfinityHub token to:
1. Process the liquidity addition transaction
2. Process the liquidity removal transaction  
3. Simulate buy/sell sequences to calculate taxes

Transaction details:
- Liquidity Add: 0x57acab49778bb8c38efb5261e2b3473feca6c018856f0557f9a3f4d976355745
- Liquidity Remove: 0xc59e835c4fbfc53c66f02ac1354478232ced0c7fb560cd2a9564532b12d8ad2c
- Token: InfinityHub (♾️INHB)
- Pool: Uniswap V2 INHB/WETH
"""

import sys
import os
sys.path.insert(0, os.path.dirname(os.path.dirname(__file__)))

import pyreth

def main():
    print("InfinityHub Trading Simulation Example")
    print("=" * 60)
    
    # Create PyReth instance
    pyreth_client = pyreth.PyReth()
    print("✓ Connected to PyReth")
    
    # Get processors
    processor = pyreth_client.tx_processor()
    trading_sim = pyreth_client.trading_simulator()
    
    # Transaction hashes
    liquidity_add_hash = "0x57acab49778bb8c38efb5261e2b3473feca6c018856f0557f9a3f4d976355745"
    liquidity_remove_hash = "0xc59e835c4fbfc53c66f02ac1354478232ced0c7fb560cd2a9564532b12d8ad2c"
    
    # InfinityHub token and pool addresses (extracted from transactions)
    token_address = "0xf068C6d0390797e622b668531789adb31fF2A3c0"  # InfinityHub (♾️INHB)
    pool_address = "0xf068C6d0390797e622b668531789adb31fF2A3c0"   # Uniswap V2: ♾️INHB Pool
    
    print(f"\nProcessing transactions...")
    print(f"Token: InfinityHub (♾️INHB)")
    print(f"Token Address: {token_address}")
    print(f"Pool Address: {pool_address}")
    
    # Process the liquidity addition transaction
    print(f"\n1. Processing liquidity addition tx...")
    print(f"   Hash: {liquidity_add_hash[:10]}...")
    try:
        tx1_processed = processor.process_transaction(liquidity_add_hash)
        print(f"   ✓ Block: {tx1_processed.block_number}")
        print(f"   ✓ Status: {'Success' if tx1_processed.status == '1' else 'Failed'}")
        print(f"   ✓ From: {tx1_processed.from_address}")
        print(f"   ✓ To: {tx1_processed.to_address}")
    except Exception as e:
        print(f"   ✗ Error processing tx1: {e}")
        tx1_processed = None
    
    # Process the liquidity removal transaction
    print(f"\n2. Processing liquidity removal tx...")
    print(f"   Hash: {liquidity_remove_hash[:10]}...")
    try:
        tx2_processed = processor.process_transaction(liquidity_remove_hash)
        print(f"   ✓ Block: {tx2_processed.block_number}")
        print(f"   ✓ Status: {'Success' if tx2_processed.status == '1' else 'Failed'}")
        print(f"   ✓ From: {tx2_processed.from_address}")
        print(f"   ✓ To: {tx2_processed.to_address}")
        
        # Show token transfers
        if tx2_processed.erc20_transfers:
            print(f"   ✓ ERC20 Transfers: {len(tx2_processed.erc20_transfers)}")
            for transfer in tx2_processed.erc20_transfers[:2]:  # Show first 2
                print(f"     - {transfer.amount} tokens from {transfer.from_address[:10]}...")
                
    except Exception as e:
        print(f"   ✗ Error processing tx2: {e}")
        tx2_processed = None
    
    # Simulate trading sequence after liquidity addition
    if tx1_processed:
        print(f"\n3. Simulating buy/sell after liquidity addition...")
        try:
            result_after_add = trading_sim.simulate_tx_with_buy_sell_seq(
                prior_tx=tx1_processed,  # Use liquidity add as prior tx
                token_address=token_address,
                pool_address=pool_address,
                block_number=None  # Use latest block
            )
            
            print(f"   ✓ Trading enabled: {result_after_add.trading_enabled}")
            print(f"   ✓ Buy tax: {result_after_add.buy_tax:.2f}%")
            print(f"   ✓ Sell tax: {result_after_add.sell_tax:.2f}%")
            print(f"   ✓ Block: {result_after_add.block_number}")
            
            # Access full ProcessedTransaction objects
            print(f"\n   Transaction Results:")
            if result_after_add.prior_tx:
                print(f"   - Prior TX: {result_after_add.prior_tx.hash[:10]}... (liquidity add)")
            print(f"   - Buy TX: {result_after_add.buy_tx.hash[:10]}... (Status: {result_after_add.buy_tx.status})")
            print(f"   - Approve TX: {result_after_add.approve_tx.hash[:10]}... (Status: {result_after_add.approve_tx.status})")
            print(f"   - Sell TX: {result_after_add.sell_tx.hash[:10]}... (Status: {result_after_add.sell_tx.status})")
            
        except Exception as e:
            print(f"   ✗ Error in simulation: {e}")
    
    # Simulate trading sequence after liquidity removal
    if tx2_processed:
        print(f"\n4. Simulating buy/sell after liquidity removal...")
        try:
            result_after_remove = trading_sim.simulate_tx_with_buy_sell_seq(
                prior_tx=tx2_processed,  # Use liquidity removal as prior tx
                token_address=token_address,
                pool_address=pool_address,
                block_number=None  # Use latest block
            )
            
            print(f"   ✓ Trading enabled: {result_after_remove.trading_enabled}")
            print(f"   ✓ Buy tax: {result_after_remove.buy_tax:.2f}%")
            print(f"   ✓ Sell tax: {result_after_remove.sell_tax:.2f}%")
            print(f"   ✓ Block: {result_after_remove.block_number}")
            
            # Show if taxes changed after liquidity removal
            if tx1_processed and 'result_after_add' in locals():
                print(f"\n   Tax Changes:")
                buy_diff = result_after_remove.buy_tax - result_after_add.buy_tax
                sell_diff = result_after_remove.sell_tax - result_after_add.sell_tax
                print(f"   - Buy tax change: {buy_diff:+.2f}%")
                print(f"   - Sell tax change: {sell_diff:+.2f}%")
                
        except Exception as e:
            print(f"   ✗ Error in simulation: {e}")
    
    # Simulate without any prior transaction (current state)
    print(f"\n5. Simulating buy/sell with current state (no prior tx)...")
    try:
        result_current = trading_sim.simulate_tx_with_buy_sell_seq(
            prior_tx=None,  # No prior transaction
            token_address=token_address,
            pool_address=pool_address,
            block_number=None  # Use latest block
        )
        
        print(f"   ✓ Trading enabled: {result_current.trading_enabled}")
        print(f"   ✓ Buy tax: {result_current.buy_tax:.2f}%")
        print(f"   ✓ Sell tax: {result_current.sell_tax:.2f}%")
        print(f"   ✓ Block: {result_current.block_number}")
        
        # Access buy transaction details
        buy_tx = result_current.buy_tx
        print(f"\n   Buy Transaction Details:")
        print(f"   - Hash: {buy_tx.hash[:10]}...")
        print(f"   - Gas Used: {buy_tx.fees.gas_used}")
        print(f"   - Status: {'Success' if buy_tx.status == '1' else 'Failed'}")
        
        # Access sell transaction details  
        sell_tx = result_current.sell_tx
        print(f"\n   Sell Transaction Details:")
        print(f"   - Hash: {sell_tx.hash[:10]}...")
        print(f"   - Gas Used: {sell_tx.fees.gas_used}")
        print(f"   - Status: {'Success' if sell_tx.status == '1' else 'Failed'}")
        
    except Exception as e:
        print(f"   ✗ Error in current state simulation: {e}")
    
    print("\n" + "=" * 60)
    print("Simulation complete!")

if __name__ == "__main__":
    main()