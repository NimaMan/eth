import asyncio

import orjson

from eth_data.blockchain.live_block_processor import LiveBlockProcessor


class _FakeExchange:
    def __init__(self):
        self.published = []

    async def publish(self, message, routing_key: str):
        self.published.append((message, routing_key))


def test_publish_block_uses_existing_exchange():
    processor = LiveBlockProcessor(rabbitmq_url="amqp://guest:guest@localhost/")
    fake_exchange = _FakeExchange()
    processor.blocks_exchange = fake_exchange

    success = asyncio.run(processor.publish_block(26))

    assert success is True
    assert len(fake_exchange.published) == 1
    message, routing_key = fake_exchange.published[0]
    assert routing_key == "processed_blocks"
    payload = orjson.loads(message.body)
    assert payload["block_number"] == 26


def test_publish_block_initializes_exchange(monkeypatch):
    processor = LiveBlockProcessor(rabbitmq_url="amqp://guest:guest@localhost/")
    fake_exchange = _FakeExchange()

    async def fake_setup() -> bool:
        processor.blocks_exchange = fake_exchange
        return True

    monkeypatch.setattr(processor, "setup_rabbitmq", fake_setup)

    success = asyncio.run(processor.publish_block(100))

    assert success is True
    assert len(fake_exchange.published) == 1


def test_publish_alert_uses_existing_exchange():
    processor = LiveBlockProcessor(rabbitmq_url="amqp://guest:guest@localhost/")
    fake_exchange = _FakeExchange()
    processor.alerts_exchange = fake_exchange

    alert_payload = {"alert": "example", "value": 1}

    success = asyncio.run(processor.publish_alert(alert_payload))

    assert success is True
    assert len(fake_exchange.published) == 1
    message, routing_key = fake_exchange.published[0]
    assert routing_key == "eth_txn_alerts"
    assert orjson.loads(message.body) == alert_payload


def test_publish_alert_initializes_exchange(monkeypatch):
    processor = LiveBlockProcessor(rabbitmq_url="amqp://guest:guest@localhost/")
    fake_exchange = _FakeExchange()

    async def fake_setup() -> bool:
        processor.alerts_exchange = fake_exchange
        return True

    monkeypatch.setattr(processor, "setup_rabbitmq", fake_setup)

    success = asyncio.run(processor.publish_alert({"alert": "init"}))

    assert success is True
    assert len(fake_exchange.published) == 1
