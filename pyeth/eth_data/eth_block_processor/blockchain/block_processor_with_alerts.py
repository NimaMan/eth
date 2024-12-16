"""
Block Processor Module

Objective:
---------
Provide a flexible and efficient system for processing Ethereum blocks and their transactions,
supporting different processing strategies for various use cases like real-time monitoring,
historical analysis, and parallel processing.

Key Components and Flow:
----------------------
1. Block Processing Strategies:
   - Sequential Processing: Process blocks one by one (current implementation)
   - Parallel Processing: Process multiple blocks concurrently
   - Batch Processing: Process blocks in configurable batch sizes
   - Real-time Processing: Process latest blocks as they arrive, as efficinetly as possible (preferably within 1 second)

2. Core Operations:
   - Block Fetching: Retrieve blocks from Ethereum node
   - Transaction Analysis: Detailed analysis of each transaction
   - Alert Generation: Monitor for specific patterns or events
   - Data Persistence: Store results and alerts in database

3. Performance Characteristics:
   - I/O Bound: Block fetching and database operations
   - CPU Bound: Transaction analysis and alert checking
   - Memory Usage: Depends on processing strategy and batch size

Design Considerations:
-------------------
1. Processing Strategies:
   - Sequential: Best for maintaining strict order and consistency
   - Parallel: Optimal for historical analysis and catching up
   - Real-time: Suitable for monitoring and immediate alerts
   - Batch: Balance between performance and resource usage

2. Trade-offs:
   - Sequential vs Parallel: Consistency vs Speed
   - Memory vs Performance: Batch size considerations. Memroy is not a botlleneck for our system.
   - Real-time vs Delayed: Latency vs Processing guarantees

3. Extensibility:
   - Strategy Pattern: Different processing implementations
   - Plugin Architecture: Custom analyzers and alert types
   - Configurable Components: Adjustable parameters per strategy

Usage Examples:
-------------
1. Real-time Monitoring:
   processor = RealTimeBlockProcessor(node_url)
   await processor.process_latest_block()

2. Historical Analysis:
   processor = ParallelBlockProcessor(node_url)
   await processor.process_block_range(start_block, end_block)

3. Batch Processing:
   processor = BatchBlockProcessor(node_url, batch_size=100)
   await processor.process_block_range(start_block, end_block)
"""

from web3 import Web3
import asyncio
from typing import Union, Dict

from eth_block_processor.txn.txn_analyzer import TransactionAnalyzer
from eth_block_processor.blockchain.block_fetcher import BlockFetcher
from eth_block_processor.alert.alert_manager import AlertManager
from eth_block_processor.alert.alert_db import AlertDB
from eth_block_processor.utils.logger import get_logger
from eth_block_processor.utils.profiling.decorators import profile_async


logger = get_logger(name="block_processor", log_folder="eth_block_processor")


class BlockProcessorwithAlerts:
    """
    A class to process Ethereum blocks and their transactions.

    This class combines BlockFetcher and TransactionAnalyzer to provide
    comprehensive block and transaction analysis.

    Attributes:
        w3 (Web3): A Web3 instance for interacting with the Ethereum network.
        block_fetcher (BlockFetcher): An instance of BlockFetcher for retrieving blocks.
        transaction_analyzer (TransactionAnalyzer): An instance of TransactionAnalyzer for analyzing transactions.
        alert_manager (AlertManager): An instance of AlertManager for managing alerts.
    """

    def __init__(self, node_url: str = "http://127.0.0.1:8545", 
                 save_alert_db: bool = True, 
                 save_erc20_txn_to_db: bool = True):
        """
        Initialize the BlockProcessor with necessary components.

        Args:
            node_url (str): A URL for interacting with the Ethereum network.
            save_alert_db (bool): Whether to save alerts to the database. Defaults to True.
            save_erc20_transaction (bool): Whether to save ERC20 transactions to the database. Defaults to True.
        """
        self.w3 = Web3(Web3.HTTPProvider(node_url))
        self.block_fetcher = BlockFetcher(node_url)
        self.transaction_analyzer = TransactionAnalyzer(w3=self.w3, 
                                                        save_erc20_txn_to_db=save_erc20_txn_to_db)
        self.alert_manager = AlertManager()
        self.save_alert_db = save_alert_db
            
    async def process_transaction(self, txn):
        """
        Process a single transaction and its alerts.

        Args:
            tx: The transaction to process.

        Returns:
            detailed_txn: DetailedTransaction
            alerts: List[AlertData]
        """
        try:
            detailed_txn = self.transaction_analyzer.analyze_transaction(txn)
            alerts = []
            if detailed_txn:
                alerts = await self.alert_manager.check_alerts_async(detailed_txn)
            return txn.hash.hex(), detailed_txn, alerts
        except Exception as e:
            logger.error(f"Error processing transaction {txn.hash.hex()}: {e}")
            return txn.hash.hex(), None, []
    
    @profile_async("profiler")
    async def process_block(self, block_input: Union[int, Dict]):
        """
        Process a single block and its transactions.

        Args:
            block_input: Either a block number (int) or a block object (Dict) with full transaction data

        Returns:
            Tuple[int, Dict, List]: (block_number, transactions_dict, alerts_list)

        Raises:
            ValueError: If the block is not found or invalid
        """
        try:
            # Handle block input
            if isinstance(block_input, int):
                block = await self.block_fetcher.fetch_block_by_number(block_input)
                if not block:
                    raise ValueError(f"Block {block_input} not found")
            else:
                block = block_input
                
            # Validate block has transactions
            if 'transactions' not in block:
                raise ValueError(f"Block {block.get('number', 'unknown')} has no transactions")

            # Process transactions
            tasks = [self.process_transaction(tx) for tx in block['transactions']]
            results = await asyncio.gather(*tasks)

            block_transactions = {}
            block_alerts = []
            for txn_hash, txn_result, alerts in results:
                block_transactions[txn_hash] = txn_result
                block_alerts.extend(alerts)
           
            return block['number'], block_transactions, block_alerts
            
        except Exception as e:
            logger.error(f"Error processing block: {str(e)}")
            raise

    @profile_async("profiler")
    async def save_alerts(self, block_alerts):
        if self.save_alert_db and len(block_alerts) > 0:
            async with AlertDB() as alert_db:
                await alert_db.add_alerts(block_alerts)

    async def process_latest_block(self):
        """
        Process the most recent block on the Ethereum network.

        Returns:
            Dict[str, Any]: A dictionary containing the processed latest block data.
        """
        latest_block_number = self.w3.eth.get_block_number()
        return await self.process_block(latest_block_number)

    async def process_block_range(self, start_block: int = None, end_block: int = None, block_range: int = 10000):
        """
        Process a range of blocks.
        """
        if start_block is None:
            end_block = self.w3.eth.get_block_number()
            start_block = max(0, end_block - block_range + 1)
        
        processed_blocks = {}
        
        async with AlertDB() as alert_db:
            for block_number in range(start_block, end_block + 1):
                try:
                    block_number, block_transactions, block_alerts = await self.process_block(block_number)
                    processed_blocks[block_number] = block_transactions
                    await alert_db.add_alerts(block_alerts)
                    if block_number % 100 == 0:  # Log every 100 blocks
                        logger.info(f"Processed block {block_number}")
                except Exception as e:
                    logger.error(f"Error processing block {block_number}: {str(e)}")
                    processed_blocks[block_number] = e

            await alert_db.flush_cache()

        logger.info(f"Processed blocks from {start_block} to {end_block}")
        return processed_blocks
