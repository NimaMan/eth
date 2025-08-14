#!/usr/bin/env python3
"""
Test batch processing performance with parallel execution
"""

import rs_tx_processor
import time

# Test transactions (mix of valid and complex)
test_hashes = [
    "0x6a904d36e7f808fb08f7dcd04d1b2132a34ca6697b910a93013117d97fe98dd7",  # Complex swap
    "0x5fde29bd2c13db39f758031e2ed054a9d703e3edf04a4d5efa55191c8b71e589",  # Another tx
    "0xc9322b57c24f763d92ac58d414557b61e351f874284fa80e33a1fe73c8b9a5bf",  # Another tx
    "0x82926c3447d7ce6cffbb69ac7ab1571a117e110661532f144ac5bd11053f8d60",  # Another tx
    "0xb1def6eafe5a48b715b5ed210ab4d8a24e40655934aff2180643bd31e42d3c4f",  # Another tx
]

def test_batch_processing():
    """Test different batch processing methods"""
    
    # Initialize processor (no reth_datadir needed - it's hardcoded)
    processor = rs_tx_processor.TxProcessor()
    
    print("🚀 Testing Batch Processing Performance")
    print("=" * 60)
    print(f"Processing {len(test_hashes)} transactions")
    print()
    
    # Test 1: Sequential processing (single transactions)
    print("1️⃣ Sequential Processing (one by one):")
    start_time = time.time()
    sequential_results = []
    for hash in test_hashes:
        try:
            tx = processor.process_transaction(hash)
            sequential_results.append(tx)
        except Exception as e:
            print(f"   Error processing {hash[:10]}...: {e}")
    sequential_time = time.time() - start_time
    print(f"   ✅ Processed {len(sequential_results)} transactions")
    print(f"   ⏱️ Time: {sequential_time:.2f} seconds")
    print(f"   📊 Rate: {len(sequential_results)/sequential_time:.1f} tx/sec")
    print()
    
    # Test 2: Batch processing (default parallel)
    print("2️⃣ Batch Processing (parallel by default):")
    start_time = time.time()
    batch_results = processor.process_transactions_batch(test_hashes)
    batch_time = time.time() - start_time
    print(f"   ✅ Processed {len(batch_results)} transactions")
    print(f"   ⏱️ Time: {batch_time:.2f} seconds")
    print(f"   📊 Rate: {len(batch_results)/batch_time:.1f} tx/sec")
    print(f"   🚀 Speedup: {sequential_time/batch_time:.1f}x faster")
    print()
    
    # Test 3: Detailed batch processing with error handling
    print("3️⃣ Detailed Batch Processing (with error tracking):")
    start_time = time.time()
    detailed_results = processor.process_transactions_detailed(
        test_hashes,
        parallel=True,
        max_workers=4  # Limit to 4 workers
    )
    detailed_time = time.time() - start_time
    print(f"   ✅ Successful: {detailed_results['successful']} transactions")
    print(f"   ❌ Failed: {detailed_results['failed_count']} transactions")
    print(f"   ⏱️ Time: {detailed_time:.2f} seconds")
    print(f"   📊 Rate: {detailed_results['successful']/detailed_time:.1f} tx/sec")
    
    # Show any errors
    if detailed_results['failed_count'] > 0:
        print("\n   Failed transactions:")
        for failed in detailed_results['failed']:
            print(f"     - {failed['hash'][:10]}...: {failed['error']}")
    print()
    
    # Test 4: Compare parallel vs sequential in detailed mode
    print("4️⃣ Detailed Sequential Processing (for comparison):")
    start_time = time.time()
    sequential_detailed = processor.process_transactions_detailed(
        test_hashes,
        parallel=False  # Force sequential
    )
    seq_detailed_time = time.time() - start_time
    print(f"   ✅ Successful: {sequential_detailed['successful']} transactions")
    print(f"   ⏱️ Time: {seq_detailed_time:.2f} seconds")
    print(f"   📊 Rate: {sequential_detailed['successful']/seq_detailed_time:.1f} tx/sec")
    print(f"   🚀 Parallel speedup: {seq_detailed_time/detailed_time:.1f}x faster")
    print()
    
    # Summary
    print("📈 Performance Summary:")
    print("=" * 60)
    print(f"Sequential (one-by-one):  {sequential_time:.2f}s ({len(sequential_results)/sequential_time:.1f} tx/s)")
    print(f"Batch (parallel):         {batch_time:.2f}s ({len(batch_results)/batch_time:.1f} tx/s)")
    print(f"Detailed (parallel):      {detailed_time:.2f}s ({detailed_results['successful']/detailed_time:.1f} tx/s)")
    print(f"Detailed (sequential):    {seq_detailed_time:.2f}s ({sequential_detailed['successful']/seq_detailed_time:.1f} tx/s)")
    print()
    print(f"🚀 Best speedup: {max(sequential_time/batch_time, seq_detailed_time/detailed_time):.1f}x")
    
    # Verify results are consistent
    print("\n🔍 Verifying result consistency:")
    if len(sequential_results) == len(batch_results):
        # Check first transaction matches
        if sequential_results and batch_results:
            seq_first = sequential_results[0]
            batch_first = batch_results[0]
            if seq_first.hash == batch_first.hash:
                print("   ✅ Results match between sequential and batch processing")
            else:
                print("   ⚠️ Results might be in different order")
    
    return batch_results

if __name__ == "__main__":
    results = test_batch_processing()
    
    # Show sample transaction details
    if results:
        print("\n📋 Sample Transaction Details:")
        print("=" * 60)
        tx = results[0]
        print(f"Hash: {tx.hash}")
        print(f"Block: {tx.block_number}")
        print(f"Type: {tx.txn_type}")
        print(f"ERC20 Transfers: {len(tx.erc20_transfers)}")
        print(f"Internal Txs: {len(tx.internal_transactions)}")
        print(f"State Changes: {len(tx.state_changes)} addresses")