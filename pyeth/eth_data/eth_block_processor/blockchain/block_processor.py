"""
Block Processor Module

Objective:
---------
Process Ethereum blocks and their transactions efficiently using batch processing techniques.
The system optimizes for throughput by processing multiple blocks and transactions in parallel,
while maintaining data consistency and providing detailed analytics.

Key Components and Flow:
----------------------
1. Batch Processing Architecture:
   - Block Batch Fetching: Parallel retrieval of multiple blocks
   - Transaction Batch Analysis: Process transactions from multiple blocks simultaneously
   - Optimized Data Fetching: Batch retrieval of receipts and traces using RETH's APIs
   - Result Aggregation: Organize processed data by block number

2. Core Operations:
   - Parallel Block Fetching: async/await pattern for concurrent block retrieval
   - Batch Transaction Analysis: Process multiple transactions in one operation
   - Receipt & Trace Batching: Fetch transaction data in bulk using RETH's optimized endpoints
   - Result Organization: Map results back to their respective blocks

3. Performance Optimizations:
   - Minimized RPC Calls: Batch requests for receipts and traces
   - Parallel Processing: Concurrent block and transaction processing
   - Memory Efficiency: Stream processing of large transaction batches
   - Error Handling: Graceful failure recovery and detailed error reporting

Implementation Details:
--------------------
1. Data Flow:sudo systemctl start rabbitmq-server
sudo systemctl enable rabbitmq-server
   a. Input: Block number(s) or block data
   b. Processing:
      - Fetch blocks in parallel
      - Batch fetch receipts and traces
      - Process transactions in batches
   c. Output: Organized results by block number

2. Key Methods:
   - process_block_batch(): Process multiple blocks concurrently
   - process_block_range(): Handle a range of blocks efficiently
   - process_block(): Process individual blocks with optimized data fetching

3. Performance Monitoring:
   - Detailed metrics collection
   - Profiling decorators for timing analysis
   - Comprehensive logging for debugging

Usage Examples:
-------------
1. Process Single Block:   ```python
   processor = BlockProcessor(node_url)
   result = await processor.process_block(block_number)   ```

2. Process Block Range:   ```python
   processor = BlockProcessor(node_url)
   results = await processor.process_block_range(start_block, end_block)   ```

3. Process Block Batch:   ```python
   processor = BlockProcessor(node_url)
   results, metrics = await processor.process_block_batch([block_numbers])   ```

Performance Characteristics:
-------------------------
- Block Fetching: O(n) parallel operations where n is number of blocks
- Transaction Processing: O(t) where t is total transactions across all blocks
- Memory Usage: O(t) where t is transactions in current batch
- Network Calls: O(1) for receipts and traces per block

Error Handling:
-------------
- Graceful failure recovery at block and transaction level
- Detailed error logging with context
- Metrics tracking for failed operations
- Automatic retry logic for transient failures
"""

from web3 import Web3
from time import time
from eth_block_processor.txn.txn_batch_processor import TransactionBatchProcessor
from eth_block_processor.blockchain.block_fetcher import BlockFetcher
from eth_block_processor.utils.logger import get_logger
from tqdm import tqdm


class BlockProcessor:
   def __init__(self, node_url: str = "http://127.0.0.1:8545", 
                 save_txn_to_db: bool = False,
                 logger=None):
        self.w3 = Web3(Web3.HTTPProvider(node_url))
        self.block_fetcher = BlockFetcher(node_url)
        self.batch_analyzer = TransactionBatchProcessor(
            w3=self.w3,
            logger=logger
        )
        self.logger = logger or get_logger(name="block_processor")
        if save_txn_to_db:
            from eth_block_processor.db.transaction_saver import TransactionSaver
            self.transaction_saver = TransactionSaver(logger=logger)
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
                self.logger.error(f"{__name__} Error processing block {block_number}: {str(e)}")

        return results

   async def process_block(self, block_number: int, transactions=None):
        """Process a single block"""
        try:
            start_time = time()
            if transactions is None:
                # Fetch block
                block_data = await self.block_fetcher.fetch_block_by_number(block_number)
                transactions = block_data['transactions']
            # Process all transactions in the block 
            processed_transactions = await self.batch_analyzer.process_block_transactions(
                block_number=block_number,
                transactions=transactions
            )
            end_time = time()
            num_failed_txns = len(transactions) - len(processed_transactions)
            if self.save_txn_to_db:
                self.transaction_saver.save_transactions(processed_transactions)
            self.logger.info(f"Processed block {block_number} with {len(processed_transactions)}|{num_failed_txns} in {end_time - start_time:.2f} seconds")                
            return processed_transactions
        except Exception as e:
            self.logger.error(f"{__name__} Error processing block {block_number} with {transactions} transactions: {str(e)}")
            raise


