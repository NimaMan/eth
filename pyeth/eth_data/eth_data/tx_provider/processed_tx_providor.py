"""Compatibility provider backed by PyReth/Rust transaction processing."""

from __future__ import annotations

import asyncio
from typing import Dict, Iterable, List

from eth_data.blockchain.block_processor import BlockProcessor
from eth_data.pyreth_client import PyrethClient
from eth_data.tx_processor.pyreth_tx_processor import PyRethTransactionProcessor


class ProcessedTxProvider:
    """Legacy provider API using the Rust processed transaction provider."""

    def __init__(
        self,
        w3=None,
        calculate_state_changes: bool = True,
        eth_state_change_threshold: float = 0,
        logger=None,
    ) -> None:
        self.w3 = w3
        self.logger = logger
        self.calculate_state_changes = calculate_state_changes

        client = PyrethClient.instance()
        self.processed_tx_provider = client.processed_tx_provider()
        self.tx_processor = PyRethTransactionProcessor(
            w3=w3,
            calculate_address_balance_changes=calculate_state_changes,
            eth_state_change_threshold=eth_state_change_threshold,
            tx_processor=client.tx_processor(),
            processed_tx_provider=self.processed_tx_provider,
        )
        self.block_processor = BlockProcessor(
            w3=w3,
            logger=logger,
            processed_tx_provider=self.processed_tx_provider,
        )

    def get_processed_tx(self, tx_hash: str, include_trace: bool = True):
        _ = include_trace
        return self.tx_processor.process_transaction_by_hash(tx_hash)

    async def get_processed_tx_async(self, tx_hash: str, include_trace: bool = True):
        _ = include_trace
        return await asyncio.to_thread(self.get_processed_tx, tx_hash)

    def get_processed_tx_batch(
        self,
        tx_hashes: Iterable[str],
        include_trace: bool = True,
    ) -> Dict[str, object]:
        _ = include_trace
        transactions = self.tx_processor.process_transaction_hash_list(tx_hashes)
        return {tx.hash: tx for tx in transactions}

    async def get_processed_tx_batch_async(
        self,
        tx_hashes: Iterable[str],
        include_trace: bool = True,
    ) -> Dict[str, object]:
        _ = include_trace
        return await asyncio.to_thread(self.get_processed_tx_batch, list(tx_hashes))

    def get_block_transactions(
        self,
        block_number: int,
        include_trace: bool = True,
    ) -> List[object]:
        _ = include_trace
        block = self.processed_tx_provider.process_block(int(block_number))
        return sorted(block.transactions, key=lambda tx: int(tx.tx_index))

    async def get_block_transactions_async(
        self,
        block_number: int,
        include_trace: bool = True,
    ) -> List[object]:
        _ = include_trace
        return await asyncio.to_thread(self.get_block_transactions, block_number)

    def get_transaction_by_position(self, block_number: int, tx_index: int):
        transactions = self.get_block_transactions(block_number)
        try:
            return transactions[int(tx_index)]
        except IndexError as exc:
            raise ValueError(
                f"Transaction index {tx_index} out of range for block {block_number}"
            ) from exc


__all__ = ["ProcessedTxProvider"]
