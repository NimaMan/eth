"""Compatibility facades for retired Python RPC transaction fetchers."""

from __future__ import annotations

from typing import Any, Dict, Iterable, List, Tuple


class TransactionDataFetcher:
    """Return minimal hash-bearing payloads for legacy callers.

    The Rust processor now owns loading receipts, traces, and state from Reth.
    """

    def __init__(self, w3=None):
        self.w3 = w3

    def get_transaction_data(
        self,
        tx_hash: str,
        receipt: bool = True,
        trace: bool = True,
        state_diff: bool = False,
    ) -> Dict[str, Any]:
        _ = receipt, trace, state_diff
        return {
            "transaction": {"hash": tx_hash},
            "receipt": {"transactionHash": tx_hash},
            "trace": None,
            "state_diff": None,
        }

    def get_base_transaction(self, tx_hash: str) -> Dict[str, Any]:
        return {"hash": tx_hash}

    def get_transaction_receipt(self, tx_hash: str) -> Dict[str, Any]:
        return {"transactionHash": tx_hash}

    def get_transaction_trace(self, tx_hash: str) -> None:
        return None

    def get_state_diff(self, tx_hash: str) -> None:
        return None


class TransactionBatchDataFetcher:
    """Legacy no-RPC batch fetcher facade."""

    def __init__(self, w3=None):
        self.w3 = w3

    async def fetch_block_data(self, block_number: int) -> Tuple[Dict[str, Any], Dict[str, Any]]:
        _ = block_number
        return {}, {}

    async def fetch_tx_hash_list(
        self,
        tx_hashes: Iterable[str],
    ) -> Tuple[Dict[str, Any], Dict[str, Any], Dict[str, Any]]:
        transactions = {tx_hash: {"hash": tx_hash} for tx_hash in tx_hashes}
        receipts = {tx_hash: {"transactionHash": tx_hash} for tx_hash in transactions}
        return transactions, receipts, {}


__all__ = ["TransactionDataFetcher", "TransactionBatchDataFetcher"]
