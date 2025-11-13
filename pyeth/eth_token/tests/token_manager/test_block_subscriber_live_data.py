import pytest

from eth_token.token_manager.block_subscriber import BlockSubscriber


class DummyReader:
    def __init__(self, snapshot):
        self.snapshot = snapshot
        self.requested = []

    def get_block(self, number: int):
        self.requested.append(number)
        return self.snapshot


def test_resolve_payload_with_transactions_passthrough():
    payload = {
        "block_number": 10,
        "transactions": [{"hash": "0xabc"}],
        "block_header": {"number": "0xa"},
    }
    subscriber = BlockSubscriber()

    resolved = subscriber._resolve_block_payload(payload)

    assert resolved is payload


def test_resolve_payload_fetches_from_live_data_reader():
    snapshot = {
        "block_number": 20,
        "header": {"number": "0x14"},
        "transactions": [{"hash": "0xdef"}],
    }
    reader = DummyReader(snapshot)
    subscriber = BlockSubscriber(live_data_reader=reader)

    resolved = subscriber._resolve_block_payload({"block_number": 20})

    assert resolved["block_number"] == 20
    assert resolved["block_header"] == snapshot["header"]
    assert resolved["transactions"] == snapshot["transactions"]
    assert reader.requested == [20]


def test_resolve_payload_missing_snapshot_returns_none():
    reader = DummyReader(snapshot=None)
    subscriber = BlockSubscriber(live_data_reader=reader)

    resolved = subscriber._resolve_block_payload({"block_number": 30})

    assert resolved is None
