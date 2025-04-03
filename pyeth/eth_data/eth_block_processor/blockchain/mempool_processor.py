"""
Ethereum Mempool Processor with Integrated State Diff Analysis

Objective:
----------
1. Continuously poll the Ethereum mempool for pending transactions using MempoolDataFetcher
2. Extract simulated state changes (state diffs) for new transactions
3. Provide access to transactions and their state diffs through a simple API
4. Track basic metrics about mempool transactions
"""

import asyncio
import time
from typing import Dict, List, Any
from web3 import Web3
from collections import OrderedDict

from eth_block_processor.txn.mempool_tx_state_diff_processor import StateDiffProcessor
from eth_block_processor.txn.mempool_data_fetcher import MempoolDataFetcher
from eth_token.utils.logger import get_logger


class MempoolTxCache:
    """
    A limited-size queue for transactions that maintains FIFO behaviorwhile providing O(1) lookup by transaction hash and aggregates state diffs by affected address.
    """
    
    def __init__(self, max_size: int = 2000000):
        """
        Initialize a limited queue.
        
        Args:
            max_size: Maximum number of items to store.
        """
        self.max_size = max_size
        self._items = OrderedDict()  # transaction hash -> transaction data
        self._first_seen = {} # transaction hash -> timestamp
        self._state_diffs = {} # address -> aggregated state diff

    def add(self, item_hash: str, item: Dict) -> None:
        """
        Add an item (transaction) to the queue.
        If the queue is full, the oldest item is removed.
        
        Args:
            item_hash: Transaction hash.
            item: Transaction data.
        """
        if len(self._items) >= self.max_size:
            oldest_hash, _ = self._items.popitem(last=False)
            if oldest_hash in self._first_seen:
                del self._first_seen[oldest_hash]
            # We do not remove entries from _state_diffs here since they are aggregated.
        self._items[item_hash] = item
        if item_hash not in self._first_seen:
            self._first_seen[item_hash] = time.time()

    def add_state_diff(self, state_diff: Dict) -> None:
        """
        Update the aggregated state diffs for affected addresses.
        For each address in the provided state_diff, if it does not exist in _state_diffs,
        add it; otherwise update its 'after' value and sum the new change.

        Args:
            state_diff: Dict mapping address -> diff info.
                        Each diff info should contain 'before' and 'after' values. If 'change'
                        is missing, it is calculated as (after - before).
        """
        for address, diff in state_diff.items():
            new_before = diff.get('before')
            new_after = diff.get('after')
            new_change = diff.get('change')
            if address not in self._state_diffs:
                self._state_diffs[address] = {
                    'before': new_before,
                    'after': new_after,
                    'change': new_change
                }
            else:
                entry = self._state_diffs[address]
                entry['after'] = new_after
                entry['change'] += new_change

    def get(self, item_hash: str) -> Dict:
        """Retrieve an item (transaction) by its hash."""
        return self._items.get(item_hash)

    def get_state_diff(self, address: str) -> Dict:
        """
        Retrieve the aggregated state diff for a specified address.
        
        Args:
            address: The Ethereum address.
            
        Returns:
            Aggregated state diff dict or None if not present.
        """
        return self._state_diffs.get(address)

    def remove(self, item_hash: str) -> None:
        """
        Remove an item by its transaction hash.
        Note: This method does not remove aggregated state diffs.
        
        Args:
            item_hash: Transaction hash.
        """
        if item_hash in self._items:
            del self._items[item_hash]
        if item_hash in self._first_seen:
            del self._first_seen[item_hash]

    def get_first_seen(self, item_hash: str) -> float:
        """Return the timestamp when the transaction was first seen."""
        return self._first_seen.get(item_hash)

    def get_all_with_times(self) -> Dict[str, Dict]:
        """
        Return all transactions with their first-seen timestamps.
        """
        result = {}
        for item_hash, item_data in self._items.items():
            first_seen = self._first_seen.get(item_hash)
            entry = {
                'data': item_data,
                'first_seen': first_seen,
                'time_in_queue': time.time() - first_seen if first_seen else None
            }
            result[item_hash] = entry
        return result

    def get_all(self) -> Dict[str, Dict]:
        """Return all stored transactions."""
        return self._items.copy()

    def __len__(self) -> int:
        """Return the number of stored transactions."""
        return len(self._items)

    def __contains__(self, item_hash: str) -> bool:
        """Check if a transaction is in the queue."""
        return item_hash in self._items


