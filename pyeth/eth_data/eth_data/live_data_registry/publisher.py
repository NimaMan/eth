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
        block_k = keys.processed_block_snapshot_key(block_number)
        header_k = keys.block_header_key(block_number)
        latest_k = keys.latest_block_number_key()
        tx_map_k = keys.processed_tx_map_key(block_number)

        header_payload = snapshot.get("header")
        tx_entries = snapshot.get("transactions") or []
        tx_map: Dict[str, str] = {}
        for tx in tx_entries:
            tx_hash = tx.get("hash")
            if not tx_hash:
                continue
            tx_map[tx_hash] = _dumps_json(tx)

        async with self.redis.pipeline(transaction=False) as pipe:
            pipe.set(block_k, payload, ex=ttl_seconds)
            pipe.set(latest_k, block_number)

            if header_payload:
                pipe.set(header_k, header_payload, ex=ttl_seconds)
            else:
                pipe.delete(header_k)

            pipe.delete(tx_map_k)
            if tx_map:
                pipe.hset(tx_map_k, mapping=tx_map)
                if ttl_seconds is not None:
                    pipe.expire(tx_map_k, ttl_seconds)

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
    
    async def delete_token(self, token_address: str) -> None:
        """
        Remove a previously published token snapshot.
        """
        await self.redis.delete(keys.token_key(token_address))

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
        await self.redis.delete(
            keys.processed_block_snapshot_key(block_number),
            keys.block_header_key(block_number),
            keys.processed_tx_map_key(block_number),
        )


def _prepare_payload(snapshot: Dict[str, Any]) -> str:
    if "updated_at" not in snapshot:
        snapshot = dict(snapshot)
        snapshot["updated_at"] = time.time()
    return orjson.dumps(snapshot).decode()


def _dumps_json(value: Dict[str, Any]) -> str:
    return orjson.dumps(value).decode()


__all__ = ["LiveDataPublisher"]
