import asyncio
import json
import logging
import os
import uuid

import redis.asyncio as aioredis

from eth_data.blockchain.live_block_processor import LiveBlockProcessor


def _build_test_logger():
    logger = logging.getLogger("test_live_block_processor_publish")
    if not logger.handlers:
        logger.addHandler(logging.NullHandler())
    logger.propagate = False
    return logger


TEST_LOGGER = _build_test_logger()
REDIS_URL = os.getenv("LIVE_BLOCKCHAIN_DATA_REDIS_URL", "redis://localhost:6379/0")


async def _next_pubsub_message(pubsub, timeout: float = 5.0):
    async def _wait_for_message():
        while True:
            message = await pubsub.get_message(ignore_subscribe_messages=True, timeout=0.5)
            if message is not None:
                return message
            await asyncio.sleep(0)

    return await asyncio.wait_for(_wait_for_message(), timeout=timeout)


def test_live_block_processor_publishes_blocks_via_redis():
    asyncio.run(_assert_block_notification())


async def _assert_block_notification():
    channel = f"test_eth_live_block_notifications_{uuid.uuid4().hex}"
    processor = LiveBlockProcessor(
        block_notification_channel=channel,
        redis_url=REDIS_URL,
        logger=TEST_LOGGER,
    )

    redis_client = aioredis.from_url(REDIS_URL, decode_responses=True)
    pubsub = redis_client.pubsub()
    await pubsub.subscribe(channel)

    try:
        success = await processor.publish_block_notification(123456)
        assert success is True

        message = await _next_pubsub_message(pubsub)
        assert message["channel"] == channel
        payload = json.loads(message["data"])
        assert payload["block_number"] == 123456
    finally:
        await pubsub.unsubscribe(channel)
        await pubsub.aclose()
        await redis_client.aclose()
        await processor.cleanup()
