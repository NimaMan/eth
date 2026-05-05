import asyncio
import contextlib
import json
from typing import Optional, Callable

import redis.asyncio as aioredis

from eth_data.live_data_registry import RedisSnapshotReader


class RedisBlockSubscriber:
    """Subscribe to live block notifications via Redis Pub/Sub."""

    def __init__(
        self,
        redis_url: str = "redis://localhost:6379/0",
        channel: str = "eth/live/block_notifications",
        callback: Optional[Callable] = None,
        logger=None,
        block_snapshot_reader: Optional[RedisSnapshotReader] = None,
    ) -> None:
        self.logger = logger
        self.redis_url = redis_url
        self.channel = channel
        self.callback = callback
        self.block_snapshot_reader = block_snapshot_reader or RedisSnapshotReader()
        self._redis: Optional[aioredis.Redis] = None
        self._pubsub: Optional[aioredis.client.PubSub] = None
        self._task: Optional[asyncio.Task] = None

    async def start(self) -> None:
        self._redis = aioredis.from_url(self.redis_url, decode_responses=True)
        self._pubsub = self._redis.pubsub()
        await self._pubsub.subscribe(self.channel)
        self._task = asyncio.create_task(self._listen())

    async def _listen(self) -> None:
        if self._pubsub is None:
            return

        while True:
            try:
                message = await self._pubsub.get_message(
                    ignore_subscribe_messages=True,
                    timeout=1.0,
                )
                if message is None:
                    await asyncio.sleep(0)
                    continue
                try:
                    payload = json.loads(message["data"])
                except json.JSONDecodeError:
                    payload = {"raw": message["data"]}
                await self._handle_payload(payload)
            except asyncio.CancelledError:
                raise
            except Exception as exc:
                if self.logger:
                    self.logger.error(
                        "Redis block subscriber failed: %s",
                        exc,
                        exc_info=True,
                    )
                return

    async def _handle_payload(self, payload: dict) -> None:
        block_number = payload.get("block_number")
        if block_number is None:
            return
        try:
            block_number = int(block_number)
        except (TypeError, ValueError):
            return
        try:
            processed = await asyncio.to_thread(
                self.block_snapshot_reader.fetch_processed_block,
                block_number,
            )
        except Exception as exc:
            if self.logger:
                self.logger.error(
                    "Failed to fetch processed block %s from Redis: %s",
                    block_number,
                    exc,
                    exc_info=True,
                )
            return

        if not processed or not self.callback:
            return

        try:
            await self.callback(
                {
                    "block_number": block_number,
                    "transactions": processed,
                }
            )
        except Exception as exc:
            if self.logger:
                self.logger.error(
                    "Error delivering block %s to callback: %s",
                    block_number,
                    exc,
                    exc_info=True,
                )

    async def stop(self) -> None:
        if self._task is not None:
            self._task.cancel()
            with contextlib.suppress(asyncio.CancelledError):
                await self._task
            self._task = None
        if self._pubsub is not None:
            await self._pubsub.unsubscribe(self.channel)
            await self._pubsub.close()
            self._pubsub = None
        if self._redis is not None:
            await self._redis.close()
            self._redis = None
