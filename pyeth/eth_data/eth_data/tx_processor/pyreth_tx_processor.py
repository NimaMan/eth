"""PyReth-backed transaction processor compatibility layer."""

from __future__ import annotations

import asyncio
from typing import Any, Dict, Iterable, List, Optional, Set


class PyRethTransactionProcessor:
    """Expose Rust transaction processing through the legacy Python API."""

    def __init__(
        self,
        w3=None,
        calculate_address_balance_changes: bool = False,
        eth_state_change_threshold: float = 0.005,
        tx_processor: Optional[Any] = None,
        processed_tx_provider: Optional[Any] = None,
    ) -> None:
        self.w3 = w3
        self.calculate_address_balance_changes = bool(calculate_address_balance_changes)
        self.eth_state_change_threshold = eth_state_change_threshold

        if tx_processor is not None and processed_tx_provider is not None:
            self.tx_processor = tx_processor
            self.processed_tx_provider = processed_tx_provider
            return

        from eth_data.pyreth_client import PyrethClient

        client = PyrethClient.instance()
        self.tx_processor = tx_processor or client.tx_processor()
        self.processed_tx_provider = processed_tx_provider or client.processed_tx_provider()

    def process_transaction(
        self,
        transaction: Any,
        receipt: Optional[Dict[str, Any]] = None,
        trace: Optional[Dict[str, Any]] = None,
        block_timestamp: int = 0,
        state_diff: bool = False,
    ) -> Any:
        """Process a transaction by hash using Rust/PyReth.

        The legacy Python implementation accepted pre-fetched transaction,
        receipt, and trace dictionaries.  The Rust processor owns loading,
        tracing, and decoding now, so those inputs are only used to recover the
        transaction hash.
        """
        _ = trace, block_timestamp, state_diff
        tx_hash = self._extract_tx_hash(transaction, receipt)
        return self.process_transaction_by_hash(tx_hash)

    async def process_transaction_async(
        self,
        transaction: Any,
        receipt: Optional[Dict[str, Any]] = None,
        trace: Optional[Dict[str, Any]] = None,
        block_timestamp: int = 0,
        state_diff: bool = False,
    ) -> Any:
        return await asyncio.to_thread(
            self.process_transaction,
            transaction,
            receipt,
            trace,
            block_timestamp,
            state_diff,
        )

    def process_transaction_by_hash(self, tx_hash: str) -> Any:
        """Process one transaction through the Rust tx processor."""
        return self.tx_processor.process_transaction_from_hash_with_simulation(
            self._normalize_tx_hash(tx_hash)
        )

    def load_transaction_by_hash(self, tx_hash: str) -> Any:
        """Load one decoded transaction through Rust without extra simulation."""
        return self.tx_processor.load_transaction_from_hash_db_only(
            self._normalize_tx_hash(tx_hash)
        )

    def process_transaction_hash_list(self, tx_hashes: Iterable[str]) -> List[Any]:
        """Process transaction hashes through the Rust batch path."""
        hashes = [self._normalize_tx_hash(tx_hash) for tx_hash in tx_hashes]
        if not hashes:
            return []
        return list(self.tx_processor.process_transaction_hash_list(hashes))

    async def process_transaction_hash_list_async(self, tx_hashes: Iterable[str]) -> List[Any]:
        return await asyncio.to_thread(self.process_transaction_hash_list, list(tx_hashes))

    def needs_trace(self, tx: Dict[str, Any]) -> bool:
        """Legacy helper retained for callers that still branch on it."""
        to_address = tx.get("to") if isinstance(tx, dict) else getattr(tx, "to", None)
        input_data = tx.get("input", "0x") if isinstance(tx, dict) else getattr(tx, "input", "0x")
        return to_address is None or len(str(input_data or "0x")) > 2

    def extend_unique_addresses(
        self,
        from_address,
        to_address,
        internal_transactions,
        unique_addresses,
        erc20_contracts,
        contract_address=None,
    ):
        """Small compatibility helper; canonical address sets come from Rust."""
        addresses: Set[Any] = set(unique_addresses or [])
        contracts: Set[Any] = set(erc20_contracts or [])
        for address in (from_address, to_address, contract_address):
            if address:
                addresses.add(address)
        for internal_tx in internal_transactions or []:
            src = getattr(internal_tx, "from_address", None)
            dst = getattr(internal_tx, "to_address", None)
            if src:
                addresses.add(src)
            if dst:
                addresses.add(dst)
        addresses.update(address for address in contracts if address)
        contracts.discard("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2")
        return contracts, addresses

    @classmethod
    def _extract_tx_hash(
        cls,
        transaction: Any,
        receipt: Optional[Dict[str, Any]] = None,
    ) -> str:
        for source, keys in (
            (transaction, ("hash", "transactionHash", "tx_hash")),
            (receipt or {}, ("transactionHash", "hash", "tx_hash")),
        ):
            if source is None:
                continue
            for key in keys:
                value = cls._get_field(source, key)
                if value:
                    return cls._normalize_tx_hash(value)
        raise ValueError("transaction hash is required for Rust transaction processing")

    @staticmethod
    def _get_field(source: Any, key: str) -> Any:
        if isinstance(source, dict):
            return source.get(key)
        return getattr(source, key, None)

    @staticmethod
    def _normalize_tx_hash(tx_hash: Any) -> str:
        if hasattr(tx_hash, "hex") and not isinstance(tx_hash, str):
            value = tx_hash.hex()
        elif isinstance(tx_hash, bytes):
            value = tx_hash.hex()
        else:
            value = str(tx_hash)
        value = value.strip()
        return value if value.startswith("0x") else f"0x{value}"


__all__ = ["PyRethTransactionProcessor"]
