"""
Transaction Batch Analyzer for Ethereum Blockchain

Objective:
---------
Provides optimized batch processing of transactions by:
1. Fetching transaction receipts and traces in batches
2. Processing multiple transactions in parallel
3. Minimizing RPC calls through batch requests

Key Components:
-------------
1. Batch Data Fetching:
   - Groups RPC calls for receipts and traces
   - Executes parallel requests for different data types
   
2. Parallel Processing:
   - Uses thread pool for CPU-bound analysis tasks
   - Maintains transaction ordering within blocks
"""

from web3 import Web3
from typing import List, Dict, Any, Union
import asyncio
from eth_block_processor.data_models.txn_models import ProcessedTransaction
from eth_block_processor.txn.txn_processor import TransactionProcessor
from eth_block_processor.txn.txn_data_fetcher import BatchTransactionDataFetcher
from eth_block_processor.tokens.erc20_token_txn_store import ERC20TransactionDB
from eth_block_processor.utils.logger import get_logger


class TransactionBatchProcessor:
    def __init__(self, w3: Web3 = None, save_erc20_txn_to_db: bool = False, logger=None):
        if w3 is None:
            w3 = Web3(Web3.HTTPProvider("http://127.0.0.1:8545"))
        self.w3 = w3
        self.save_erc20_txn_to_db = save_erc20_txn_to_db
        self.transaction_processor = TransactionProcessor(w3=w3, save_erc20_txn_to_db=save_erc20_txn_to_db)
        self.batch_data_fetcher = BatchTransactionDataFetcher(w3=w3)
        if logger is None:
            logger = get_logger(name="txn_processor")
        self.logger = logger

    async def _process_single_transaction(self, 
                                             transaction: Dict[str, Any], 
                                             receipt: Dict[str, Any] = None,
                                             trace: Dict[str, Any] = None,
                                             txn_hash: str = None) -> ProcessedTransaction:
        """Safely analyze a single transaction with error handling"""
        try:
            processed_txn = await self.transaction_processor.process_transaction_async(
                transaction=transaction,
                receipt=receipt,
                trace=trace
            )
            return processed_txn
        except Exception as e:
            self.logger.error(f"{__name__} Error analyzing transaction {txn_hash}: {str(e)}")
            e.txn_hash = txn_hash  # Attach txn_hash to exception for tracking
            raise
    
    async def process_block_transactions(self, block_number: int, transactions: List[Dict[str, Any]]) -> List[ProcessedTransaction]:
        """
        Analyzes all transactions in a block using batch processing
        """
        results = await self._process_transaction_batch_asyncio(block_number, transactions)
        if self.save_erc20_txn_to_db and results:
            with ERC20TransactionDB() as db:
                db.add_transactions_batch(results)

        return results

    async def _process_transaction_batch_asyncio(self, block_number: int, transactions: List[Dict[str, Any]]) -> List[ProcessedTransaction]:
        """Process transactions using asyncio for comparison with thread pool version"""
        receipt_map, trace_map = await self.batch_data_fetcher.fetch_block_data(block_number)
        # Pre-process transaction data
        batch_data = []
        for txn in transactions:
            txn_hash = f"0x{txn['hash'].hex()}" if not isinstance(txn['hash'], str) else txn['hash']
            receipt = receipt_map.get(txn_hash)
            trace = trace_map.get(txn_hash)
            if receipt:
                batch_data.append((txn, receipt, trace, txn_hash))
        
        # Process transactions concurrently using asyncio
        tasks = []
        for txn, receipt, trace, txn_hash in batch_data:
            task = asyncio.create_task(
                self._process_single_transaction(
                    transaction=txn,
                    receipt=receipt,
                    trace=trace,
                    txn_hash=txn_hash
                )
            )
            tasks.append(task)
        
        # Wait for all tasks to complete
        results = []
        failed_txns = []
        completed_tasks = await asyncio.gather(*tasks, return_exceptions=True)
        for result in completed_tasks:
            if isinstance(result, Exception):
                if hasattr(result, 'txn_hash'):
                    failed_txns.append(result.txn_hash)
                continue
            if result is not None:
                results.append(result)
        if failed_txns:
            self.logger.warning(f"{__name__} Failed to process {len(failed_txns)} transactions: {failed_txns}")
        
        return results

    async def process_batch_with_fetched_data(self, blocks: Dict[int, Any], block_data: Dict[int, Dict]) -> Dict[int, List[ProcessedTransaction]]:
        analyzed_blocks = {}
        
        for block_num, block in blocks.items():
            if block_num not in block_data:
                continue
            
            receipts = block_data[block_num]["receipts"]
            traces = block_data[block_num]["traces"]
            batch_data = []
            
            # Match exactly how the working version handles transactions
            for txn in block['transactions']:
                txn_hash = f"0x{txn['hash'].hex()}" if not isinstance(txn['hash'], str) else txn['hash']
                receipt = receipts.get(txn_hash)
                trace = traces.get(txn_hash)
                if receipt:  # Only check receipt like the working version
                    batch_data.append((txn, receipt, trace, txn_hash))
            
            # Process transactions
            tasks = []
            for txn, receipt, trace, txn_hash in batch_data:
                task = self._process_single_transaction(
                    transaction=txn,
                    receipt=receipt,
                    trace=trace,
                    txn_hash=txn_hash
                )
                tasks.append(task)
            
            analyzed_blocks[block_num] = await asyncio.gather(*tasks, return_exceptions=True)
        
        return analyzed_blocks