"""
Processed Transaction Provider
------------------------------

This module provides an efficient data access layer for retrieving and processing Ethereum transaction data from both
local caches and blockchain nodes (like Reth).

# Objective

The primary goal of ProcessedTransactionProvider is to retrieve blockchain transaction data with minimal repeat
processing by implementing a robust caching system, reducing network calls and CPU-intensive operations.

# Caching Architecture

The provider employs a two-level caching strategy:
1. Block-level cache: Stores all processed transactions for each block (processed_block_cache)
2. Transaction-level cache: Stores individual transaction objects by hash (processed_tx_cache)

This dual approach enables efficient lookups when:
- Multiple transactions from the same block are needed
- The same transaction is accessed repeatedly
- Address activity spans multiple transactions across common blocks

# Performance Considerations

Performance optimizations include:
- Immediate cache checks before any processing
- Batched database lookups to reduce query overhead
- Elimination of duplicate block processing
- Block pre-loading for expected future access patterns
- Asynchronous processing with synchronous wrappers for compatibility

# Key Methods

- get_processed_transactions_from_block_numbers: Retrieves transactions for specific blocks, using cache first
- get_processed_transactions_from_tx_hashes: Retrieves transactions by hash, resolving block numbers as needed
- fetch_address_processed_transactions: Gets all transactions involving a specific address
- load_blocks_into_cache: Pre-loads blocks into cache for anticipated future use

# Algorithm Flow

For transaction retrieval:
1. Check transaction cache for direct hits
2. For missing transactions, determine their block numbers
3. Check block cache for blocks containing those transactions
4. Process only uncached blocks (avoiding redundant processing)
5. Extract needed transactions from processed blocks
6. Update both cache levels with new data
7. Return requested transactions

# Dependencies

- BlockProcessor: For actual block processing and transaction extraction
- TxMetaDataFetcher: For database lookups of transaction metadata
"""

from typing import List, Dict, Any, Optional, Set
from tqdm import tqdm
from web3 import Web3

from eth_data.blockchain.block_processor import BlockProcessor
from eth_data.tx_processor.data_models.txn_models import ProcessedTransaction
from eth_data.database.db_fetchers.tx_meta_data_fetcher import TxMetaDataFetcher


