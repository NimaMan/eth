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
import concurrent.futures
from eth_block_processor.data_models.txn_models import DetailedTransaction
from eth_block_processor.txn.txn_analyzer import TransactionAnalyzer
from eth_block_processor.txn.txn_data_fetcher import BatchTransactionDataFetcher
from eth_block_processor.tokens.erc20_token_txn_store import ERC20TransactionDB
from eth_block_processor.utils.logger import get_logger


class TransactionBatchAnalyzer:
    def __init__(self, w3: Web3 = None, save_erc20_txn_to_db: bool = False, logger=None):
        if w3 is None:
            w3 = Web3(Web3.HTTPProvider("http://127.0.0.1:8545"))
        self.w3 = w3
        self.save_erc20_txn_to_db = save_erc20_txn_to_db
        self.transaction_analyzer = TransactionAnalyzer(w3=w3, save_erc20_txn_to_db=save_erc20_txn_to_db)
        self.batch_data_fetcher = BatchTransactionDataFetcher(w3=w3)
        if logger is None:
            logger = logger = get_logger(name="txn_analyzer")
        self.logger = logger

    async def _analyze_single_transaction_safe(self, 
                                             transaction: Dict[str, Any], 
                                             receipt: Dict[str, Any] = None,
                                             trace: Dict[str, Any] = None,
                                             txn_hash: str = None) -> DetailedTransaction:
        """Safely analyze a single transaction with error handling"""
        try:
            return await self.transaction_analyzer.analyze_transaction_async(
                transaction=transaction,
                receipt=receipt,
                trace=trace
            )
        except Exception as e:
            self.logger.error(f"{__name__} Error analyzing transaction {txn_hash}: {str(e)}")
            e.txn_hash = txn_hash  # Attach txn_hash to exception for tracking
            raise
    
    async def analyze_block_transactions(self, block_number: int, transactions: List[Dict[str, Any]], use_asyncio: bool = True) -> List[DetailedTransaction]:
        """
        Analyzes all transactions in a block using batch processing
        Args:
            use_asyncio: If True, uses asyncio version instead of thread pool
        """
        if use_asyncio:
            results = await self._process_transaction_batch_asyncio(block_number, transactions)
        else:
            results = await self._process_transaction_batch_thread_pool(block_number, transactions)
        
        if self.save_erc20_txn_to_db and results:
            with ERC20TransactionDB() as db:
                db.add_transactions_batch(results)

        return results

    async def _process_transaction_batch_thread_pool(self, block_number: int, transactions: List[Dict[str, Any]]) -> List[DetailedTransaction]:
        """Process a batch of transactions with optimized data fetching and timing metrics"""
        
        receipt_map, trace_map = await self.batch_data_fetcher.fetch_block_data(block_number)
        
        results = []
        failed_txns = []
        
        # Use 8 workers as it shows optimal performance
        with concurrent.futures.ThreadPoolExecutor(max_workers=8) as executor:
            futures = []
            for idx, txn in enumerate(transactions):
                txn_hash = f"0x{txn['hash'].hex()}" if not isinstance(txn['hash'], str) else txn['hash']
                receipt = receipt_map.get(txn_hash)    
                trace = trace_map.get(txn_hash)
                
                future = executor.submit(
                    self.transaction_analyzer.analyze_transaction,
                    transaction=txn,
                    receipt=receipt,
                    trace=trace,
                )
                futures.append((idx, txn_hash, future))
            
            # Process completed futures in order
            for idx, txn_hash, future in futures:
                try:
                    result = future.result()
                    if result is not None:
                        results.append(result)
                except Exception as e:
                    failed_txns.append(txn_hash)
                    self.logger.error(f"{__name__} Error processing transaction at index {idx} with hash {txn_hash}: {str(e)}")
                    continue  # Continue with next transaction
        
        if failed_txns:
            self.logger.warning(f"{__name__} Failed to process {len(failed_txns)} transactions: {failed_txns}")
        
        return results

    async def _process_transaction_batch_asyncio(self, block_number: int, transactions: List[Dict[str, Any]]) -> List[DetailedTransaction]:
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
                self._analyze_single_transaction_safe(
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

    async def process_batch_with_fetched_data(self, blocks: Dict[int, Any], block_data: Dict[int, Dict]) -> Dict[int, List[DetailedTransaction]]:
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
                task = self._analyze_single_transaction_safe(
                    transaction=txn,
                    receipt=receipt,
                    trace=trace,
                    txn_hash=txn_hash
                )
                tasks.append(task)
            
            analyzed_blocks[block_num] = await asyncio.gather(*tasks, return_exceptions=True)
        
        return analyzed_blocks