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

from hexbytes import HexBytes
from web3 import Web3
from typing import List, Dict, Any, Union
import asyncio
import concurrent.futures
from eth_block_processor.data_models.txn_models import DetailedTransaction
from eth_block_processor.txn.txn_analyzer import TransactionAnalyzer
from eth_block_processor.txn.txn_data_fetcher import BatchTransactionDataFetcher
from eth_block_processor.tokens.erc20_token_txn_store import ERC20TransactionDB
import time
from eth_block_processor.utils.logger import get_logger


logger = get_logger()


class TransactionBatchAnalyzer:
    def __init__(self, w3: Web3 = None, save_erc20_txn_to_db: bool = False):
        if w3 is None:
            w3 = Web3(Web3.HTTPProvider("http://127.0.0.1:8545"))
        self.w3 = w3
        self.save_erc20_txn_to_db = save_erc20_txn_to_db
        self.transaction_analyzer = TransactionAnalyzer(w3=w3, save_erc20_txn_to_db=save_erc20_txn_to_db)
        self.batch_data_fetcher = BatchTransactionDataFetcher(w3=w3)

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
        # Use 8 workers as it shows optimal performance
        with concurrent.futures.ThreadPoolExecutor(max_workers=8) as executor:
            futures = []
            for idx, txn in enumerate(transactions):
                if not isinstance(txn['hash'], str):
                    txn_hash = f"0x{txn['hash'].hex()}"
                else:
                    txn_hash = txn['hash']
                receipt = receipt_map.get(txn_hash)    
                trace = trace_map.get(txn_hash)
                
                future = executor.submit(
                    self.transaction_analyzer.analyze_transaction,
                    transaction=txn,
                    receipt=receipt,
                    trace=trace,
                )
                futures.append((idx, future))
            
            # Process completed futures in order
            for idx, future in futures:
                try:
                    result = future.result()
                    if result is not None:
                        results.append(result)
                except Exception as e:
                    logger.error(f"Error processing transaction at index {idx} with hash {txn_hash}: {str(e)}")
        

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
                batch_data.append((txn, receipt, trace))
        
        # Process transactions concurrently using asyncio
        tasks = []
        for txn, receipt, trace in batch_data:
            task = asyncio.create_task(
                self._analyze_single_transaction(
                    transaction=txn,
                    receipt=receipt,
                    trace=trace
                )
            )
            tasks.append(task)
        
        # Wait for all tasks to complete
        results = []
        try:
            completed_tasks = await asyncio.gather(*tasks)
            for result in completed_tasks:
                if result is not None:
                    results.append(result)
        except Exception as e:
            logger.error(f"Error in async processing: {str(e)}")
        
        return results

    async def _analyze_single_transaction(self, 
                                        transaction: Dict[str, Any], 
                                        receipt: Dict[str, Any] = None,
                                        trace: Dict[str, Any] = None,
                                        state_diff: bool = False) -> DetailedTransaction:
        """Analyze a single transaction using pre-fetched data"""
        try:
            return await self.transaction_analyzer.analyze_transaction_async(
                transaction=transaction,
                receipt=receipt,
                trace=trace,
                state_diff=state_diff
            )
        except Exception as e:
            logger.error(f"Error analyzing transaction {transaction['hash']}: {str(e)}")
            raise
    