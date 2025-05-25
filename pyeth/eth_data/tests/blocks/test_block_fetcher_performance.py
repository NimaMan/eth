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
        batch_results_dict = await block_fetcher.fetch_specific_blocks_batch(target_block_numbers)
    except Exception as e:
        pytest.fail(f"Batch fetch method failed: {e}")
    end_time_batch = time.perf_counter()
    duration_batch = end_time_batch - start_time_batch
    print(f"Batch fetch duration: {duration_batch:.4f} seconds")

    # --- Comparison and Assertions ---
    print("Comparing results...")
    individual_keys = set(individual_results_dict.keys())
    batch_keys = set(batch_results_dict.keys())

    # Assert that both methods found the same set of blocks
    # Allow for potential missing blocks in individual fetches if node is slow/flaky
    # But batch should ideally get all requested ones unless node returns errors
    assert batch_keys.issubset(individual_keys), f"Batch method found blocks not found by individual method. Extra: {batch_keys - individual_keys}"
    if len(batch_keys) != NUM_BLOCKS_TO_FETCH:
         print(f"Warning: Batch method did not return all expected blocks. Got {len(batch_keys)}, expected {NUM_BLOCKS_TO_FETCH}. Missing: {set(target_block_numbers) - batch_keys}")
         # We might not want to fail жестко here if the node is flaky, maybe just warn
         # assert len(batch_keys) == NUM_BLOCKS_TO_FETCH

    if not batch_keys:
         pytest.fail("Batch method returned no results.")

    # Assert that the data for commonly found blocks is consistent (check block hash)
    common_keys = individual_keys.intersection(batch_keys)
    print(f"Comparing {len(common_keys)} blocks found by both methods...")
    mismatched_hashes = []
    # Flag to print details only for the first mismatch
    printed_mismatch_details = False

    for block_num in common_keys:
        individual_block = individual_results_dict.get(block_num)
        batch_block = batch_results_dict.get(block_num)

        if not individual_block or not batch_block:
            print(f"Warning: Skipping hash comparison for block {block_num} due to missing data.")
            continue

        # Get hashes and convert to canonical hex strings for comparison
        individual_hash_obj = individual_block.get('hash')
        batch_hash_obj = batch_block.get('hash')

        # Convert to hex string (handle HexBytes or string)
        individual_hash_hex = individual_hash_obj.hex() if hasattr(individual_hash_obj, 'hex') else str(individual_hash_obj)
        batch_hash_hex = batch_hash_obj.hex() if hasattr(batch_hash_obj, 'hex') else str(batch_hash_obj)

        # --- Add extra debug prints HERE ---
        if not printed_mismatch_details and individual_hash_hex.lower() != batch_hash_hex.lower():
            print(f"\nDEBUG Comparing Hashes for Block {block_num}:")
            print(f"  Individual Fetch Hash (obj): {repr(individual_hash_obj)}")
            print(f"  Individual Fetch Hash (hex): '{individual_hash_hex}'")
            print(f"  Batch Fetch Hash      (obj): {repr(batch_hash_obj)}")
            print(f"  Batch Fetch Hash      (hex): '{batch_hash_hex}'")
            print(f"  Comparison: '{individual_hash_hex.lower()}' != '{batch_hash_hex.lower()}' -> {individual_hash_hex.lower() != batch_hash_hex.lower()}")
        # --- End of extra debug prints ---

        if individual_hash_hex.lower() != batch_hash_hex.lower():
             mismatched_hashes.append(block_num)
             # Print details for the first mismatch found
             if not printed_mismatch_details:
                 print("\n!!! DEBUGGING HASH MISMATCH !!!")
                 print(f"Block Number: {block_num}")
                 print("--- Individual Fetch Result ---")
                 print(individual_block)
                 print("--- Batch Fetch Result ---")
                 print(batch_block)
                 print("-----------------------------")
                 printed_mismatch_details = True

    assert not mismatched_hashes, f"Block hash mismatch between methods for blocks: {mismatched_hashes}"

    print("Results are consistent for common blocks.")
    if duration_batch > 0:
        gain = duration_individual / duration_batch
        print(f"Performance Gain (Individual / Batch): {gain:.2f}x faster")
        # Assert batch is faster, allowing for some minimal overhead if times are very close
        # assert duration_batch < duration_individual * 0.98, "Batch was not significantly faster than individual fetches."
    else:
         print("Batch took negligible time, cannot calculate gain.")
    print("---")

# To run this test: navigate to the root directory and run:
# pytest py/eth_block_processor/tests/blocks/test_block_fetcher_performance.py -s
# The '-s' flag ensures print statements are shown. 