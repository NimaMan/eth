#!/usr/bin/env python3
"""
Python validation script for transaction dea3844306bbf1712d9284dadd8ee1e89303f2446efe7438134c2e74346a8be4
Generated automatically by Rust comprehensive state diff test
"""

import sys
import os
sys.path.append('/home/nima/code/crypto/py/eth_block_processor')

from web3 import Web3
from eth_block_processor.txn.txn_data_fetcher import TransactionDataFetcher
from eth_block_processor.txn.txn_processor import TransactionProcessor

def validate_transaction():
    # Setup
    w3 = Web3(Web3.HTTPProvider('http://localhost:8545'))
    txn_data_fetcher = TransactionDataFetcher(w3)
    txn_processor = TransactionProcessor(w3, calculate_state_changes=True)
    
    # Fetch transaction data
    tx_hash = 'dea3844306bbf1712d9284dadd8ee1e89303f2446efe7438134c2e74346a8be4'
    print(f"🐍 Processing transaction: {tx_hash}")
    
    try:
        txn_data = txn_data_fetcher.get_transaction_data(tx_hash)
        if not txn_data:
            print(f"❌ Transaction {tx_hash} not found")
            return False
            
        # Process transaction
        processed_tx = txn_processor.process_transaction(
            txn_data['transaction'], 
            txn_data['receipt'], 
            txn_data['trace']
        )
        
        # Print state changes in format compatible with Rust test
        state_changes = processed_tx.state_changes
        print(f"✅ Found state changes for {len(state_changes)} addresses")
        
        print("📊 PYTHON STATE CHANGES:")
        print("=" * 50)
        for address, changes in state_changes.items():
            print(f"Address: {address}")
            print(f"  Token Net: {changes['token_net']:.6f}")
            print(f"  Denom Net: {changes['denom_net']:.6f}")
            print(f"  Movements: {changes['movements']}")
            print()
        
        # Generate JSON for Rust comparison
        import json
        json_output = json.dumps(state_changes, indent=2, default=str)
        print("🔄 JSON OUTPUT FOR RUST COMPARISON:")
        print("=" * 50)
        print(json_output)
        
        return True
        
    except Exception as e:
        print(f"❌ Error processing transaction: {e}")
        import traceback
        traceback.print_exc()
        return False

if __name__ == "__main__":
    success = validate_transaction()
    sys.exit(0 if success else 1)
