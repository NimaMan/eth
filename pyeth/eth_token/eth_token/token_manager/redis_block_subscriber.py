import asyncio
import json
from typing import Optional, Callable

from eth_data.live_data_registry import RedisSnapshotReader
from eth_data.notifications import RedisSignalSubscriber


class RedisBlockSubscriber:
    """Subscribe to live block notifications via Redis Pub/Sub."""

    def __init__(
        self,
        redis_url: str = "redis://localhost:6379/0",
        channel: str = "live_blocks",
        callback: Optional[Callable] = None,
        logger=None,
        block_snapshot_reader: Optional[RedisSnapshotReader] = None,
    ) -> None:
        self.logger = logger
        self.redis_url = redis_url
        self.channel = channel
        self.callback = callback
        self.block_snapshot_reader = block_snapshot_reader or RedisSnapshotReader()
        self._subscriber = RedisSignalSubscriber(redis_url)
        self._task: Optional[asyncio.Task] = None

    async def start(self) -> None:
        async def _handler(channel: str, payload: dict) -> None:
            block_number = payload.get("block_number")
            if block_number is None:
                return
            processed = await self.block_snapshot_reader.fetch_processed_block(block_number)
            if processed and self.callback:
                await self.callback(processed)

        await self._subscriber.subscribe([self.channel], _handler)

    async def stop(self) -> None:
        await self._subscriber.close()