class MempoolProcessor:
    """
    Core mempool monitoring system that continuously polls for pending transactions
    and immediately extracts their state diffs.
    
    This class implements a high-frequency polling approach to monitor the Ethereum
    mempool, tracking new transactions and their simulated state changes.
    """
    
    def __init__(self, 
                w3: Web3 = None,
                poll_interval: float = 0.5,
                max_transactions: int = 2000000,
                logger=None):
        """
        Initialize the mempool processor.
        
        Args:
            w3: Web3 instance connected to an Ethereum node
            poll_interval: How often to poll for new transactions (seconds)
            max_transactions: Maximum number of transactions to keep in queue
            logger: Logger instance
        """
        self.logger = logger or get_logger("mempool_processor")
        self.w3 = w3 or Web3(Web3.HTTPProvider("http://127.0.0.1:8545"))
        self.poll_interval = poll_interval
        
        # Single queue with state markers
        self.mempool_tx_cache = MempoolTxCache(max_size=max_transactions)
        
        # Tracking hashes we've already seen
        self.seen_hashes = set()
        
        # Mempool data fetcher for retrieving transactions
        self.mempool_fetcher = MempoolDataFetcher(w3=self.w3, logger=self.logger)
        
        # State diff processor
        self.state_diff_processor = StateDiffProcessor(w3=self.w3, logger=self.logger)
        
        # Control flags and tasks
        self._shutdown_event = asyncio.Event()
        self._processing_task = None
        
        # Metrics
        self.last_metrics_time = time.time()
        
    async def start(self):
        """Start the mempool processor."""
        if self._processing_task is not None and not self._processing_task.done():
            return
        
        self._shutdown_event.clear()
        self._processing_task = asyncio.create_task(self._process_mempool())
        
        if self.logger:
            self.logger.info("Started mempool processor")
    
    async def stop(self):
        """Stop the mempool processor."""
        if self._processing_task is None or self._processing_task.done():
            return
        
        self._shutdown_event.set()
        try:
            await self._processing_task
        except asyncio.CancelledError:
            pass
        
        if self.logger:
            self.logger.info("Stopped mempool processor")
    
    async def _process_mempool(self):
        """
        Combined task that polls the mempool for new transactions 
        and immediately processes their state diffs.
        """
        if self.logger:
            self.logger.info("Starting mempool polling and state diff processing")
        
        while not self._shutdown_event.is_set():
            try:                
                # Get transactions
                pending_txs, queued_txs = await self.mempool_fetcher.get_mempool_transactions()
                
                # Process with state markers
                await self._process_new_transactions(pending_txs, 'pending')
                await self._process_new_transactions(queued_txs, 'queued')
                
                #await self._log_metrics(pending_txs, queued_txs)
                # Wait before next poll
                await asyncio.sleep(self.poll_interval)
                
            except Exception as e:
                if self.logger:
                    self.logger.error(f"Error in mempool processing: {e}")
                await asyncio.sleep(1)  # Wait longer after error
    
    async def _process_new_transactions(self, transactions, state):
        """Process transactions with their state (pending or queued)."""
        for txn in transactions:
            tx_hash = txn.get('hash')
            if not tx_hash or tx_hash in self.seen_hashes:
                continue
                
            # Add state marker
            txn['tx_state'] = state
            self.seen_hashes.add(tx_hash)
            self.mempool_tx_cache.add(tx_hash, txn)
            
            try:
                state_diff = await self.state_diff_processor.extract_state_diffs(txn)
                if state_diff:
                    self.mempool_tx_cache.add_state_diff(state_diff)
            except Exception as e:
                error_msg = str(e)
                # More informative error logging
                if "Response is too big" in error_msg:
                    self.logger.debug(f"Transaction {tx_hash} produced too large response for simulation")
                elif "insufficient funds" in error_msg:
                    self.logger.debug(f"Transaction {tx_hash} has insufficient funds for execution")
                else:
                    self.logger.debug(
                        f"Error extracting state diff for {tx_hash}: {error_msg}"
                    )
        
    def get_transactions(self, state=None):
        """
        Get transactions, optionally filtered by state.
        
        Args:
            state: Optional filter ('pending', 'queued', or None for all)
        """
        all_txns = self.mempool_tx_cache.get_all()
        if state is None:
            return all_txns
        
        return {
            tx_hash: tx_data for tx_hash, tx_data in all_txns.items() 
            if tx_data.get('tx_state') == state
        }
    
    def get_address_state_diffs(self, address):
        """Get the aggregated state diff for a specific address."""
        return self.mempool_tx_cache.get_state_diff(address)
    
    async def _log_metrics(self, pending_txs, queued_txs):
        """Log current mempool metrics with sample state diff."""
        # Basic queue stats
        total_cached = len(self.mempool_tx_cache)
        total_state_diffs = len(self.mempool_tx_cache._state_diffs)
        
        self.logger.info(
            f"MEMPOOL: Cached: {total_cached} txs | "
            f"New pending: {len(pending_txs)} | "
            f"New queued: {len(queued_txs)} | "
            f"Addresses with state diffs: {total_state_diffs}"
        )
        
        # Sample a single transaction with state diff if available
        if self.mempool_tx_cache._state_diffs and total_cached > 0:
            # Get a sample address with state diff
            sample_address = next(iter(self.mempool_tx_cache._state_diffs))
            sample_diff = self.mempool_tx_cache._state_diffs[sample_address]
            
            # Get a sample transaction
            sample_tx_hash = next(iter(self.mempool_tx_cache._items))
            sample_tx = self.mempool_tx_cache._items[sample_tx_hash]
            
            self.logger.info(
                f"SAMPLE: Tx: {sample_tx_hash[:10]}... | "
                f"To: {sample_tx.get('to', 'contract_creation')[:10]}... | "
                f"State diff for {sample_address[:10]}... | "
                f"Before: {sample_diff.get('before')} | "
                f"After: {sample_diff.get('after')} | "
                f"Change: {sample_diff.get('change')}"
            )
