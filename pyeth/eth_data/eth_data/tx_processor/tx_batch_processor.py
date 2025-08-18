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
from eth_data.tx_processor.data_models.txn_models import ProcessedTransaction
from eth_data.tx_processor.tx_processor import TransactionProcessor
from eth_data.tx_processor.tx_data_fetcher import TransactionBatchDataFetcher
from eth_data.utils.logger import get_logger


class TransactionBatchProcessor:
    def __init__(self, w3: Web3 = None, logger=None, calculate_state_changes: bool = False):
        if w3 is None:
            w3 = Web3(Web3.HTTPProvider("http://127.0.0.1:8545"))
        self.w3 = w3
        self.transaction_processor = TransactionProcessor(w3=w3, calculate_state_changes=calculate_state_changes)
        self.batch_data_fetcher = TransactionBatchDataFetcher(w3=w3)
        self.logger = logger

    async def _process_single_transaction(
            self, 
            transaction: Dict[str, Any], 
            receipt: Dict[str, Any] = None,
            trace: Dict[str, Any] = None,
            txn_hash: str = None,
            block_timestamp: int = 0) -> ProcessedTransaction:
        """Safely analyze a single transaction with error handling"""
        try:
            processed_txn = await self.transaction_processor.process_transaction_async(
                transaction=transaction,
                receipt=receipt,
                trace=trace,
                block_timestamp=block_timestamp
            )
            return processed_txn
        except Exception as e:
            if self.logger is not None:
                self.logger.error(f"{__name__} Error analyzing transaction {txn_hash}: {str(e)}")
            e.txn_hash = txn_hash  # Attach txn_hash to exception for tracking
            raise
    
    def _check_rpc_error(self, data: Dict[str, Any], block_number: int) -> bool:
        if 'error' in data:
            if self.logger is not None:
                self.logger.error(f"{__name__} Error fetching block {block_number}: {data['error']}")
            else:
                raise Exception(f"Error fetching block {block_number}: {data['error']}")
        return False

    async def process_block_transactions(self, block_number: int, transactions: List[Dict[str, Any]], block_timestamp: int=0) -> List[ProcessedTransaction]:
        """Process transactions using asyncio for comparison with thread pool version"""
        receipt_map, trace_map = await self.batch_data_fetcher.fetch_block_data(block_number)
        if self._check_rpc_error(receipt_map, block_number):
            receipt_map = {}
        if self._check_rpc_error(trace_map, block_number):
            trace_map = {}
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
                    txn_hash=txn_hash,
                    block_timestamp=block_timestamp
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
            if self.logger is not None:
                self.logger.warning(f"{__name__} Failed to process {len(failed_txns)} transactions: {failed_txns}")
        
        return results

    async def process_batch_with_fetched_data(self, blocks: Dict[int, Any], block_data: Dict[int, Dict]) -> Dict[int, List[ProcessedTransaction]]:
        processed_blocks = {}
        
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
            
            processed_blocks[block_num] = await asyncio.gather(*tasks, return_exceptions=True)
        
        return processed_blocks

    async def process_transaction_list(self, tx_hashes: List[str]) -> List[ProcessedTransaction]:
        """
        Process a list of transactions from any blocks using optimized batch fetching
        
        Args:
            tx_hashes: List of transaction hashes to process
            
        Returns:
            List of processed transactions
        """
        if not tx_hashes:
            return []
            
        # Fetch all transaction data in one optimized operation
        start_time = asyncio.get_event_loop().time()
        transaction_map, receipt_map, trace_map = await self.batch_data_fetcher.fetch_transaction_list_data(tx_hashes)
        fetch_time = asyncio.get_event_loop().time() - start_time
        
        if self.logger:
            self.logger.info(f"Fetched {len(transaction_map)}/{len(tx_hashes)} transactions in {fetch_time:.4f}s")
        
        # Prepare batch data from maps
        batch_data = []
        for tx_hash in tx_hashes:
            transaction = transaction_map.get(tx_hash)
            receipt = receipt_map.get(tx_hash)
            trace = trace_map.get(tx_hash)
            
            if transaction and receipt:
                batch_data.append((transaction, receipt, trace, tx_hash))
        
        # Process transactions concurrently
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
        
        if tasks:
            completed_tasks = await asyncio.gather(*tasks, return_exceptions=True)
            for result in completed_tasks:
                if isinstance(result, Exception):
                    if hasattr(result, 'txn_hash'):
                        failed_txns.append(result.txn_hash)
                    continue
                if result is not None:
                    results.append(result)
        
        if failed_txns and self.logger:
            self.logger.warning(f"{__name__} Failed to process {len(failed_txns)} transactions: {failed_txns}")
        
        if self.logger:
            process_time = asyncio.get_event_loop().time() - start_time
            self.logger.info(f"Processed {len(results)}/{len(tx_hashes)} transactions in {process_time:.4f}s")
        
        return results