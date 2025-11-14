"""
PortfolioStateStore: Redis-backed snapshot of live positions.

The legacy `eth_portfolio_manager.live` package stored portfolio state in
Redis so dashboards and auxiliary services could read positions without
touching the trading loop. This module provides the same functionality in
the new live_trading architecture: it persists lightweight position
snapshots keyed by token address and optionally keeps a bounded history.
"""

from __future__ import annotations

import orjson
from datetime import datetime
from typing import Dict, Iterable, Optional

import redis.asyncio as aioredis

from eth_portfolio_manager.utils.logger import get_logger


class PortfolioStateStore:
    """Persist live position snapshots to Redis."""

    def __init__(
        self,
        redis_url: str = "redis://localhost:6379/0",
        namespace: str = "live_portfolio",
        logger=None,
    ) -> None:
        self.redis = aioredis.from_url(redis_url, decode_responses=True)
        self.namespace = namespace.rstrip(":")
        self.logger = logger or get_logger("portfolio_state_store")

    def _position_key(self, token_address: str) -> str:
        return f"{self.namespace}:position:{token_address.lower()}"

    def _history_key(self, token_address: str) -> str:
        return f"{self.namespace}:history:{token_address.lower()}"

    async def load_positions(self) -> Dict[str, Dict]:
        """Load all tracked positions from Redis."""
        positions: Dict[str, Dict] = {}
        try:
            pattern = f"{self.namespace}:position:*"
            async for key in self.redis.scan_iter(match=pattern):
                payload = await self.redis.get(key)
                if not payload:
                    continue
                token_address = key.split(":")[-1]
                positions[token_address] = orjson.loads(payload)
        except Exception as exc:
            self.logger.error("Error loading portfolio state: %s", exc)
        return positions

    async def upsert_positions(self, positions: Dict[str, Dict]) -> None:
        """Insert or update one or more position snapshots."""
        if not positions:
            return
        timestamp = datetime.utcnow().isoformat()
        try:
            async with self.redis.pipeline(transaction=True) as pipe:
                for token_address, position in positions.items():
                    key = self._position_key(token_address)
                    pipe.set(key, orjson.dumps(position))

                    history_entry = {"timestamp": timestamp, **position}
                    pipe.rpush(self._history_key(token_address), orjson.dumps(history_entry))
                    # Keep a bounded history (latest 200 entries)
                    pipe.ltrim(self._history_key(token_address), -200, -1)
                await pipe.execute()
        except Exception as exc:
            self.logger.error("Error upserting portfolio state: %s", exc)

    async def remove_positions(self, token_addresses: Iterable[str]) -> None:
        """Delete positions for tokens that are no longer active."""
        addresses = list(token_addresses)
        if not addresses:
            return
        try:
            async with self.redis.pipeline(transaction=True) as pipe:
                for token_address in addresses:
                    pipe.delete(self._position_key(token_address))
                await pipe.execute()
        except Exception as exc:
            self.logger.error("Error removing portfolio positions: %s", exc)

    async def clear(self) -> None:
        """Remove all state within the namespace (used by tests)."""
        pattern = f"{self.namespace}:*"
        try:
            keys = []
            async for key in self.redis.scan_iter(match=pattern):
                keys.append(key)
            if keys:
                await self.redis.delete(*keys)
        except Exception as exc:
            self.logger.error("Error clearing portfolio state: %s", exc)
