#!/usr/bin/env python3
"""
Test error handling in tx_processor_py module
"""

import sys
import tx_processor_py

print("🧪 Testing Error Handling")
print("=" * 40)

# Test 1: Invalid Reth data directory
print("\n1. Testing invalid data directory:")
try:
    processor = tx_processor_py.TxProcessor("/invalid/path/to/reth")
    print("❌ Should have failed with invalid path")
except Exception as e:
    print(f"✅ Correctly raised error: {type(e).__name__}")
    print(f"   Message: {str(e)[:100]}...")

# Test 2: Valid processor
processor = tx_processor_py.TxProcessor("/home/nima/.local/share/reth/mainnet")
print("\n2. Testing invalid transaction hash:")

# Test 2a: Malformed hash
try:
    tx = processor.process_transaction("not-a-valid-hash")
    print("❌ Should have failed with invalid hash")
except Exception as e:
    print(f"✅ Correctly raised error: {type(e).__name__}")
    print(f"   Message: {str(e)}")

# Test 2b: Non-existent transaction
print("\n3. Testing non-existent transaction:")
try:
    # Hash that doesn't exist
    tx = processor.process_transaction("0x0000000000000000000000000000000000000000000000000000000000000000")
    print("❌ Should have failed with non-existent tx")
except Exception as e:
    print(f"✅ Correctly raised error: {type(e).__name__}")
    print(f"   Message: {str(e)[:100]}...")

# Test 3: Batch processing with mixed valid/invalid
print("\n4. Testing batch with invalid hashes:")
tx_hashes = [
    "0x6a904d36e7f808fb08f7dcd04d1b2132a34ca6697b910a93013117d97fe98dd7",  # Valid
    "invalid-hash",  # Invalid format
    "0x0000000000000000000000000000000000000000000000000000000000000000",  # Non-existent
]

try:
    results = processor.process_transactions_batch(tx_hashes)
    print("❌ Should have failed with invalid hash in batch")
except Exception as e:
    print(f"✅ Correctly raised error: {type(e).__name__}")
    print(f"   Message: {str(e)}")

# Test 4: Empty batch
print("\n5. Testing empty batch:")
try:
    results = processor.process_transactions_batch([])
    print(f"✅ Empty batch handled: {len(results)} results")
except Exception as e:
    print(f"❌ Unexpected error: {e}")

# Test 5: Address transactions (placeholder functionality)
print("\n6. Testing address transactions (placeholder):")
try:
    results = processor.process_address_transactions(
        "0xc04b517e75907965ad59976c63912c8c8af97d96",
        start_block=22893000,
        end_block=22894000,
        limit=10
    )
    print(f"✅ Address query handled: {len(results)} results (expected 0 for placeholder)")
except Exception as e:
    print(f"❌ Unexpected error: {e}")

# Test 6: Invalid address format
print("\n7. Testing invalid address format:")
try:
    results = processor.process_address_transactions(
        "not-an-address",
        start_block=22893000,
        end_block=22894000
    )
    print("❌ Should have failed with invalid address")
except Exception as e:
    print(f"✅ Correctly raised error: {type(e).__name__}")
    print(f"   Message: {str(e)}")

print("\n✅ Error handling tests complete!")