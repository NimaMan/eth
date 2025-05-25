import pytest
import pytest_asyncio
import asyncio
import time
from typing import List, Dict, Any

from eth_block_processor.blockchain.block_processor import BlockProcessor
from eth_block_processor.data_models.txn_models import ProcessedTransaction

# Configuration
NODE_URL = "http://127.0.0.1:8545"  # Adjust if your node URL is different
NUM_BLOCKS_TO_PROCESS = 10         # Number of recent blocks to test with (increased from 5 to 10)

@pytest_asyncio.fixture(scope="module")
async def block_processor():
    """Provides an initialized BlockProcessor instance and closes it afterwards."""
    # Set save_txn_to_db=False to avoid database side effects during testing
    processor = BlockProcessor(node_url=NODE_URL, save_txn_to_db=False, calculate_state_changes=True)
    yield processor
    # Ensure cleanup
    print("\nClosing BlockProcessor resources...")
    await processor.close()


def compare_processed_txs(list1: List[ProcessedTransaction], list2: List[ProcessedTransaction]) -> bool:
    """Helper to compare two lists of ProcessedTransaction objects based on hash."""
    if len(list1) != len(list2):
        print(f"Length mismatch: {len(list1)} vs {len(list2)}")
        return False
    
    hashes1 = sorted([tx.hash for tx in list1])
    hashes2 = sorted([tx.hash for tx in list2])
    
    if hashes1 != hashes2:
        print(f"Hash set mismatch: {set(hashes1).symmetric_difference(set(hashes2))}")
        return False
        
    # Optional: Add deeper comparison if needed (e.g., check specific fields)
    # for h in hashes1:
    #     tx1 = next(tx for tx in list1 if tx.hash == h)
    #     tx2 = next(tx for tx in list2 if tx.hash == h)
    #     if tx1 != tx2: # Requires __eq__ method on ProcessedTransaction
    #         print(f"Data mismatch for tx {h}")
    #         return False
            
    return True

@pytest.mark.asyncio
async def test_processor_performance_comparison(block_processor: BlockProcessor):
    """Compares performance of sequential vs batch block processing."""
    print(f"\n--- Testing BlockProcessor Performance ({NUM_BLOCKS_TO_PROCESS} Blocks) ---")

    try:
        latest_block_num = await block_processor.block_fetcher.fetch_latest_block_number()
        print(f"Latest block number: {latest_block_num}")
    except Exception as e:
        pytest.fail(f"Failed to connect to the node or fetch latest block: {e}")

    if latest_block_num < NUM_BLOCKS_TO_PROCESS:
        pytest.skip(f"Not enough blocks on the chain ({latest_block_num}) to run test with {NUM_BLOCKS_TO_PROCESS} blocks.")

    target_block_numbers = list(range(latest_block_num - NUM_BLOCKS_TO_PROCESS + 1, latest_block_num + 1))
    print(f"Target block numbers: {min(target_block_numbers)} to {max(target_block_numbers)}")

    # --- Method 1: Sequential Processing ---
    print("Running sequential processing...")
    start_time_sequential = time.perf_counter()
    sequential_results: Dict[int, List[ProcessedTransaction]] = {}
    for block_num in target_block_numbers:
        try:
            # Using process_block which fetches and processes one by one
            processed_txs = await block_processor.process_block(block_num)
            sequential_results[block_num] = processed_txs
        except Exception as e:
            print(f"Error processing block {block_num} sequentially: {e}")
            sequential_results[block_num] = [] # Mark as empty on error
    end_time_sequential = time.perf_counter()
    duration_sequential = end_time_sequential - start_time_sequential
    print(f"Sequential processing duration: {duration_sequential:.4f} seconds")

    # --- Method 2: Batch Processing ---
    print("Running batch processing...")
    start_time_batch = time.perf_counter()
    batch_results_dict = {}
    try:
        batch_results_dict = await block_processor.process_blocks_in_batch(target_block_numbers)
    except Exception as e:
        pytest.fail(f"Batch processing method failed: {e}")
    end_time_batch = time.perf_counter()
    duration_batch = end_time_batch - start_time_batch
    print(f"Batch processing duration: {duration_batch:.4f} seconds")

    # --- Comparison and Assertions ---
    print("Comparing results...")
    sequential_keys = set(sequential_results.keys())
    batch_keys = set(batch_results_dict.keys())

    # Assert that both methods attempted the same set of blocks (though some might have failed)
    assert sequential_keys == batch_keys, f"Mismatch in block numbers processed. Sequential: {len(sequential_keys)}, Batch: {len(batch_keys)}. Diff: {sequential_keys.symmetric_difference(batch_keys)}"
    assert len(batch_keys) == NUM_BLOCKS_TO_PROCESS, f"Batch method did not return results for expected number of blocks. Got {len(batch_keys)}, expected {NUM_BLOCKS_TO_PROCESS}"

    # Assert that the processed transactions for each block are consistent
    mismatched_blocks = []
    for block_num in batch_keys:
        seq_txs = sequential_results.get(block_num, [])
        batch_txs = batch_results_dict.get(block_num, [])
        
        if not compare_processed_txs(seq_txs, batch_txs):
            mismatched_blocks.append(block_num)
            print(f"Mismatch detected for block {block_num}")
            print(f"  Sequential tx hashes: {[tx.hash for tx in seq_txs]}")
            print(f"  Batch tx hashes: {[tx.hash for tx in batch_txs]}")

    assert not mismatched_blocks, f"Processed transaction mismatch between methods for blocks: {mismatched_blocks}"

    print("Results are consistent.")
    if duration_batch > 0:
        gain = duration_sequential / duration_batch
        print(f"Performance Gain (Sequential / Batch): {gain:.2f}x faster")
    else:
         print("Batch took negligible time, cannot calculate gain.")
    print("---")

# To run this test: navigate to the root directory and run:
# pytest py/eth_block_processor/tests/blocks/test_block_processor_performance.py -s 