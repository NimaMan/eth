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
    *   **Save Results (Optional)**: If configured, pass the list of `ProcessedTransaction` objects returned by the batch processor to `TransactionWriter`.
    *   **Return**: `ProcessedBlockResult` bundling processed transactions and the serialized block header.

3.  **Block Range Processing (`process_block_range`)**:
    *   **Input**: `start_block`, `end_block`.
    *   **Iterate**: Loop through each `block_number` in the range.
    *   **Delegate**: Call `process_block(block_number)` for each block.
    *   **Aggregate**: Collect results per block number.
    *   **Return**: Dictionary mapping block numbers to `ProcessedBlockResult` instances.

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
1. Data Flow:
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
   result = await processor.process_block(block_number)
   print(result.block_header)
   for tx in result:
       ...   ```

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
from typing import Any, Dict, Optional

from tqdm import tqdm
from web3 import Web3

from .block_data_models import BlockHeader, ProcessedBlockResult
from eth_data.blockchain.block_fetcher import BlockFetcher
from eth_data.database.writers.transaction_writer import TransactionAddresstoTxIndexer
from eth_data.tx_processor.tx_batch_processor import TransactionBatchProcessor
#from baygus.qarqa.ethereum_today.etfs.etf_analyzer import BlockLevelETFAnalyzer
#from baygus.qarqa.ethereum_today.stablecoins.stablecoin_analyzer import BlockLevelStablecoinAnalyzer


class BlockProcessor:
    def __init__(self, node_url: str = "http://127.0.0.1:8545", 
                 index_address_txs: bool = False,
                 calculate_address_balance_changes: bool = False,
                 logger=None, 
                 w3=None):
        self.w3 = w3 or Web3(Web3.HTTPProvider(node_url))
        self.index_address_txs = index_address_txs
        self.logger = logger
        self.block_fetcher = BlockFetcher(node_url)
        self.tx_batch_processor = TransactionBatchProcessor(
            w3=self.w3,
            logger=logger,
            calculate_address_balance_changes=calculate_address_balance_changes
        )
        self.transaction_writer = TransactionAddresstoTxIndexer() if index_address_txs else None
        self.stablecoin_analyzer = None
        self.etf_analyzer = None

    def _extract_block_header(self, block_data: Dict[str, Any]) -> Optional[BlockHeader]:
        if not block_data:
            return None
        return BlockHeader.from_rpc_dict(block_data)
    
    async def process_block(
        self,
        block_number: int,
        transactions=None,
    ) -> ProcessedBlockResult:
        """Process a single block."""
        try:
            start_time = time.perf_counter()
            block_data = await self.block_fetcher.fetch_block_by_number(block_number)
            block_timestamp = block_data['timestamp']
            block_header = self._extract_block_header(block_data)
            if transactions is None:
                transactions = block_data['transactions']
            processed_transactions = await self.tx_batch_processor.process_block_transactions(
                block_number=block_number,
                transactions=transactions,
                block_timestamp=block_timestamp,
            )
            end_time = time.perf_counter()
            if self.index_address_txs and self.transaction_writer is not None:
                self.transaction_writer.write_transactions_address_tx(processed_transactions)

            if self.logger is not None:
                num_failed_txs = len(transactions) - len(processed_transactions)
                self.logger.info(
                    f"{block_number}->{len(processed_transactions)}|{num_failed_txs} in {end_time - start_time:.2f}s"
                )

            result = ProcessedBlockResult(
                transactions=processed_transactions,
                block_header=block_header,
            )
            return result
        except Exception as e:
            if self.logger is not None:
                self.logger.error(
                    f"{__name__} Error processing block {block_number} with {len(transactions)} transactions: {str(e)}",
                    exc_info=True,
                )
            raise

    async def process_block_range(self, start_block: int, end_block: int) -> Dict[int, ProcessedBlockResult]:
        """
        Process a range of blocks sequentially with progress bar
        Args:
            start_block: Starting block number
            end_block: Ending block number
        """
        results: Dict[int, ProcessedBlockResult] = {}
        for block_number in tqdm(range(start_block, end_block + 1), desc="Processing blocks", unit="blocks"):    
            try:
                result = await self.process_block(block_number)
                results[block_number] = result
            except Exception as e:
                if self.logger is not None:
                    self.logger.error(f"{__name__} Error processing block {block_number}: {str(e)}", exc_info=True)

        return results

    async def process_blocks_in_batch(self, block_numbers: list[int]) -> Dict[int, ProcessedBlockResult]:
        """Process a batch of blocks"""
        try:
            start_time = time.perf_counter()
            blocks = await self.block_fetcher.fetch_blocks_batch(block_numbers[0], block_numbers[-1])
            processed_results: Dict[int, ProcessedBlockResult] = {}
            for block_number, block_data in blocks.items():
                transactions = block_data['transactions']
                block_timestamp = block_data['timestamp']
                    
                processed_txs = await self.tx_batch_processor.process_block_transactions(
                    block_number=block_number,
                    transactions=transactions,
                    block_timestamp=block_timestamp
                )
                header = self._extract_block_header(block_data)
                processed_results[block_number] = ProcessedBlockResult(
                    transactions=processed_txs,
                    block_header=header,
                )

            end_time = time.perf_counter()
            if self.logger is not None:
                self.logger.info(f"Processed {len(block_numbers)} blocks in batch in {end_time - start_time:.4f} seconds")
            return processed_results
        except Exception as e:
            if self.logger is not None:
                self.logger.error(f"{__name__} Error processing blocks in batch: {str(e)}", exc_info=True)
            raise

    async def close(self):
        """Close all aiohttp sessions and other resources"""
        # Close block fetcher sessions
        await self.block_fetcher.close()    
        if self.logger:
            self.logger.info("BlockProcessor core resources cleaned up (fetcher, batch processor)")
    
