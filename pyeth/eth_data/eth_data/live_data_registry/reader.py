"""
Synchronous reader helpers for live data snapshots.
"""

from __future__ import annotations

from typing import Any, Dict, Optional

import orjson

from . import keys
from .redis_client import get_sync_client


class LiveDataReader:
    """
    Convenience wrapper for retrieving live snapshots by key.
    """

    def __init__(self, *, redis_client=None, redis_url: Optional[str] = None) -> None:
        self.redis = redis_client or get_sync_client(redis_url)

    def get_block(self, block_number: int) -> Optional[Dict[str, Any]]:
        raw = self.redis.get(keys.block_key(block_number))
        return _decode(raw)

    def get_latest_block_number(self) -> Optional[int]:
        value = self.redis.get(keys.latest_block_key())
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

    def get_position(self, portfolio_id: str, token_address: str) -> Optional[Dict[str, Any]]:
        raw = self.redis.get(keys.position_key(portfolio_id, token_address))
        return _decode(raw)


def _decode(payload: Optional[str]) -> Optional[Dict[str, Any]]:
    if payload is None:
        return None
    return orjson.loads(payload)


__all__ = ["LiveDataReader"]
