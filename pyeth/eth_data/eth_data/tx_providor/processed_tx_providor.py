"""
ProcessedTransaction Provider

A clean provider that uses existing processors to fetch and process transactions.
No code duplication - just orchestrates the existing components.
"""

from typing import Dict, List, Optional, Any
from web3 import Web3
from eth_data.tx_processor.txn_processor import TransactionProcessor
from eth_data.blockchain.block_processor import BlockProcessor
from eth_data.blockchain.block_fetcher import BlockFetcher
from eth_data.tx_processor.tx_data_fetcher import TransactionDataFetcher
from eth_data.tx_processor.tx_batch_processor import TransactionBatchProcessor
from eth_data.tx_processor.data_models.txn_models import ProcessedTransaction
import logging


class ProcessedTxProvider:
    """Provider for fetching and processing transactions using existing processors."""
    
    def __init__(self, w3: Web3 = None, calculate_state_changes: bool = True, eth_state_change_threshold: float = 0, logger: Optional[logging.Logger] = None):
        if w3 is None:
            self.w3 = Web3(Web3.HTTPProvider("http://127.0.0.1:8545"))
        else:
            self.w3 = w3
            
        self.logger = logger or logging.getLogger(__name__)
        self.calculate_state_changes = calculate_state_changes
        
        # Initialize processors
        self.tx_processor = TransactionProcessor(w3=self.w3, calculate_state_changes=calculate_state_changes, eth_state_change_threshold=eth_state_change_threshold)
        self.block_processor = BlockProcessor(w3=self.w3, logger=self.logger)
        self.block_fetcher = BlockFetcher(node_url="http://127.0.0.1:8545")
        self.tx_data_fetcher = TransactionDataFetcher(w3=self.w3)
        self.batch_processor = TransactionBatchProcessor(w3=self.w3, calculate_state_changes=calculate_state_changes, logger=self.logger)

    def get_processed_tx(self, tx_hash: str, include_trace: bool = True) -> ProcessedTransaction:
        """Get a single processed transaction."""
        txn_data = self.tx_data_fetcher.get_transaction_data(
            tx_hash, 
            receipt=True, 
            trace=include_trace,
            state_diff=self.calculate_state_changes
        )
        
        if not txn_data.get('transaction'):
            raise ValueError(f"Transaction {tx_hash} not found")
            
        processed_tx = self.tx_processor.process_transaction(
            txn_data['transaction'], 
            txn_data['receipt'], 
            txn_data.get('trace'),
            block_timestamp=0
        )
        return processed_tx
    
    async def get_processed_tx_async(self, tx_hash: str, include_trace: bool = True) -> ProcessedTransaction:
        """Get a single processed transaction asynchronously."""
        txn_data = self.tx_data_fetcher.get_transaction_data(
            tx_hash, 
            receipt=True, 
            trace=include_trace,
            state_diff=self.calculate_state_changes
        )
        
        if not txn_data.get('transaction'):
            raise ValueError(f"Transaction {tx_hash} not found")
            
        processed_tx = await self.tx_processor.process_transaction_async(
            txn_data['transaction'], 
            txn_data['receipt'], 
            txn_data.get('trace'),
            state_diff=self.calculate_state_changes
        )
        return processed_tx
    
    def get_processed_tx_batch(self, tx_hashes: List[str], include_trace: bool = True) -> Dict[str, ProcessedTransaction]:
        """Get a batch of processed transactions."""
        # Use the batch processor which handles concurrent fetching
        processed_txs = self.batch_processor.process_transactions(
            tx_hashes,
            include_trace=include_trace
        )
        return processed_txs
    
    async def get_processed_tx_batch_async(self, tx_hashes: List[str], include_trace: bool = True) -> Dict[str, ProcessedTransaction]:
        """Get a batch of processed transactions asynchronously."""
        processed_txs = await self.batch_processor.process_transactions_async(
            tx_hashes,
            include_trace=include_trace,
            state_diff=self.calculate_state_changes
        )
        return processed_txs
    
    def get_block_transactions(self, block_number: int, include_trace: bool = True) -> List[ProcessedTransaction]:
        """Get all processed transactions from a block."""
        # Fetch block
        block = self.block_fetcher.fetch_block(block_number)
        if not block:
            raise ValueError(f"Block {block_number} not found")
        
        # Process block transactions
        processed_block = self.block_processor.process_block(
            block,
            db_write=False,  # Don't write to database
            publish=False     # Don't publish to message queue
        )
        
        return processed_block.transactions if processed_block else []
    
    async def get_block_transactions_async(self, block_number: int, include_trace: bool = True) -> List[ProcessedTransaction]:
        """Get all processed transactions from a block asynchronously."""
        # Fetch block
        block = self.block_fetcher.fetch_block(block_number)
        if not block:
            raise ValueError(f"Block {block_number} not found")
        
        # Get transaction hashes
        tx_hashes = [tx['hash'].hex() if hasattr(tx['hash'], 'hex') else tx['hash'] for tx in block['transactions']]
        
        # Process using batch processor
        processed_txs = await self.get_processed_tx_batch_async(tx_hashes, include_trace=include_trace)
        
        # Return as list in block order
        return [processed_txs[tx_hash] for tx_hash in tx_hashes if tx_hash in processed_txs]
    
    def get_transaction_by_position(self, block_number: int, tx_index: int) -> ProcessedTransaction:
        """Get a processed transaction by its position in a block."""
        block = self.block_fetcher.fetch_block(block_number)
        if not block:
            raise ValueError(f"Block {block_number} not found")
            
        if tx_index >= len(block['transactions']):
            raise ValueError(f"Transaction index {tx_index} out of range for block {block_number}")
            
        tx_hash = block['transactions'][tx_index]['hash']
        if hasattr(tx_hash, 'hex'):
            tx_hash = tx_hash.hex()
            
        return self.get_processed_tx(tx_hash)
    


