"""
Synchronous reader helpers for live data snapshots.
"""
from typing import Any, Dict, Optional
import orjson

from . import keys
from .redis_client import get_sync_client


class RedisSnapshotReader:
    """
    Convenience wrapper for retrieving live snapshots by key.
    """

    def __init__(self, *, redis_client=None, redis_url: Optional[str] = None) -> None:
        self.redis = redis_client or get_sync_client(redis_url)

    def get_block(self, block_number: int) -> Optional[Dict[str, Any]]:
        header = self.fetch_block_header(block_number)
        txs = self.fetch_processed_block(block_number)
        if header is None or txs is None:
            return None
        return {
            "header": header,
            "transactions": txs,
        }

    def get_block_snapshot(self, block_number: int) -> Optional[Dict[str, Any]]:
        return self.get_block(block_number)

    def fetch_block_header(self, block_number: int) -> Optional[Dict[str, Any]]:
        """Return the cached header for a specific block if available."""
        raw = self.redis.get(keys.block_header_key(block_number))
        return _decode(raw)

    def get_latest_block_number(self) -> Optional[int]:
        value = self.redis.get(keys.latest_block_number_key())
        if value is None:
            return None
        try:
            return int(value)
        except ValueError:
            return None

    def get_latest_block(self) -> Optional[Dict[str, Any]]:
        number = self.get_latest_block_number()
        if number is None:
            return None
        return self.get_block(number)

    def get_token(self, token_address: str) -> Optional[Dict[str, Any]]:
        raw = self.redis.get(keys.token_key(token_address))
        return _decode(raw)

    def get_token_snapshot(self, token_address: str) -> Optional[Dict[str, Any]]:
        """Return the stored token snapshot for address if present."""
        return self.get_token(token_address)

    def get_position(self, portfolio_id: str, token_address: str) -> Optional[Dict[str, Any]]:
        raw = self.redis.get(keys.position_key(portfolio_id, token_address))
        return _decode(raw)

    def fetch_processed_block(self, block_number: int) -> Optional[list]:
        """Return the ordered processed transactions for a block if cached."""
        hash_key = keys.processed_tx_map_key(block_number)
        raw = self.redis.hgetall(hash_key)
        if not raw:
            return None
        txs = [_decode(value) for value in raw.values() if value]
        txs.sort(key=_tx_sort_key)
        return txs

    def get_processed_tx(self, block_number: int, tx_hash: str) -> Optional[Dict[str, Any]]:
        """Return a single processed transaction by block/tx hash."""
        key = keys.processed_tx_map_key(block_number)
        raw = self.redis.hget(key, tx_hash)
        if raw is None and tx_hash:
            lowered = tx_hash.lower()
            if lowered != tx_hash:
                raw = self.redis.hget(key, lowered)
        return _decode(raw)


def _decode(payload: Optional[str]) -> Optional[Dict[str, Any]]:
    if payload is None:
        return None
    return orjson.loads(payload)


def _tx_sort_key(value: Optional[Dict[str, Any]]) -> int:
    if not isinstance(value, dict):
        return 0
    tx_data = value.get("transaction") or {}
    idx = tx_data.get("tx_index")
    if idx is None:
        idx = value.get("tx_index")
    if isinstance(idx, int):
        return idx
    try:
        return int(idx)
    except (TypeError, ValueError):
        return 0


__all__ = ["RedisSnapshotReader"]