class ProcessedTransactionProvider:
    """
    Orchestrates fetching transaction data from the database and node,
    processes it, and provides ProcessedTransaction objects with caching.
    """

    def __init__(self, w3=None, logger=None):
        """
        Initializes the provider.

        Args:
            w3: Web3 instance
            logger: An optional logger instance.
        """
        
        self.w3 = w3 or Web3(Web3.HTTPProvider('http://localhost:8545'))
        self.logger = logger
        self.tx_meta_data_fetcher = TxMetaDataFetcher(logger=self.logger)
        self.block_processor = BlockProcessor(w3=self.w3, logger=logger, calculate_state_changes=True)

        # Cache for fully processed blocks: {block_number: List[ProcessedTransaction]}
        self.processed_block_cache: Dict[int, List[ProcessedTransaction]] = {}
        # Cache for specific processed transactions: {tx_hash: ProcessedTransaction}
        self.processed_tx_cache: Dict[str, ProcessedTransaction] = {}


    def _update_tx_cache(self, processed_txs: List[ProcessedTransaction]):
        for tx in processed_txs:
            self.processed_tx_cache[tx.hash] = tx

    async def get_processed_transactions_from_block_numbers(
        self,
        target_block_numbers: List[int]
    ) -> Dict[int, List[ProcessedTransaction]]:
        """
        Fetches and returns ProcessedTransaction objects for a given list of block numbers.
        Utilizes caching of processed blocks for efficiency.
        
        This is a core method that implements the block-caching logic. It checks the cache
        first for each requested block and only processes blocks that aren't already cached.
        Both successful and failed block processing results are cached to prevent repeated
        attempts to process problematic blocks.
        
        The method optimizes for:
        - Minimal network calls by batch processing only uncached blocks
        - No redundant processing of previously seen blocks
        - Fast lookup of frequently accessed blocks
        
        Args:
            target_block_numbers: List of block numbers to retrieve transactions for
            
        Returns:
            Dictionary mapping block numbers to lists of ProcessedTransaction objects
            
        Performance note:
            This method is significantly faster than processing each block individually,
            especially when many blocks are already in the cache.
        """
        if not target_block_numbers:
            return {}
            
        # Use a set for faster lookups and to eliminate duplicates
        unique_block_numbers = set(target_block_numbers)
        block_processed_txs: Dict[int, List[ProcessedTransaction]] = {}
        blocks_to_load: List[int] = []

        # Check cache first and identify blocks that need to be loaded
        for block_num in unique_block_numbers:
            if block_num in self.processed_block_cache:
                block_processed_txs[block_num] = self.processed_block_cache[block_num]
            else:
                blocks_to_load.append(block_num)
                
        # No need to process if all blocks are cached
        if not blocks_to_load:
            return block_processed_txs

        # Process blocks that aren't in cache
        if self.logger: self.logger.info(f"Processing {len(blocks_to_load)} blocks (out of {len(unique_block_numbers)} requested) that weren't in cache")
        
        for block_num in blocks_to_load:
            try:
                processed_txs_in_block = await self.block_processor.process_block(block_num)
                
                # Store in cache and results
                if processed_txs_in_block is not None:
                    self.processed_block_cache[block_num] = processed_txs_in_block
                    block_processed_txs[block_num] = processed_txs_in_block
                    self._update_tx_cache(processed_txs_in_block)
                    if self.logger and len(processed_txs_in_block) > 0:
                        self.logger.debug(f"Processed block {block_num}: {len(processed_txs_in_block)} transactions")
                else:
                    # Handle empty or failed blocks
                    self.processed_block_cache[block_num] = []
                    block_processed_txs[block_num] = []
                    if self.logger:
                        self.logger.debug(f"Block {block_num} processed with no transactions")
            except Exception as e:
                if self.logger:
                    self.logger.error(f"Failed to process block {block_num}: {e}")
                # Still add empty entry to avoid repeated processing attempts
                self.processed_block_cache[block_num] = []
                block_processed_txs[block_num] = []

        return block_processed_txs
    
    async def get_processed_transactions_from_tx_hashes(
        self,
        target_tx_hashes: List[str]
    ) -> Dict[str, ProcessedTransaction]:
        """
        Fetches and returns ProcessedTransaction objects for a given list of transaction hashes.
        Utilizes caching of processed blocks and transactions for efficiency.
        
        This method implements a multi-level lookup strategy:
        1. First checks the transaction-level cache for immediate hits
        2. For misses, looks up block numbers for each transaction hash via database
        3. Retrieves and processes missing blocks using get_processed_transactions_from_block_numbers()
        4. Extracts the specific transactions needed from those blocks
        5. Updates both transaction and block caches
        
        This approach is optimized for cases where:
        - Multiple transactions may be requested from the same block
        - The same transactions are accessed repeatedly in a session
        - Transaction data needs to be enriched with additional blockchain context
        
        Args:
            target_tx_hashes: A list of transaction hashes to retrieve processed data for.
            
        Returns:
            A dictionary mapping transaction hashes to their ProcessedTransaction objects.
            Hashes for which data could not be retrieved will be missing from the dictionary.
        """
        if not target_tx_hashes:
            return {}
            
        final_results: Dict[str, ProcessedTransaction] = {}
        txs_to_find: List[str] = []

        # 1. Check transaction cache first
        for tx_hash in target_tx_hashes:
            if tx_hash in self.processed_tx_cache:
                final_results[tx_hash] = self.processed_tx_cache[tx_hash]
            else:
                txs_to_find.append(tx_hash)

        if not txs_to_find:
            return final_results

        # 2. Identify blocks for missing transactions using the fetcher
        try:
            tx_to_block_map = self.tx_meta_data_fetcher.get_block_number_for_tx_hashes(txs_to_find)
            if not tx_to_block_map:
                if self.logger:
                    self.logger.warning(f"Could not find block numbers for {len(txs_to_find)} transactions")
                return final_results
        except Exception as e:
            if self.logger:
                self.logger.error(f"Database error fetching block numbers for transactions: {e}", exc_info=True)
            return final_results

        # 3. Get unique blocks needed
        unique_block_numbers = set(tx_to_block_map.values())
        
        # 4. Process needed blocks and get processed transactions
        processed_blocks = await self.get_processed_transactions_from_block_numbers(list(unique_block_numbers))
        
        # 5. Get transactions from processed blocks
        for tx_hash in txs_to_find:
            block_number = tx_to_block_map.get(tx_hash)
            if block_number is None or block_number not in processed_blocks:
                continue
                
            # Find the transaction in the block
            for processed_tx in processed_blocks[block_number]:
                if processed_tx.hash == tx_hash:
                    final_results[tx_hash] = processed_tx
                    self.processed_tx_cache[tx_hash] = processed_tx
                    break

        return final_results
    
    def fetch_address_processed_transactions(
        self,
        address: str,
        start_block: Optional[int] = None,
        end_block: Optional[int] = None, 
        num_blocks: Optional[int] = None,
    ) -> Dict[str, ProcessedTransaction]:
        """Fetches tx hashes from DB and then processed tx data from provider."""
        try:
            # Helper function to run async code synchronously
            def run_async_safely(coro):
                try:
                    # Check if we're in a running event loop
                    import asyncio
                    loop = asyncio.get_event_loop()
                    if loop.is_running():
                        # We're in a Jupyter notebook or similar environment
                        # Run in a thread with its own event loop
                        import concurrent.futures
                        with concurrent.futures.ThreadPoolExecutor() as executor:
                            def run_in_new_loop(coroutine):
                                new_loop = asyncio.new_event_loop()
                                asyncio.set_event_loop(new_loop)
                                try:
                                    return new_loop.run_until_complete(coroutine)
                                finally:
                                    new_loop.close()
                            return executor.submit(run_in_new_loop, coro).result()
                    else:
                        # Standard case - use the current loop
                        return loop.run_until_complete(coro)
                except RuntimeError:
                    # No event loop - create one
                    loop = asyncio.new_event_loop()
                    asyncio.set_event_loop(loop)
                    try:
                        return loop.run_until_complete(coro)
                    finally:
                        loop.close()
                except Exception as e:
                    if self.logger:
                        self.logger.error(f"Error running async code: {e}", exc_info=True)
                    raise
            
            # Define the async implementation
            async def fetch_async():
                address_processed_txs = {}
                tx_data = self.tx_meta_data_fetcher.get_tx_hashes_and_blocks_for_address(
                    address, start_block, end_block, num_blocks
                )
                if not tx_data:
                    return {}
                
                # Get block numbers and tx hashes
                block_numbers = [tx[1] for tx in tx_data]
                tx_hashes = [tx[0] for tx in tx_data]
                
                # Process the blocks
                processed_blocks = await self.get_processed_transactions_from_block_numbers(block_numbers)
                
                # Extract transactions matching the hashes
                for tx_hash in tx_hashes:
                    for block_num in processed_blocks:
                        for tx in processed_blocks[block_num]:
                            if tx.hash == tx_hash:
                                address_processed_txs[tx_hash] = tx
                                break
                
                return address_processed_txs
            
            # Run the async implementation synchronously
            return run_async_safely(fetch_async())
            
        except Exception as e:
            if self.logger:
                self.logger.error(f"Failed to fetch transactions for {address}: {e}", exc_info=True)
            return {}
        
    async def load_blocks_into_cache(self, blocks_to_load: List[int]):
        """
        Loads processed transaction data for a list of blocks into cache.
        Skips blocks that are already in the cache.
        
        This method is particularly useful for pre-loading blocks that will be needed
        in the near future, such as when expanding a fund flow network or analyzing
        address activity over a time range. By proactively loading blocks, subsequent
        transaction lookups become much faster.
        
        Performance characteristics:
        - Only processes each unique block once
        - Skips blocks already in cache
        - Updates both block-level and transaction-level caches
        - Handles failures gracefully (storing empty lists for failed blocks)
        
        Args:
            blocks_to_load: List of block numbers to load into cache
            
        Returns:
            None (results are stored in the instance caches)
        """
        if not blocks_to_load:
            return

        unique_blocks_to_load = sorted(list(set(blocks_to_load)))
        
        # Filter out blocks that are already in the cache
        blocks_not_in_cache = [block_num for block_num in unique_blocks_to_load 
                               if block_num not in self.processed_block_cache]
        
        if not blocks_not_in_cache:
            if self.logger: 
                self.logger.info(f"All {len(unique_blocks_to_load)} blocks already in cache")
            return
        
        if self.logger: 
            self.logger.info(f"Loading {len(blocks_not_in_cache)} blocks into cache (out of {len(unique_blocks_to_load)} requested)")
        
        processed_count = 0
        failed_count = 0
        total_tx_count = 0
        
        for block_num in blocks_not_in_cache:
            try:
                processed_txs_in_block = await self.block_processor.process_block(block_num)
                
                if processed_txs_in_block is not None:
                    self.processed_block_cache[block_num] = processed_txs_in_block
                    self._update_tx_cache(processed_txs_in_block)
                    processed_count += 1
                    total_tx_count += len(processed_txs_in_block)
                    if self.logger and len(processed_txs_in_block) > 0:
                        self.logger.debug(f"Processed block {block_num} with {len(processed_txs_in_block)} transactions")
                else:
                    # Empty block or processing failed
                    self.processed_block_cache[block_num] = []
                    failed_count += 1
            except Exception as e:
                failed_count += 1
                if self.logger: 
                    self.logger.error(f"Failed to process block {block_num} for cache: {e}")
                # Store empty list to prevent repeated processing attempts
                self.processed_block_cache[block_num] = []
        
        if self.logger: 
            self.logger.info(f"Cache update complete. Successfully processed {processed_count} blocks with {total_tx_count} txns. Failed: {failed_count} blocks.")
        