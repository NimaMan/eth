"""
Async publisher for live snapshots.
"""

import time
from typing import Any, Dict, Iterable, Optional

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
        max_blocks: Optional[int] = 1000,
    ) -> None:
        block_hash = _block_hash(snapshot)
        parent_hash = _parent_hash(snapshot)
        timestamp = _block_timestamp(snapshot)
        processed_at_ms = int(time.time() * 1000)
        tx_entries = snapshot.get("transactions") or []
        meta_payload = _dumps_json(
            {
                "schema_version": 1,
                "chain": "eth",
                "chain_id": 1,
                "block_number": int(block_number),
                "block_hash": block_hash,
                "parent_hash": parent_hash,
                "timestamp": timestamp,
                "tx_count": len(tx_entries),
                "processed_at_ms": processed_at_ms,
            }
        )
        meta_k = keys.block_meta_key(block_number)
        header_k = keys.block_header_key(block_number)
        latest_k = keys.latest_block_number_key()
        latest_hash_k = keys.latest_block_hash_key()
        tx_map_k = keys.processed_tx_map_key(block_number)
        tx_index_k = keys.tx_index_key(block_number)
        addresses_k = keys.block_addresses_key(block_number)
        recent_k = keys.recent_blocks_key()

        header_payload = snapshot.get("header")
        tx_map: Dict[str, str] = {}
        tx_index: Dict[str, int] = {}
        addresses = set()
        for tx in tx_entries:
            tx_hash = tx.get("hash")
            if not tx_hash:
                continue
            tx_map[tx_hash] = _dumps_json(tx)
            tx_index[tx_hash] = _tx_index(tx)
            addresses.update(_string_values(tx.get("unique_addresses") or []))

        async with self.redis.pipeline(transaction=False) as pipe:
            pipe.set(meta_k, meta_payload, ex=ttl_seconds)
            pipe.set(latest_k, block_number)
            if block_hash:
                pipe.set(latest_hash_k, block_hash)

            if header_payload:
                pipe.set(header_k, header_payload, ex=ttl_seconds)
            else:
                pipe.delete(header_k)

            pipe.delete(tx_map_k)
            pipe.delete(tx_index_k)
            pipe.delete(addresses_k)
            if tx_map:
                pipe.hset(tx_map_k, mapping=tx_map)
                if ttl_seconds is not None:
                    pipe.expire(tx_map_k, ttl_seconds)
            if tx_index:
                pipe.zadd(tx_index_k, tx_index)
                if ttl_seconds is not None:
                    pipe.expire(tx_index_k, ttl_seconds)
            if addresses:
                pipe.sadd(addresses_k, *sorted(addresses))
                if ttl_seconds is not None:
                    pipe.expire(addresses_k, ttl_seconds)

            pipe.zadd(recent_k, {str(int(block_number)): int(block_number)})
            if max_blocks is not None and max_blocks > 0:
                pipe.zremrangebyrank(recent_k, 0, -(max_blocks + 1))
            stream_fields = {
                "schema_version": 1,
                "chain": "eth",
                "chain_id": 1,
                "block_number": int(block_number),
                "block_hash": block_hash or "",
                "tx_count": len(tx_entries),
                "processed_at_ms": processed_at_ms,
            }
            if parent_hash:
                stream_fields["parent_hash"] = parent_hash
            if max_blocks is not None and max_blocks > 0:
                pipe.xadd(
                    keys.processed_block_stream_key(),
                    stream_fields,
                    maxlen=max_blocks,
                    approximate=False,
                )
            else:
                pipe.xadd(keys.processed_block_stream_key(), stream_fields)

            await pipe.execute()

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

    async def delete_block(self, block_number: int) -> None:
        """
        Remove a previously published block snapshot.
        """
        await self.redis.delete(
            keys.block_meta_key(block_number),
            keys.block_header_key(block_number),
            keys.processed_tx_map_key(block_number),
            keys.tx_index_key(block_number),
            keys.block_addresses_key(block_number),
            keys.chain_state_snapshot_key(block_number),
        )
        await self.redis.zrem(keys.recent_blocks_key(), int(block_number))


def _prepare_payload(snapshot: Dict[str, Any]) -> str:
    if "updated_at" not in snapshot:
        snapshot = dict(snapshot)
        snapshot["updated_at"] = time.time()
    return orjson.dumps(snapshot).decode()


def _dumps_json(value: Dict[str, Any]) -> str:
    return orjson.dumps(value).decode()


def _header_dict(snapshot: Dict[str, Any]) -> Dict[str, Any]:
    header = snapshot.get("header") or {}
    if isinstance(header, str):
        try:
            return orjson.loads(header)
        except orjson.JSONDecodeError:
            return {}
    if isinstance(header, dict):
        return header
    return {}


def _block_hash(snapshot: Dict[str, Any]) -> Optional[str]:
    return snapshot.get("block_hash") or _header_dict(snapshot).get("hash")


def _parent_hash(snapshot: Dict[str, Any]) -> Optional[str]:
    return snapshot.get("parent_hash") or _header_dict(snapshot).get("parentHash")


def _block_timestamp(snapshot: Dict[str, Any]) -> Optional[int]:
    timestamp = snapshot.get("timestamp") or _header_dict(snapshot).get("timestamp")
    return _parse_int(timestamp)


def _tx_index(tx: Dict[str, Any]) -> int:
    idx = tx.get("tx_index")
    if idx is None and isinstance(tx.get("transaction"), dict):
        idx = tx["transaction"].get("tx_index")
    parsed = _parse_int(idx)
    return parsed if parsed is not None else 0


def _parse_int(value: Any) -> Optional[int]:
    if value is None:
        return None
    if isinstance(value, int):
        return value
    if isinstance(value, str):
        try:
            return int(value, 16) if value.startswith("0x") else int(value)
        except ValueError:
            return None
    try:
        return int(value)
    except (TypeError, ValueError):
        return None


def _string_values(values: Iterable[Any]) -> list[str]:
    return [str(value) for value in values if value]


__all__ = ["LiveDataPublisher"]
