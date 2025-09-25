#!/usr/bin/env python3
"""
Example: Process multiple transactions from a list of hashes
"""

import pyreth
import time

def main():
    # Create PyReth instance with shared database
    reth = pyreth.PyReth()
    
    # Get tx processor
    processor = reth.tx_processor()
    
    print("=== Transaction Processor: Batch Processing ===\n")
    
    # List of transaction hashes to process
    tx_hashes = [
        "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
        "0x2345678901bcdef02345678901bcdef02345678901bcdef02345678901bcdef0",
        "0x3456789012cdef013456789012cdef013456789012cdef013456789012cdef01",
        "0x4567890123def0124567890123def0124567890123def0124567890123def012",
        "0x567890abcdef0123567890abcdef0123567890abcdef0123567890abcdef0123",
    ]
    
    print(f"Processing {len(tx_hashes)} transactions...\n")
    
    # Process transactions in batch (parallel processing)
    start_time = time.time()
    
    try:
        # Use the new method name: process_transaction_hash_list
        results = processor.process_transaction_hash_list(tx_hashes)
        elapsed = time.time() - start_time
        
        print(f"✅ Processed {len(results)} transactions in {elapsed:.2f} seconds")
        print(f"   Average: {elapsed/len(results):.3f} seconds per transaction")
        print(f"   Throughput: {len(results)/elapsed:.1f} tx/sec\n")
        
        # Display summary of results
        print("Transaction Summary:")
        print("-" * 50)
        
        for i, tx in enumerate(results):
            print(f"\n{i+1}. Hash: {tx.hash[:10]}...")
            print(f"   Block: {tx.block_number}")
            print(f"   Type: {tx.txn_type}")
            print(f"   From: {tx.from_address}")
            print(f"   To: {tx.to_address if tx.to_address else 'Contract Creation'}")
            print(f"   Status: {'Success' if tx.status == 'True' else 'Failed'}")
            
            # Count events
            event_counts = []
            if tx.erc20_transfers:
                event_counts.append(f"ERC20:{len(tx.erc20_transfers)}")
            if tx.erc721_transfers:
                event_counts.append(f"ERC721:{len(tx.erc721_transfers)}")
            if tx.uniswap_v2_swaps:
                event_counts.append(f"V2Swaps:{len(tx.uniswap_v2_swaps)}")
            if tx.uniswap_v3_swaps:
                event_counts.append(f"V3Swaps:{len(tx.uniswap_v3_swaps)}")
            
            if event_counts:
                print(f"   Events: {', '.join(event_counts)}")
            
            if tx.internal_transactions:
                print(f"   Internal Txs: {len(tx.internal_transactions)}")
        
    except Exception as e:
        print(f"❌ Error processing transactions: {e}")
    
    # Compare with sequential processing
    print("\n" + "="*50)
    print("Comparing batch vs sequential processing:\n")
    
    # Take just 2 transactions for comparison
    test_hashes = tx_hashes[:2]
    
    # Sequential processing
    start_time = time.time()
    sequential_results = []
    for hash in test_hashes:
        try:
            tx = processor.process_transaction_from_hash_with_simulation(hash)
            sequential_results.append(tx)
        except:
            pass
    sequential_time = time.time() - start_time
    
    # Batch processing
    start_time = time.time()
    try:
        batch_results = processor.process_transaction_hash_list(test_hashes)
        batch_time = time.time() - start_time
    except:
        batch_time = 0
        batch_results = []
    
    print(f"Sequential ({len(test_hashes)} txs): {sequential_time:.3f} seconds")
    print(f"Batch ({len(test_hashes)} txs): {batch_time:.3f} seconds")
    
    if batch_time > 0:
        speedup = sequential_time / batch_time
        print(f"Speedup: {speedup:.1f}x faster with batch processing")
    
    print("\n✅ Batch processing example complete!")

if __name__ == "__main__":
    main()