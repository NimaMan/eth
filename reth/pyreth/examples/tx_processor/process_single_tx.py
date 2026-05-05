#!/usr/bin/env python3
"""
Example: Process a single transaction using PyReth tx_processor
"""

from pyreth import tx_processor as pyreth_tx_processor

def main():
    # Create PyReth instance with shared database
    
    # Get tx processor that shares the database connection
    processor = pyreth_tx_processor()
    
    print("=== Transaction Processor: Single Transaction ===\n")
    
    # Example transaction hash (a USDC transfer)
    tx_hash = "0x6a85c721e474ad9774c5c30c6d6f4169e152005cc5d2b1e5503446d65d5f9bb8"
    
    try:
        # Process transaction with simulation for balance changes
        print(f"Processing transaction: {tx_hash}")
        tx = processor.process_transaction_from_hash_with_simulation(tx_hash)
        
        # Display transaction details
        print(f"\nTransaction Details:")
        print(f"  Block: {tx.block_number}")
        print(f"  From: {tx.from_address}")
        print(f"  To: {tx.to_address}")
        print(f"  Value: {tx.value} wei")
        print(f"  Type: {tx.tx_type}")
        print(f"  Status: {'Success' if bool(tx.status) else 'Failed'}")
        
        # Display events
        if tx.erc20_transfers:
            print(f"\nERC20 Transfers ({len(tx.erc20_transfers)}):")
            for transfer in tx.erc20_transfers[:3]:  # Show first 3
                print(f"  {transfer.token_address}: {transfer.from_address} -> {transfer.to_address}")
                print(f"    Amount: {transfer.amount}")
        
        if tx.uniswap_v2_swaps:
            print(f"\nUniswap V2 Swaps ({len(tx.uniswap_v2_swaps)}):")
            for swap in tx.uniswap_v2_swaps[:2]:
                print(f"  Pair: {swap.pair_address}")
                print(f"  Amounts: {swap.amount0_in}/{swap.amount1_in} -> {swap.amount0_out}/{swap.amount1_out}")
        
        if tx.uniswap_v3_swaps:
            print(f"\nUniswap V3 Swaps ({len(tx.uniswap_v3_swaps)}):")
            for swap in tx.uniswap_v3_swaps[:2]:
                print(f"  Pool: {swap.pool_address}")
                print(f"  Amount0: {swap.amount0}, Amount1: {swap.amount1}")
        
        # Display balance changes
        if tx.address_balance_changes:
            print(f"\nAddress Balance Changes:")
            for addr, changes in list(tx.address_balance_changes.items())[:5]:
                print(f"  {addr}: {changes}")
        
        # Display internal transactions
        if tx.internal_transactions:
            print(f"\nInternal Transactions ({len(tx.internal_transactions)}):")
            for internal in tx.internal_transactions[:3]:
                print(f"  {internal.from_address} -> {internal.to_address}")
                print(f"    Value: {internal.value} wei, Type: {internal.type}")
        
    except Exception as e:
        print(f"Error processing transaction: {e}")
    
    # Process without simulation (faster, no balance changes)
    print("\n" + "="*50)
    print("Processing without simulation (DB only):")
    
    try:
        tx_fast = processor.load_transaction_from_hash_db_only(tx_hash)
        print(f"  Loaded transaction type: {tx_fast.tx_type}")
        print(f"  Events decoded: ERC20={len(tx_fast.erc20_transfers)}, ERC721={len(tx_fast.erc721_transfers)}")
        print(f"  Balance changes available: {bool(tx_fast.address_balance_changes)}")
    except Exception as e:
        print(f"Error loading transaction: {e}")
    
    print("\n✅ Transaction processing complete!")

if __name__ == "__main__":
    main()
