#!/usr/bin/env python3
"""
Test basic import and functionality of tx_processor_py module
"""

import sys

# Test import
try:
    import rs_tx_processor
    print("✅ Successfully imported rs_tx_processor")
except ImportError as e:
    print(f"❌ Failed to import: {e}")
    sys.exit(1)

# Test initialization
try:
    processor = rs_tx_processor.TxProcessor()
    print(f"✅ Successfully initialized: {processor}")
except Exception as e:
    print(f"❌ Failed to initialize: {e}")
    sys.exit(1)

# Test stats
try:
    stats = processor.get_stats()
    print(f"✅ Stats retrieved: {stats}")
except Exception as e:
    print(f"❌ Failed to get stats: {e}")
    sys.exit(1)

# Test processing a transaction
try:
    tx_hash = "0x6a904d36e7f808fb08f7dcd04d1b2132a34ca6697b910a93013117d97fe98dd7"
    print(f"\n🔍 Processing transaction: {tx_hash[:10]}...")
    
    tx = processor.process_transaction(tx_hash)
    print(f"✅ Transaction processed successfully")
    print(f"   Type: {type(tx)}")
    print(f"   Repr: {tx}")
    print(f"   Hash: {tx.hash}")
    print(f"   Block: {tx.block_number}")
    print(f"   Status: {tx.status}")
    
    # Test accessing complex fields
    print(f"\n📊 Testing complex field access:")
    print(f"   ERC20 transfers: {len(tx.erc20_transfers)}")
    print(f"   Internal transactions: {len(tx.internal_transactions)}")
    print(f"   Unique addresses: {len(tx.unique_addresses)}")
    print(f"   Fees: {tx.fees}")
    
    # Test to_dict conversion
    tx_dict = tx.to_dict()
    print(f"\n📋 Dictionary conversion:")
    print(f"   Keys: {list(tx_dict.keys())}")
    
except Exception as e:
    print(f"❌ Failed to process transaction: {e}")
    import traceback
    traceback.print_exc()
    sys.exit(1)

print("\n✅ All tests passed!")