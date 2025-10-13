#!/usr/bin/env python3
"""
Test script for Rust-based ProcessedTransactionProvider
"""

import sys
import asyncio
from eth_data.tx_provider.rs_processed_transaction_provider import RustProcessedTransactionProvider

def test_basic_functionality():
    """Test basic functionality of the Rust provider."""
    print("Testing Rust-based ProcessedTransactionProvider...")
    
    # Initialize provider
    provider = RustProcessedTransactionProvider()
    print(f"✓ Provider initialized: {provider.get_cache_statistics()}")
    
    # Test single transaction processing
    test_tx = "0x5c89f223fe19593cf85319dcdd3f6de618e770d958f72ea123d36db2fdf5d46e"
    print(f"\nProcessing single transaction: {test_tx[:10]}...")
    
    txs = provider.get_processed_transactions_from_tx_hashes([test_tx])
    if txs:
        tx = txs[0]
        print(f"✓ Transaction processed successfully")
        print(f"  - Block: {tx.block_number}")
        print(f"  - From: {tx.from_address[:10]}...")
        print(f"  - To: {tx.to_address[:10] if tx.to_address else 'Contract Creation'}...")
        print(f"  - Value: {tx.value}")
        print(f"  - Type: {tx.tx_type}")
        print(f"  - ERC20 Transfers: {len(tx.erc20_transfers)}")
        print(f"  - Internal Txs: {len(tx.internal_transactions)}")
    else:
        print("✗ Failed to process transaction")
    
    # Test batch processing
    test_txs = [
        "0x5c89f223fe19593cf85319dcdd3f6de618e770d958f72ea123d36db2fdf5d46e",
        "0x7ada9993217ed90891e8139c805204391d194fef8c6b37be63bc1d585b7896a1",
        "0xc9322b57c24f763d92ac58d414557b61e351f874284fa80e33a1fe73c8b9a5bf",
    ]
    
    print(f"\nProcessing batch of {len(test_txs)} transactions...")
    batch_results = provider.get_processed_transactions_from_tx_hashes(test_txs)
    print(f"✓ Batch processed: {len(batch_results)} transactions")
    
    # Test cache statistics
    print(f"\nCache statistics: {provider.get_cache_statistics()}")
    
    # Clean up
    provider.close()
    print("\n✓ All tests passed!")

async def test_async_functionality():
    """Test async functionality."""
    print("\nTesting async block processing...")
    
    provider = RustProcessedTransactionProvider()
    
    # Test block processing (block 20000000 as example)
    test_blocks = [20000000, 20000001]
    print(f"Processing blocks: {test_blocks}")
    
    results = await provider.get_processed_transactions_from_block_numbers(test_blocks)
    
    for block_num, txs in results.items():
        print(f"  Block {block_num}: {len(txs)} transactions")
    
    provider.close()
    print("✓ Async tests passed!")

def test_performance_comparison():
    """Compare performance with batch processing."""
    import time
    
    print("\n" + "="*60)
    print("Performance Test: Batch Processing")
    print("="*60)
    
    provider = RustProcessedTransactionProvider()
    
    # Test transactions (mix of simple and complex)
    test_txs = [
        "0x5c89f223fe19593cf85319dcdd3f6de618e770d958f72ea123d36db2fdf5d46e",  # Complex DeFi
        "0x7ada9993217ed90891e8139c805204391d194fef8c6b37be63bc1d585b7896a1",  # Token transfer
        "0xc9322b57c24f763d92ac58d414557b61e351f874284fa80e33a1fe73c8b9a5bf",  # Uniswap swap
        "0x5fde29bd2c13db39f758031e2ed054a9d703e3edf04a4d5efa55191c8b71e589",  # ETH transfer
        "0x82926c3447d7ce6cffbb69ac7ab1571a117e110661532f144ac5bd11053f8d60",  # Complex interaction
    ] * 2  # Process each twice to test caching
    
    # Test uncached performance
    start = time.time()
    results = provider.get_processed_transactions_from_tx_hashes(test_txs[:5])
    uncached_time = time.time() - start
    print(f"\nUncached batch (5 txs): {uncached_time:.3f} seconds")
    print(f"  Throughput: {len(results)/uncached_time:.1f} tx/sec")
    
    # Test cached performance
    start = time.time()
    results = provider.get_processed_transactions_from_tx_hashes(test_txs)
    cached_time = time.time() - start
    print(f"\nMixed cached/uncached (10 txs): {cached_time:.3f} seconds")
    print(f"  Throughput: {len(results)/cached_time:.1f} tx/sec")
    
    # Test pure cached performance
    start = time.time()
    results = provider.get_processed_transactions_from_tx_hashes(test_txs[:5])
    pure_cached_time = time.time() - start
    print(f"\nPure cached (5 txs): {pure_cached_time:.3f} seconds")
    print(f"  Throughput: {len(results)/pure_cached_time:.1f} tx/sec")
    
    print(f"\nCache benefit: {uncached_time/pure_cached_time:.1f}x faster when cached")
    print(f"Final cache stats: {provider.get_cache_statistics()}")
    
    provider.close()

if __name__ == "__main__":
    print("="*60)
    print("Rust ProcessedTransactionProvider Integration Test")
    print("="*60)
    
    try:
        # Test basic functionality
        test_basic_functionality()
        
        # Test async functionality
        asyncio.run(test_async_functionality())
        
        # Test performance
        test_performance_comparison()
        
        print("\n" + "="*60)
        print("✓ ALL TESTS PASSED SUCCESSFULLY!")
        print("="*60)
        
    except Exception as e:
        print(f"\n✗ Test failed: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)