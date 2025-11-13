"""
Async publisher for live snapshots.
"""

from __future__ import annotations

import time
from typing import Any, Dict, Optional

import orjson

from . import keys
from .redis_client import get_async_client


class LiveDataPublisher:
    """
    High-level helper for writing live data snapshots to Redis.
    """

    def __init__(self, *, redis_client=None, redis_url: Optional[str] = None) -> None:
        self.redis = redis_client or get_async_client(redis_url)

    async def publish_block(
        self,
        block_number: int,
        snapshot: Dict[str, Any],
        *,
        ttl_seconds: Optional[int] = None,
    ) -> None:
        payload = _prepare_payload(snapshot)
        block_k = keys.block_key(block_number)
        latest_k = keys.latest_block_key()
        async with self.redis.pipeline(transaction=False) as pipe:
            pipe.set(block_k, payload, ex=ttl_seconds)
            pipe.set(latest_k, block_number)
            await pipe.execute()

    async def publish_token(
        self,
        token_address: str,
        snapshot: Dict[str, Any],
        *,
        ttl_seconds: Optional[int] = None,
    ) -> None:
        payload = _prepare_payload(snapshot)
        await self.redis.set(keys.token_key(token_address), payload, ex=ttl_seconds)

    async def publish_position(
        self,
        portfolio_id: str,
        token_address: str,
        snapshot: Dict[str, Any],
        *,
        ttl_seconds: Optional[int] = None,
    ) -> None:
        payload = _prepare_payload(snapshot)
        key = keys.position_key(portfolio_id, token_address)
        await self.redis.set(key, payload, ex=ttl_seconds)

    async def delete_block(self, block_number: int) -> None:
        """
        Remove a previously published block snapshot.
        """
        await self.redis.delete(keys.block_key(block_number))


def _prepare_payload(snapshot: Dict[str, Any]) -> str:
    if "updated_at" not in snapshot:
        snapshot = dict(snapshot)
        snapshot["updated_at"] = time.time()
    return orjson.dumps(snapshot).decode()


__all__ = ["LiveDataPublisher"]
