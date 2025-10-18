import asyncio

import orjson

from eth_data.blockchain.block_data_models import BlockHeader, ProcessedBlockResult
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

    block = ProcessedBlockResult(
        transactions=[{"hash": "0xabc"}],
        block_header=BlockHeader(number="0x1A"),
    )

    success = asyncio.run(processor.publish_block(26, block))

    assert success is True
    assert len(fake_exchange.published) == 1
    message, routing_key = fake_exchange.published[0]
    assert routing_key == "processed_blocks"
    payload = orjson.loads(message.body)
    assert payload["block_number"] == 26
    assert payload["transactions"] == block.transactions
    header_payload = payload["block_header"]
    if isinstance(header_payload, str):
        header_payload = orjson.loads(header_payload)
    assert header_payload["number"].lower() == "0x1a"


def test_publish_block_serializes_large_integers():
    processor = LiveBlockProcessor(rabbitmq_url="amqp://guest:guest@localhost/")
    fake_exchange = _FakeExchange()
    processor.blocks_exchange = fake_exchange

    large_value = 2**80
    block = ProcessedBlockResult(
        transactions=[{"hash": "0xlarge", "value": large_value}],
        block_header=None,
    )

    success = asyncio.run(processor.publish_block(42, block))

    assert success is True
    assert len(fake_exchange.published) == 1
    message, _ = fake_exchange.published[0]
    payload = orjson.loads(message.body)
    assert payload["transactions"][0]["value"] == str(large_value)


def test_publish_last_100_blocks_without_error():
    processor = LiveBlockProcessor(rabbitmq_url="amqp://guest:guest@localhost/")
    fake_exchange = _FakeExchange()
    processor.blocks_exchange = fake_exchange

    for offset in range(100):
        block_number = 500 + offset
        value = (2**70) + offset
        block = ProcessedBlockResult(
            transactions=[{"hash": f"0x{block_number:x}", "value": value}],
            block_header=BlockHeader(number=hex(block_number)),
        )
        success = asyncio.run(processor.publish_block(block_number, block))
        assert success is True

    assert len(fake_exchange.published) == 100
    # Validate last payload still encodes large integers safely
    last_message, _ = fake_exchange.published[-1]
    last_payload = orjson.loads(last_message.body)
    assert last_payload["transactions"][0]["value"] == str((2**70) + 99)


def test_publish_block_initializes_exchange(monkeypatch):
    processor = LiveBlockProcessor(rabbitmq_url="amqp://guest:guest@localhost/")
    fake_exchange = _FakeExchange()

    async def fake_setup() -> bool:
        processor.blocks_exchange = fake_exchange
        return True

    monkeypatch.setattr(processor, "setup_rabbitmq", fake_setup)

    block = ProcessedBlockResult(transactions=[{"hash": "0xdef"}], block_header=None)
    success = asyncio.run(processor.publish_block(100, block))

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
