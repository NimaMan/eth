"""Rust-backed block processor adapter.

This is the Python compatibility layer around the PyReth/Rust block
processor.  Python callers should import ``BlockProcessor`` as before; the
implementation underneath is the Rust processed transaction provider.
"""

from __future__ import annotations

import asyncio
import time
from typing import Any, Dict, Iterable, Mapping, Optional

from tqdm import tqdm

from eth_data.blockchain.block_data_models import BlockHeader, ProcessedBlockResult
from eth_data.database.writers.transaction_writer import TransactionAddresstoTxIndexer


class PyRethBlockFetcher:
    """Small fetcher facade kept for older callers that need latest/header APIs."""

    def __init__(self, processed_tx_provider: Any) -> None:
        self._processed_tx_provider = processed_tx_provider

    async def fetch_latest_block_number(self) -> int:
        return int(await asyncio.to_thread(self._processed_tx_provider.get_latest_block))

    async def fetch_block_by_number(self, block_number: int) -> Dict[str, Any]:
        block = await asyncio.to_thread(
            self._processed_tx_provider.process_block,
            int(block_number),
        )
        header = PyRethBlockProcessor._extract_header_from_block(block)
        return header.to_rpc_dict() if header is not None else {}

    async def close(self) -> None:
        return None


class PyRethBlockProcessor:
    """Expose Rust processed blocks through the legacy Python block API."""

    def __init__(
        self,
        node_url: str = "http://127.0.0.1:8545",
        index_address_txs: bool = False,
        calculate_address_balance_changes: bool = False,
        logger=None,
        w3=None,
        processed_tx_provider: Optional[Any] = None,
        **legacy_kwargs,
    ) -> None:
        _ = legacy_kwargs
        self.node_url = node_url
        self.w3 = w3
        self.index_address_txs = bool(index_address_txs)
        self.calculate_address_balance_changes = bool(calculate_address_balance_changes)
        self.logger = logger

        if processed_tx_provider is not None:
            self.processed_tx_provider = processed_tx_provider
        else:
            from eth_data.pyreth_client import PyrethClient

            self.processed_tx_provider = PyrethClient.instance().processed_tx_provider()

        self.block_fetcher = PyRethBlockFetcher(self.processed_tx_provider)
        self.transaction_writer = (
            TransactionAddresstoTxIndexer() if self.index_address_txs else None
        )

    async def process_block(
        self,
        block_number: int,
        transactions: Optional[Iterable[Any]] = None,
    ) -> ProcessedBlockResult:
        """Process a block with the Rust provider.

        ``transactions`` is accepted for legacy call sites, but ignored.  The
        Rust provider reads canonical block data directly from Reth.
        """
        start_time = time.perf_counter()
        if transactions is not None and self.logger is not None:
            self.logger.debug(
                "Ignoring pre-fetched transactions for block %s; using PyReth/Reth data",
                block_number,
            )

        processed_block = await asyncio.to_thread(
            self.processed_tx_provider.process_block,
            int(block_number),
        )
        result = self._to_processed_block_result(processed_block, int(block_number))

        if self.index_address_txs and self.transaction_writer is not None:
            self.transaction_writer.write_transactions_address_tx(result.transactions)

        if self.logger is not None:
            elapsed = time.perf_counter() - start_time
            self.logger.info(f"{block_number}->{len(result.transactions)}|0 in {elapsed:.2f}s")

        return result

    async def process_block_range(
        self,
        start_block: int,
        end_block: int,
    ) -> Dict[int, ProcessedBlockResult]:
        results: Dict[int, ProcessedBlockResult] = {}
        for block_number in tqdm(
            range(start_block, end_block + 1),
            desc="Processing blocks",
            unit="blocks",
        ):
            try:
                results[block_number] = await self.process_block(block_number)
            except Exception as exc:
                if self.logger is not None:
                    self.logger.error(
                        "%s Error processing block %s: %s",
                        __name__,
                        block_number,
                        exc,
                        exc_info=True,
                    )
        return results

    async def close(self) -> None:
        await self.block_fetcher.close()

    def _extract_block_header(self, block_data: Any) -> Optional[BlockHeader]:
        if isinstance(block_data, Mapping):
            return BlockHeader.from_rpc_dict(dict(block_data))
        return self._extract_header_from_block(block_data)

    @classmethod
    def _to_processed_block_result(
        cls,
        processed_block: Any,
        block_number: int,
    ) -> ProcessedBlockResult:
        transactions = list(cls._get_field(processed_block, "transactions", []))
        header = cls._extract_header_from_block(processed_block)
        if header is None:
            header = BlockHeader(number=BlockHeader.format_hex(block_number))
        return ProcessedBlockResult(transactions=transactions, block_header=header)

    @classmethod
    def _extract_header_from_block(cls, block: Any) -> Optional[BlockHeader]:
        if block is None:
            return None
        return BlockHeader(
            hash=cls._format_optional_hex(cls._get_field(block, "hash")),
            parent_hash=cls._format_optional_hex(cls._get_field(block, "parent_hash")),
            number=BlockHeader.format_hex(cls._get_field(block, "number")),
            gas_limit=BlockHeader.format_hex(cls._get_field(block, "gas_limit")),
            gas_used=BlockHeader.format_hex(cls._get_field(block, "gas_used")),
            timestamp=BlockHeader.format_hex(cls._get_field(block, "timestamp")),
            base_fee_per_gas=BlockHeader.format_hex(
                cls._get_field(block, "base_fee_per_gas")
            ),
        )

    @staticmethod
    def _get_field(value: Any, field: str, default: Any = None) -> Any:
        if isinstance(value, Mapping):
            return value.get(field, default)
        return getattr(value, field, default)

    @staticmethod
    def _format_optional_hex(value: Any) -> Optional[str]:
        if value is None:
            return None
        if isinstance(value, str):
            return value if value.startswith("0x") else f"0x{value}"
        return BlockHeader.format_hex(value)


__all__ = ["PyRethBlockFetcher", "PyRethBlockProcessor"]
