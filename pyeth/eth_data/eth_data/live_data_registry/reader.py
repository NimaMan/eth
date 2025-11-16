"""
Synchronous reader helpers for live data snapshots.
"""

from __future__ import annotations

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
        raw = self.redis.get(keys.processed_block_snapshot_key(block_number))
        return _decode(raw)

    def get_block_snapshot(self, block_number: int) -> Optional[Dict[str, Any]]:
        """Alias for get_block to emphasize block snapshot semantics."""
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
        snapshot = self.get_block_snapshot(block_number)
        if not snapshot:
            return None
        return snapshot.get("transactions")

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


__all__ = ["RedisSnapshotReader"]
