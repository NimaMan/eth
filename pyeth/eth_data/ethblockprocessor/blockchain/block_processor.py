"""
BlockProcessor Module

This module provides the BlockProcessor class, which is responsible for processing
Ethereum blocks and their transactions. It combines the functionality of BlockFetcher
and TransactionAnalyzer to provide comprehensive block and transaction analysis.

Objectives:
1. Process individual Ethereum blocks, including all transactions within them.
2. Analyze the latest block in real-time.
3. Process a range of blocks for historical analysis or catching up after downtime.
"""

from web3 import Web3
import asyncio

from ethblockprocessor.txn.txn_analyzer import TransactionAnalyzer
from ethblockprocessor.blockchain.block_fetcher import BlockFetcher
from ethblockprocessor.alert.alert_manager import AlertManager
from ethblockprocessor.alert.alert_db import AlertDB
from ethblockprocessor.utils.profiler import profile
from ethblockprocessor.utils.logger import get_logger


logger = get_logger()


class BlockProcessor:
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

    async def process_block(self, block_number: int):
        """
        Process a single block and its transactions.

        Args:
            block_number (int): The number of the block to process.

        Returns:
            int: The block number processed.

        Raises:
            ValueError: If the specified block is not found.
        """
        block = await self.block_fetcher.fetch_block_by_number(block_number)
        if not block:
            raise ValueError(f"Block {block_number} not found")

        tasks = [self.process_transaction(tx) for tx in block['transactions']]
        results = await asyncio.gather(*tasks)

        block_transactions = {}
        block_alerts = []
        for txn_hash, txn_result, alerts in results:
            block_transactions[txn_hash] = txn_result
            block_alerts.extend(alerts)
       
        return block['number'], block_transactions, block_alerts

    @profile
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
