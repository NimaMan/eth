#!/usr/bin/env python3
"""
Simple PyReth example - Process transaction and show address balance changes
"""

import pyreth

def process_transaction_with_pyreth(tx_hash: str):
    """Process a transaction with PyReth and show results"""
    
    # Initialize PyReth (singleton pattern)
    reth = pyreth.PyReth()
    processor = reth.tx_processor()
    
    # Process transaction with simulation for balance changes
    tx = processor.process_transaction_from_hash_with_simulation(tx_hash)
    
    # Show basic info
    print(f"Transaction: {tx.hash}")
    print(f"Block: {tx.block_number}")
    print(f"Type: {tx.tx_type}")
    print(f"From: {tx.from_address}")
    print(f"To: {tx.to_address}")
    print(f"Value: {tx.value}")
    print(f"ERC20 transfers: {len(tx.erc20_transfers)}")
    print(f"Internal transactions: {len(tx.internal_transactions)}")
    
    # Show address balance changes
    balance_changes = tx.address_balance_changes
    print(f"\nAddress Balance Changes ({len(balance_changes)} addresses):")
    
    for address, changes in balance_changes.items():
        print(f"  {address}:")
        print(f"    Changes: {changes}")
    
    return tx

if __name__ == "__main__":
    # Test with KERMIT swap transaction
    tx_hash = "0x6c85e9d68a79ad31ec397d50e53f7736f2e4e02d3594d5165fe0a4b534cc0c4d"
    
    print("PyReth Transaction Processing Example")
    print("=" * 50)
    
    tx = process_transaction_with_pyreth(tx_hash)