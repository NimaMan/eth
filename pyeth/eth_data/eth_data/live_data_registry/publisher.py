"""Async publisher for Python-owned live snapshots."""

import time
from typing import Any, Dict, Optional

import orjson

from . import keys
from .redis_client import get_async_client
from .snapshot_serialization import json_safe


class LiveDataPublisher:
    """
    High-level helper for writing Python-owned live snapshots to Redis.

    Rust owns processed block publication.  Python uses this publisher for
    derived token and position state.
    """

    def __init__(self, *, redis_client=None, redis_url: Optional[str] = None) -> None:
        self.redis = redis_client or get_async_client(redis_url)

    async def publish_token(
        self,
        token_address: str,
        snapshot: Dict[str, Any],
        *,
        ttl_seconds: Optional[int] = None,
    ) -> None:
        payload = _prepare_payload(snapshot)
        async with self.redis.pipeline(transaction=False) as pipe:
            pipe.set(keys.token_key(token_address), payload, ex=ttl_seconds)
            pipe.sadd(keys.token_index_key(), token_address)
            await pipe.execute()
    
    async def delete_token(self, token_address: str) -> None:
        """
        Remove a previously published token snapshot.
        """
        async with self.redis.pipeline(transaction=False) as pipe:
            pipe.delete(keys.token_key(token_address))
            pipe.srem(keys.token_index_key(), token_address)
            await pipe.execute()

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


def _prepare_payload(snapshot: Dict[str, Any]) -> str:
    if "updated_at" not in snapshot:
        snapshot = dict(snapshot)
        snapshot["updated_at"] = time.time()
    return orjson.dumps(json_safe(snapshot)).decode()


__all__ = ["LiveDataPublisher"]
