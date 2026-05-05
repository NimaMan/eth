"""Rust-backed transaction batch processor compatibility layer."""

from __future__ import annotations

import asyncio
from typing import Any, Dict, Iterable, List, Optional

from eth_data.pyreth_client import PyrethClient
from eth_data.tx_processor.pyreth_tx_processor import PyRethTransactionProcessor


class TransactionBatchProcessor:
    """Expose Rust block and transaction batch processing through the old API."""

    def __init__(
        self,
        w3=None,
        logger=None,
        calculate_address_balance_changes: bool = True,
        tx_processor: Optional[Any] = None,
        processed_tx_provider: Optional[Any] = None,
    ) -> None:
        self.w3 = w3
        self.logger = logger
        self.calculate_address_balance_changes = calculate_address_balance_changes

        client = None
        if tx_processor is None or processed_tx_provider is None:
            client = PyrethClient.instance()
        self.tx_processor = tx_processor or client.tx_processor()
        self.processed_tx_provider = processed_tx_provider or client.processed_tx_provider()
        self.transaction_processor = PyRethTransactionProcessor(
            w3=w3,
            calculate_address_balance_changes=calculate_address_balance_changes,
            tx_processor=self.tx_processor,
            processed_tx_provider=self.processed_tx_provider,
        )

    async def _process_single_transaction(
        self,
        transaction: Any,
        receipt: Optional[Dict[str, Any]] = None,
        trace: Optional[Dict[str, Any]] = None,
        tx_hash: Optional[str] = None,
        block_timestamp: int = 0,
    ) -> Any:
        _ = trace, block_timestamp
        return await self.transaction_processor.process_transaction_async(
            transaction if transaction is not None else {"hash": tx_hash},
            receipt=receipt,
        )

    async def process_block_transactions(
        self,
        block_number: int,
        transactions: Optional[List[Dict[str, Any]]] = None,
        block_timestamp: int = 0,
    ) -> List[Any]:
        _ = block_timestamp
        block = await asyncio.to_thread(
            self.processed_tx_provider.process_block,
            int(block_number),
        )
        processed = list(getattr(block, "transactions", []))

        if transactions is None:
            return sorted(processed, key=lambda tx: int(getattr(tx, "tx_index", 0)))

        wanted_hashes = {
            PyRethTransactionProcessor._extract_tx_hash(tx)
            for tx in transactions
            if tx is not None
        }
        return [
            tx
            for tx in sorted(processed, key=lambda item: int(getattr(item, "tx_index", 0)))
            if PyRethTransactionProcessor._normalize_tx_hash(getattr(tx, "hash", "")) in wanted_hashes
        ]

    async def process_batch_with_fetched_data(
        self,
        blocks: Dict[int, Any],
        block_data: Dict[int, Dict],
    ) -> Dict[int, List[Any]]:
        _ = block_data
        return {
            block_num: await self.process_block_transactions(block_num)
            for block_num in blocks
        }

    async def process_transaction_list(self, tx_hashes: Iterable[str]) -> List[Any]:
        return await self.transaction_processor.process_transaction_hash_list_async(tx_hashes)


__all__ = ["TransactionBatchProcessor"]
