"""
Block Processor Module

Objective:
---------
Orchestrate the fetching and processing of Ethereum blocks and their transactions. This module acts as the entry point for ingesting blockchain data, delegating transaction-level analysis to specialized processors, and optionally saving results. It aims for efficient throughput, particularly when processing ranges or batches of blocks.

# Algorithm Overview

The `BlockProcessor` coordinates the following steps:

1.  **Initialization (`__init__`)**:
    *   Set up connections (Web3).
    *   Initialize helper components:
        *   `BlockFetcher`: For retrieving block data from the node.
        *   `TransactionBatchProcessor`: For analyzing transactions within a block.
        *   `TransactionWriter` (Optional): For saving processed transaction data to the `eth_db`.
    *   Configure options like database saving and state change calculation.

2.  **Single Block Processing (`process_block`)**:
    *   **Input**: `block_number`.
    *   **Fetch Block Data**: Use `BlockFetcher` to get block details (timestamp, list of transaction hashes/objects).
    *   **Process Transactions**: Pass the list of transactions and block metadata to `TransactionBatchProcessor.process_block_transactions`. This sub-processor handles fetching receipts/traces and analyzing each transaction individually (using `TransactionProcessor`).
    *   **Save Results (Optional)**: If configured, pass the list of `ProcessedTransaction` objects returned by the batch processor to `TransactionWriter.save_transactions`.
    *   **Return**: List of `ProcessedTransaction` objects for the block.

3.  **Block Range Processing (`process_block_range`)**:
    *   **Input**: `start_block`, `end_block`.
    *   **Iterate**: Loop through each `block_number` in the range.
    *   **Delegate**: Call `process_block(block_number)` for each block.
    *   **Aggregate**: Collect results per block number.
    *   **Return**: Dictionary mapping block numbers to their lists of `ProcessedTransaction` objects.

(Note: Batch processing of *multiple blocks* concurrently (`process_block_batch` mentioned in original docstring) is not implemented in the current code version provided but could be a future optimization.)

Key Components and Flow:
----------------------
1. Batch Processing Architecture:
   - Block Batch Fetching: Parallel retrieval of multiple blocks (via `BlockFetcher`, potentially used by a future `process_block_batch`)
   - Transaction Batch Analysis: Process transactions from multiple blocks simultaneously (handled by `TransactionBatchProcessor` which fetches receipts/traces in batches for *one block* at a time currently)
   - Optimized Data Fetching: Batch retrieval of receipts and traces using RETH's APIs (within `TransactionBatchProcessor`)
   - Result Aggregation: Organize processed data by block number

2. Core Operations:
   - Parallel Block Fetching: async/await pattern for concurrent block retrieval (in `BlockFetcher`)
   - Batch Transaction Analysis: Process multiple transactions in one operation (within `TransactionBatchProcessor`)
   - Receipt & Trace Batching: Fetch transaction data in bulk using RETH's optimized endpoints (within `TransactionBatchProcessor`)
   - Result Organization: Map results back to their respective blocks

3. Performance Optimizations:
   - Minimized RPC Calls: Batch requests for receipts and traces (within `TransactionBatchProcessor`)
   - Parallel Processing: Concurrent block and transaction processing (limited concurrency in current implementation - mainly async fetching within `BlockFetcher` and `TransactionBatchProcessor`)
   - Memory Efficiency: Stream processing of large transaction batches (potential within `TransactionBatchProcessor`)
   - Error Handling: Graceful failure recovery and detailed error reporting

Implementation Details:
--------------------
1. Data Flow:sudo systemctl start rabbitmq-server
sudo systemctl enable rabbitmq-server
   a. Input: Block number(s) or block data
   b. Processing:
      - Fetch blocks (`BlockFetcher`)
      - Batch fetch receipts and traces (`TransactionBatchProcessor`)
      - Process transactions (`TransactionBatchProcessor` -> `TransactionProcessor`)
      - Save transactions (Optional: `TransactionWriter`)
   c. Output: Organized results by block number (`process_block_range`) or list of processed transactions (`process_block`)

2. Key Methods:
   - process_block_range(): Handle a range of blocks efficiently (sequentially calling `process_block`)
   - process_block(): Process individual blocks with optimized data fetching (delegating heavily to `TransactionBatchProcessor`)

3. Performance Monitoring:
   - Detailed metrics collection (basic timing and logging implemented)
   - Profiling decorators for timing analysis (not present)
   - Comprehensive logging for debugging (present if logger is provided)

Usage Examples:
-------------
1. Process Single Block:   ```python
   processor = BlockProcessor(node_url)
   result = await processor.process_block(block_number)   ```

2. Process Block Range:   ```python
   processor = BlockProcessor(node_url)
   results = await processor.process_block_range(start_block, end_block)   ```

(Note: `process_block_batch` example removed as method is not implemented)

Performance Characteristics:
-------------------------
- Block Fetching: O(1) per block (async fetch)
- Transaction Processing: O(T_block) where T_block is transactions in the block (dominated by `TransactionBatchProcessor`'s RPC calls and analysis)
- Memory Usage: O(T_block) for storing processed transactions per block.
- Network Calls: Primarily driven by `TransactionBatchProcessor` (batches receipts/traces per block).

Error Handling:
-------------
- Graceful failure recovery at block level in `process_block_range`.
- Detailed error logging with context (if logger provided).
- Metrics tracking for failed operations (simple count logged).
- Automatic retry logic for transient failures (depends on underlying fetchers/processors).
"""

