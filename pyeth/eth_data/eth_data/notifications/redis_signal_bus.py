"""Redis-based signal publishing and subscription helpers."""

import asyncio
import contextlib
import json
from typing import Any, Awaitable, Callable, Dict, Optional

import redis.asyncio as aioredis


class RedisSignalPublisher:
    def __init__(self, redis_url: str = "redis://localhost:6379/0") -> None:
        self._redis_url = redis_url
        self._redis: Optional[aioredis.Redis] = None

    async def _ensure_client(self) -> aioredis.Redis:
        if self._redis is None:
            self._redis = aioredis.from_url(self._redis_url, decode_responses=True)
        return self._redis

    async def publish(self, channel: str, message: Dict[str, Any]) -> None:
        client = await self._ensure_client()
        payload = json.dumps(message)
        await client.publish(channel, payload)


class RedisSignalSubscriber:
    def __init__(
        self,
        redis_url: str = "redis://localhost:6379/0",
        logger: Optional[Any] = None,
    ) -> None:
        self._redis_url = redis_url
        self._redis: Optional[aioredis.Redis] = None
        self._pubsub: Optional[aioredis.client.PubSub] = None
        self._listener_task: Optional[asyncio.Task] = None
        self._logger = logger

    async def _ensure_pubsub(self) -> aioredis.client.PubSub:
        if self._redis is None:
            self._redis = aioredis.from_url(self._redis_url, decode_responses=True)
        if self._pubsub is None:
            self._pubsub = self._redis.pubsub()
        return self._pubsub

    async def subscribe(
        self,
        channels: list[str],
        handler: Callable[[str, Dict[str, Any]], Awaitable[None]],
    ) -> None:
        pubsub = await self._ensure_pubsub()
        await pubsub.subscribe(*channels)

        async def _listener() -> None:
            while True:
                try:
                    message = await pubsub.get_message(ignore_subscribe_messages=True, timeout=1.0)
                    if message is None:
                        await asyncio.sleep(0)
                        continue
                    try:
                        data = json.loads(message["data"])
                    except json.JSONDecodeError:
                        data = {"raw": message["data"]}
                    await handler(message["channel"], data)
                except asyncio.CancelledError:
                    # Normal shutdown path
                    raise
                except Exception as exc:
                    if self._logger:
                        self._logger.error(
                            "RedisSignalSubscriber listener failed: %s", exc, exc_info=True
                        )
                    # Break so the caller can decide whether to restart/exit
                    break

        self._listener_task = asyncio.create_task(_listener())

    async def unsubscribe(self, channels: Optional[list[str]] = None) -> None:
        if self._pubsub:
            if channels:
                await self._pubsub.unsubscribe(*channels)
            else:
                await self._pubsub.unsubscribe()

    async def close(self) -> None:
        if self._listener_task:
            self._listener_task.cancel()
            try:
                await self._listener_task
            except asyncio.CancelledError:
                pass
            except Exception as exc:
                if self._logger:
                    self._logger.error(
                        "RedisSignalSubscriber listener exited with error during close: %s",
                        exc,
                        exc_info=True,
                    )
        if self._pubsub:
            await self._pubsub.close()
        if self._redis:
            await self._redis.close()
