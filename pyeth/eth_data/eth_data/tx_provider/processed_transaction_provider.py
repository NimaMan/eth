"""Single PyReth-backed processed transaction provider."""

from __future__ import annotations

import asyncio
from collections import OrderedDict
from typing import Any, Dict, Iterable, List, Optional

from eth_data.database.db_fetchers.tx_meta_data_fetcher import TxMetaDataFetcher
from eth_data.pyreth_client import PyrethClient
from eth_data.tx_processor.pyreth_tx_processor import PyRethTransactionProcessor


class ProcessedTransactionProvider:
    """Cache and expose processed transactions from the Rust/PyReth provider."""

    def __init__(
        self,
        w3=None,
        logger=None,
        cache_size: int = 1000,
        tx_processor: Optional[Any] = None,
        processed_tx_provider: Optional[Any] = None,
        tx_meta_data_fetcher: Optional[Any] = None,
        **legacy_kwargs: Any,
    ) -> None:
        _ = legacy_kwargs
        self.w3 = w3
        self.logger = logger
        self.cache_size = cache_size

        client = None
        if tx_processor is None or processed_tx_provider is None:
            client = PyrethClient.instance()
        self.processed_tx_provider = processed_tx_provider or client.processed_tx_provider()
        self.transaction_processor = PyRethTransactionProcessor(
            w3=w3,
            tx_processor=tx_processor or client.tx_processor(),
            processed_tx_provider=self.processed_tx_provider,
        )
        self.tx_meta_data_fetcher = tx_meta_data_fetcher or TxMetaDataFetcher(logger=logger)

        self.processed_block_cache: OrderedDict[int, List[Any]] = OrderedDict()
        self.processed_tx_cache: OrderedDict[str, Any] = OrderedDict()

    def _manage_cache_size(self) -> None:
        while len(self.processed_block_cache) > self.cache_size:
            self.processed_block_cache.popitem(last=False)
        while len(self.processed_tx_cache) > self.cache_size * 10:
            self.processed_tx_cache.popitem(last=False)

    def _update_tx_cache(self, processed_txs: Iterable[Any]) -> None:
        for tx in processed_txs:
            tx_hash = self._get_tx_hash(tx)
            if not tx_hash:
                continue
            self.processed_tx_cache.pop(tx_hash, None)
            self.processed_tx_cache[tx_hash] = tx
        self._manage_cache_size()

    async def get_processed_transactions_from_block_numbers(
        self,
        target_block_numbers: Iterable[int],
    ) -> Dict[int, List[Any]]:
        results: Dict[int, List[Any]] = {}
        uncached_blocks: List[int] = []

        for block_number in sorted({int(block) for block in target_block_numbers}):
            if block_number in self.processed_block_cache:
                results[block_number] = self.processed_block_cache[block_number]
                self.processed_block_cache.move_to_end(block_number)
            else:
                uncached_blocks.append(block_number)

        for block_number in uncached_blocks:
            try:
                block = await asyncio.to_thread(
                    self.processed_tx_provider.process_block,
                    block_number,
                )
                transactions = sorted(
                    list(getattr(block, "transactions", [])),
                    key=lambda tx: int(self._get_tx_field(tx, "tx_index", 0) or 0),
                )
            except Exception as exc:
                if self.logger:
                    self.logger.error(
                        "Failed to process block %s: %s",
                        block_number,
                        exc,
                        exc_info=True,
                    )
                transactions = []

            self.processed_block_cache[block_number] = transactions
            self._update_tx_cache(transactions)
            results[block_number] = transactions

        self._manage_cache_size()
        return results

    async def get_processed_transactions_from_tx_hashes(
        self,
        tx_hashes: Iterable[str],
    ) -> Dict[str, Any]:
        results: Dict[str, Any] = {}
        uncached: List[str] = []

        for tx_hash in tx_hashes:
            tx_hash = self.transaction_processor._normalize_tx_hash(tx_hash)
            if tx_hash in self.processed_tx_cache:
                results[tx_hash] = self.processed_tx_cache[tx_hash]
                self.processed_tx_cache.move_to_end(tx_hash)
            else:
                uncached.append(tx_hash)

        if not uncached:
            return results

        transactions = await self.transaction_processor.process_transaction_hash_list_async(uncached)
        self._update_tx_cache(transactions)
        for tx in transactions:
            tx_hash = self._get_tx_hash(tx)
            if tx_hash:
                results[tx_hash] = tx

        return results

    def get_processed_tx(self, tx_hash: str, include_trace: bool = True) -> Any:
        _ = include_trace
        tx_hash = self.transaction_processor._normalize_tx_hash(tx_hash)
        if tx_hash in self.processed_tx_cache:
            self.processed_tx_cache.move_to_end(tx_hash)
            return self.processed_tx_cache[tx_hash]
        tx = self.transaction_processor.process_transaction_by_hash(tx_hash)
        self._update_tx_cache([tx])
        return tx

    async def get_processed_tx_async(self, tx_hash: str, include_trace: bool = True) -> Any:
        return await asyncio.to_thread(self.get_processed_tx, tx_hash, include_trace)

    def get_processed_tx_batch(
        self,
        tx_hashes: Iterable[str],
        include_trace: bool = True,
    ) -> Dict[str, Any]:
        _ = include_trace
        return self._run_async(self.get_processed_transactions_from_tx_hashes(tx_hashes))

    async def get_processed_tx_batch_async(
        self,
        tx_hashes: Iterable[str],
        include_trace: bool = True,
    ) -> Dict[str, Any]:
        _ = include_trace
        return await self.get_processed_transactions_from_tx_hashes(tx_hashes)

    def get_block_transactions(
        self,
        block_number: int,
        include_trace: bool = True,
    ) -> List[Any]:
        _ = include_trace
        return self._run_async(self.get_processed_transactions_from_block_numbers([block_number]))[
            int(block_number)
        ]

    async def get_block_transactions_async(
        self,
        block_number: int,
        include_trace: bool = True,
    ) -> List[Any]:
        _ = include_trace
        blocks = await self.get_processed_transactions_from_block_numbers([block_number])
        return blocks[int(block_number)]

    def get_transaction_by_position(self, block_number: int, tx_index: int) -> Any:
        transactions = self.get_block_transactions(block_number)
        target_index = int(tx_index)
        for tx in transactions:
            if int(self._get_tx_field(tx, "tx_index", -1) or -1) == target_index:
                return tx
        raise ValueError(f"Transaction index {tx_index} out of range for block {block_number}")

    def fetch_address_processed_transactions(
        self,
        address: str,
        start_block: Optional[int] = None,
        end_block: Optional[int] = None,
        num_blocks: Optional[int] = None,
    ) -> Dict[str, Any]:
        try:
            tx_data = self.tx_meta_data_fetcher.get_tx_hashes_and_blocks_for_address(
                address,
                start_block,
                end_block,
                num_blocks,
            )
        except Exception as exc:
            if self.logger:
                self.logger.error(
                    "Failed to fetch tx hashes for %s: %s",
                    address,
                    exc,
                    exc_info=True,
                )
            return {}

        if not tx_data:
            return {}

        tx_hashes = [item[0] for item in tx_data]
        return self.get_processed_tx_batch(tx_hashes)

    async def load_blocks_into_cache(self, block_numbers: Iterable[int]) -> None:
        await self.get_processed_transactions_from_block_numbers(block_numbers)

    def get_cache_statistics(self) -> Dict[str, Any]:
        return {
            "backend": "Rust/PyReth",
            "block_cache_size": len(self.processed_block_cache),
            "tx_cache_size": len(self.processed_tx_cache),
            "max_cache_size": self.cache_size,
        }

    def close(self) -> None:
        closer = getattr(self.tx_meta_data_fetcher, "close", None)
        if closer:
            closer()
        self.processed_block_cache.clear()
        self.processed_tx_cache.clear()

    @staticmethod
    def _get_tx_field(tx: Any, key: str, default: Any = None) -> Any:
        if isinstance(tx, dict):
            return tx.get(key, default)
        return getattr(tx, key, default)

    @classmethod
    def _get_tx_hash(cls, tx: Any) -> Optional[str]:
        return cls._get_tx_field(tx, "hash") or cls._get_tx_field(tx, "tx_hash")

    @staticmethod
    def _run_async(coro):
        try:
            loop = asyncio.get_event_loop()
        except RuntimeError:
            loop = asyncio.new_event_loop()
            asyncio.set_event_loop(loop)

        if not loop.is_running():
            return loop.run_until_complete(coro)

        import concurrent.futures

        def run_in_new_loop():
            new_loop = asyncio.new_event_loop()
            try:
                return new_loop.run_until_complete(coro)
            finally:
                new_loop.close()

        with concurrent.futures.ThreadPoolExecutor(max_workers=1) as executor:
            return executor.submit(run_in_new_loop).result()


ProcessedTxProvider = ProcessedTransactionProvider
RustProcessedTransactionProvider = ProcessedTransactionProvider

__all__ = [
    "ProcessedTransactionProvider",
    "ProcessedTxProvider",
    "RustProcessedTransactionProvider",
]
