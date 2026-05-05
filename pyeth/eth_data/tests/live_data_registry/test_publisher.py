import orjson

from eth_data.live_data_registry.publisher import _prepare_payload
from eth_data.live_data_registry.snapshot_serialization import (
    json_safe,
    normalize_block_header,
)


def test_prepare_payload_stringifies_uint256_sized_values():
    value = 2**128

    payload = _prepare_payload(
        {
            "metadata": {"total_supply": value},
            "pools": {"reserves": {"token": value}},
            "small_block": 25_028_491,
        }
    )

    decoded = orjson.loads(payload)
    assert decoded["metadata"]["total_supply"] == str(value)
    assert decoded["pools"]["reserves"]["token"] == str(value)
    assert decoded["small_block"] == 25_028_491
    assert "updated_at" in decoded


def test_json_safe_stringifies_nested_large_ints():
    value = 2**128

    decoded = json_safe({"values": [1, value]})

    assert decoded == {"values": [1, str(value)]}


def test_normalize_block_header_accepts_pyreth_processed_block_shape():
    class FakeBlock:
        number = 123
        hash = "0xblock"
        parent_hash = "0xparent"
        timestamp = 456
        gas_used = 789
        gas_limit = 1000
        base_fee_per_gas = "42"
        transactions = [{"hash": "0xtx", "tx_index": 0}]

    assert normalize_block_header(FakeBlock())["number"] == "0x7b"
