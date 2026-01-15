"""
Redis client helpers shared by publishers and readers.
"""

from __future__ import annotations

import os
from functools import lru_cache
from typing import Optional

import redis
import redis.asyncio as aioredis

DEFAULT_REDIS_URL = (
    os.getenv("LIVE_BLOCKCHAIN_DATA_REDIS_URL")
    or "redis://localhost:6379/0"
)


@lru_cache(maxsize=8)
def get_sync_client(redis_url: Optional[str] = None) -> redis.Redis:
    """
    Return a singleton synchronous Redis client for the provided URL.
    """
    url = redis_url or DEFAULT_REDIS_URL
    return redis.from_url(url, decode_responses=True)


@lru_cache(maxsize=8)
def get_async_client(redis_url: Optional[str] = None) -> aioredis.Redis:
    """
    Return a singleton asyncio Redis client for the provided URL.
    """
    url = redis_url or DEFAULT_REDIS_URL
    return aioredis.from_url(url, decode_responses=True)


__all__ = ["get_sync_client", "get_async_client", "DEFAULT_REDIS_URL"]
