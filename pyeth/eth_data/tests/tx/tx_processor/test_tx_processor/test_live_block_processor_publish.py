import asyncio

from eth_data.blockchain.live_block_processor import LiveBlockProcessor


class _FakeSignalPublisher:
    def __init__(self):
        self.published = []

    async def publish(self, channel, payload):
        self.published.append((channel, payload))


def test_publish_block_notification_uses_existing_publisher():
    processor = LiveBlockProcessor()
    fake_publisher = _FakeSignalPublisher()
    processor._block_signal_publisher = fake_publisher

    success = asyncio.run(processor.publish_block_notification(26))

    assert success is True
    assert len(fake_publisher.published) == 1
    channel, payload = fake_publisher.published[0]
    assert channel == "live_blocks"
    assert payload["block_number"] == 26


def test_publish_block_notification_custom_channel():
    processor = LiveBlockProcessor(block_notification_channel="custom_blocks")
    fake_publisher = _FakeSignalPublisher()
    processor._block_signal_publisher = fake_publisher

    success = asyncio.run(processor.publish_block_notification(100))

    assert success is True
    assert fake_publisher.published[0][0] == "custom_blocks"


def test_publish_block_notification_without_publisher_returns_false():
    processor = LiveBlockProcessor()
    processor._block_signal_publisher = None

    success = asyncio.run(processor.publish_block_notification(1))

    assert success is False
