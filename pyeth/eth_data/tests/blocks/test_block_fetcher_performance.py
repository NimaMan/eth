import pytest
import pytest_asyncio # Import the specific decorator
import asyncio
import time
from typing import List, Dict, Any

from eth_block_processor.blockchain.block_fetcher import BlockFetcher

# Configuration
NODE_URL = "http://127.0.0.1:8545"  # Adjust if your node URL is different
NUM_BLOCKS_TO_FETCH = 20         # Number of recent blocks to test with

@pytest.fixture(scope="module")
def event_loop():
    """Create an instance of the default event loop for the module."""
    # This override might cause warnings, consider removing if default loop scope is okay
    loop = asyncio.get_event_loop_policy().new_event_loop()
    yield loop
    loop.close()

# Use the specific decorator from pytest-asyncio
@pytest_asyncio.fixture(scope="module")
async def block_fetcher():
    """Provides an initialized BlockFetcher instance and closes it afterwards."""
    fetcher = BlockFetcher(node_url=NODE_URL)
    yield fetcher
    # Ensure cleanup
    print("\nClosing BlockFetcher connection...")
    await fetcher.close()

@pytest.mark.asyncio
async def test_fetch_performance_comparison(block_fetcher: BlockFetcher):
    """Compares performance of individual vs batch block fetching."""
    print(f"\n--- Testing BlockFetcher Performance ({NUM_BLOCKS_TO_FETCH} Blocks) ---")

    try:
        latest_block_num = await block_fetcher.fetch_latest_block_number()
        print(f"Latest block number: {latest_block_num}")
    except Exception as e:
        pytest.fail(f"Failed to connect to the node or fetch latest block: {e}")

    if latest_block_num < NUM_BLOCKS_TO_FETCH:
        pytest.skip(f"Not enough blocks on the chain ({latest_block_num}) to run test with {NUM_BLOCKS_TO_FETCH} blocks.")

    target_block_numbers = list(range(latest_block_num - NUM_BLOCKS_TO_FETCH + 1, latest_block_num + 1))
    print(f"Target block numbers: {min(target_block_numbers)} to {max(target_block_numbers)}")

    # --- Method 1: Individual Fetches (Concurrent) ---
    print("Running individual fetches concurrently...")
    start_time_individual = time.perf_counter()
    individual_tasks = [block_fetcher.fetch_block_by_number(num) for num in target_block_numbers]
    individual_results_list = await asyncio.gather(*individual_tasks)
    end_time_individual = time.perf_counter()
    duration_individual = end_time_individual - start_time_individual
    print(f"Individual fetches duration: {duration_individual:.4f} seconds")

    # Convert list results to dict for comparison
    individual_results_dict: Dict[int, Any] = {}
    missing_individual = 0
    for block_data in individual_results_list:
        if block_data:
            block_num_val = block_data.get('number')
            block_num = None
            if isinstance(block_num_val, int):
                block_num = block_num_val # Already an integer
            elif isinstance(block_num_val, str):
                try:
                    block_num = int(block_num_val, 16) # Convert from hex string
                except ValueError:
                    print(f"Warning: Could not convert block number string '{block_num_val}' to int.")
            
            if block_num is not None:
                individual_results_dict[block_num] = block_data
            else:
                 print(f"Warning: Could not determine block number from data: {block_data}")
                 missing_individual += 1 # Count as missing if number can't be determined
        else:
            missing_individual += 1
    if missing_individual > 0:
        print(f"Warning: Individual fetches returned None or unusable data for {missing_individual} block(s).")


    # --- Method 2: Batch Fetch ---
    print("Running batch fetch...")
    start_time_batch = time.perf_counter()
    batch_results_dict = {}
    try:
        batch_results_dict = await block_fetcher.fetch_blocks_batch(target_block_numbers[0], target_block_numbers[-1])
    except Exception as e:
        pytest.fail(f"Batch fetch method failed: {e}")
    end_time_batch = time.perf_counter()
    duration_batch = end_time_batch - start_time_batch
    print(f"Batch fetch duration: {duration_batch:.4f} seconds")

    print(f"Total batch results: {len(batch_results_dict)}")
    if not batch_results_dict:
        pytest.fail("Batch fetch did not return any results.")

    # --- Verification ---
    print("Verifying results...")
    assert len(individual_results_dict) == len(target_block_numbers), \
        f"Expected {len(target_block_numbers)} individual results, but got {len(individual_results_dict)}"
    
    assert len(batch_results_dict) == len(target_block_numbers), \
        f"Expected {len(target_block_numbers)} batch results, but got {len(batch_results_dict)}"
    
    # Verify that the block numbers match
    assert set(individual_results_dict.keys()) == set(batch_results_dict.keys()), \
        "Block numbers from individual and batch fetches do not match."
    
    # Verify that the block hashes match for each block
    for block_num in individual_results_dict:
        individual_hash = '0x' + individual_results_dict[block_num]['hash'].hex()
        batch_hash = batch_results_dict[block_num]['hash']
        assert individual_hash == batch_hash, \
            f"Hash mismatch for block {block_num}: individual '{individual_hash}' vs batch '{batch_hash}'"
    
    print(f"Results for all {len(target_block_numbers)} blocks verified successfully.")

    # --- Performance Summary ---
    print(f"\n--- Performance Summary ---")
    print(f"Individual (concurrent) fetches took {duration_individual:.4f} seconds.")
    print(f"Batch fetch took {duration_batch:.4f} seconds.")
    
    if duration_batch < duration_individual:
        improvement = (duration_individual - duration_batch) / duration_individual * 100
        print(f"Batch fetching was {improvement:.2f}% faster.")
    elif duration_individual < duration_batch:
        slowdown = (duration_batch - duration_individual) / duration_batch * 100
        print(f"Batch fetching was {slowdown:.2f}% slower.")
    else:
        print("Both methods took roughly the same amount of time.")

    print(f"--- Test Finished ---\n")
    assert True

# To run this test: navigate to the root directory and run:
# pytest py/eth_block_processor/tests/blocks/test_block_fetcher_performance.py -s
# The '-s' flag ensures print statements are shown. 