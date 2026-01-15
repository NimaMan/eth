"""
Test for BatchTransactionDataFetcher class.
Compares block-based and transaction-based fetching methods for all transactions in a block.
"""

import asyncio
import pytest
from web3 import Web3
import os
import time
from eth_data.tx_processor.tx_data_fetcher import TransactionBatchDataFetcher


@pytest.mark.asyncio
async def test_transaction_fetchers_full_block_comparison():
    """Test both transaction fetching methods with all transactions in a block"""
    
    # Initialize Web3 connection
    rpc_url = os.environ.get("ETH_RPC_URL", "http://localhost:8545")
    w3 = Web3(Web3.HTTPProvider(rpc_url))
    
    # Verify connection
    assert w3.is_connected(), "Failed to connect to Ethereum node"
    
    # Create fetcher instance
    fetcher = TransactionBatchDataFetcher(w3)
    
    # Get latest block number
    latest_block = w3.eth.block_number
    print(f"Testing with latest block: {latest_block}")
    
    # Measure time for fetch_block_data (all transactions)
    block_fetch_start = time.time()
    receipt_map, trace_map = await fetcher.fetch_block_data(latest_block)
    block_fetch_time = time.time() - block_fetch_start
    
    # Ensure we got some transactions
    tx_hashes = list(receipt_map.keys())
    
    # If the block has no transactions, try an earlier block
    retries = 0
    while len(tx_hashes) == 0 and retries < 5:
        retries += 1
        latest_block -= 1
        
        block_fetch_start = time.time()
        receipt_map, trace_map = await fetcher.fetch_block_data(latest_block)
        block_fetch_time = time.time() - block_fetch_start
        
        tx_hashes = list(receipt_map.keys())
    
    print(f"Processing all {len(tx_hashes)} transactions in block {latest_block}")
    
    # Now fetch the same transactions using the transaction-based method
    tx_fetch_start = time.time()
    transaction_map, batch_receipt_map, batch_trace_map = await fetcher.fetch_tx_hash_list(tx_hashes)
    tx_fetch_time = time.time() - tx_fetch_start
    
    # Verify result sizes match
    assert len(batch_receipt_map) == len(tx_hashes), "Transaction count mismatch"
    assert len(transaction_map) == len(tx_hashes), "Transaction count mismatch"
    
    # Verify all data matches
    data_matches = True
    for tx_hash in tx_hashes:
        # Check if essential data matches
        if (receipt_map[tx_hash]['blockNumber'] != batch_receipt_map[tx_hash]['blockNumber'] or
            receipt_map[tx_hash]['gasUsed'] != batch_receipt_map[tx_hash]['gasUsed']):
            data_matches = False
            break
            
        # Check trace data if available
        if tx_hash in trace_map and tx_hash in batch_trace_map:
            if (trace_map[tx_hash]['from'] != batch_trace_map[tx_hash]['from'] or
                trace_map[tx_hash]['to'] != batch_trace_map[tx_hash]['to']):
                data_matches = False
                break
    
    # Print performance metrics
    block_per_tx_time = block_fetch_time / len(tx_hashes) if len(tx_hashes) > 0 else 0
    tx_per_tx_time = tx_fetch_time / len(tx_hashes) if len(tx_hashes) > 0 else 0
    
    print("\n==================== RESULTS ====================")
    print(f"Data consistency check: {'PASSED' if data_matches else 'FAILED'}")
    print("\nPerformance Summary:")
    print(f"Block-based fetching: {block_fetch_time:.4f} seconds total, {block_per_tx_time:.6f} seconds per transaction")
    print(f"Transaction-based fetching: {tx_fetch_time:.4f} seconds total, {tx_per_tx_time:.6f} seconds per transaction")
    
    # Performance comparison
    if block_per_tx_time < tx_per_tx_time:
        efficiency_ratio = tx_per_tx_time / block_per_tx_time if block_per_tx_time > 0 else float('inf')
        print(f"\nFor this entire block, block-based fetching is {efficiency_ratio:.2f}x more efficient per transaction")
    else:
        efficiency_ratio = block_per_tx_time / tx_per_tx_time if tx_per_tx_time > 0 else float('inf')
        print(f"\nFor this entire block, transaction-based fetching is {efficiency_ratio:.2f}x more efficient per transaction")


if __name__ == "__main__":
    # Run test directly
    asyncio.run(test_transaction_fetchers_full_block_comparison()) 