import time
from tqdm import tqdm
from web3 import Web3
from eth_block_processor.txn.txn_batch_processor import TransactionBatchProcessor
from eth_block_processor.blockchain.block_fetcher import BlockFetcher
from sarigoz.data.db.writers.transaction_writer import TransactionWriter 
from sarigoz.stablecoins.stablecoin_analyzer import BlockLevelStablecoinAnalyzer           


class BlockProcessor:
    def __init__(self, node_url: str = "http://127.0.0.1:8545", 
                 save_txn_to_db: bool = False,
                 calculate_state_changes: bool = False,
                 logger=None, 
                 w3=None):
        self.w3 = w3 or Web3(Web3.HTTPProvider(node_url))
        self.logger = logger
        self.block_fetcher = BlockFetcher(node_url)
        self.tx_batch_processor = TransactionBatchProcessor(
            w3=self.w3,
            logger=logger,
            calculate_state_changes=calculate_state_changes
        )
        self.transaction_writer = TransactionWriter(w3=self.w3, logger=logger)
        self.stablecoin_analyzer = BlockLevelStablecoinAnalyzer(w3=self.w3)
        self.save_txn_to_db = save_txn_to_db
    
    async def process_block_range(self, start_block: int, end_block: int):
        """
        Process a range of blocks sequentially with progress bar
        Args:
            start_block: Starting block number
            end_block: Ending block number
        """
        results = {}
        
        for block_number in tqdm(range(start_block, end_block + 1), desc="Processing blocks", unit="blocks"):    
            try:
                result = await self.process_block(block_number)
                results[block_number] = result
            except Exception as e:
                if self.logger is not None:
                    self.logger.error(f"{__name__} Error processing block {block_number}: {str(e)}", exc_info=True)

        return results

    async def process_block(self, block_number: int, transactions=None):
        """Process a single block"""
        try:
            start_time = time.perf_counter()
            if transactions is None:
                # Fetch block
                block_data = await self.block_fetcher.fetch_block_by_number(block_number)
                transactions = block_data['transactions']
                block_timestamp = block_data['timestamp']
            # Process all transactions in the block 
            processed_transactions = await self.tx_batch_processor.process_block_transactions(
                block_number=block_number,
                transactions=transactions,
                block_timestamp=block_timestamp
            )
            end_time = time.perf_counter()
            num_failed_txns = len(transactions) - len(processed_transactions)
            if self.save_txn_to_db:
                self.transaction_writer.save_transactions(processed_transactions)
                self.stablecoin_analyzer.process_block_transactions(block_number, processed_transactions)
            if self.logger is not None:
                self.logger.info(f"{block_number}->{len(processed_transactions)}|{num_failed_txns} in {end_time - start_time:.2f}s")                
            return processed_transactions
        except Exception as e:
            if self.logger is not None:
                self.logger.error(f"{__name__} Error processing block {block_number} with {len(transactions)} transactions: {str(e)}", exc_info=True)
            raise

    async def close(self):
        """Close all aiohttp sessions and other resources"""
        # Close block fetcher sessions
        await self.block_fetcher.close()    
        if self.logger:
            self.logger.info("BlockProcessor core resources cleaned up (fetcher, batch processor)")
            