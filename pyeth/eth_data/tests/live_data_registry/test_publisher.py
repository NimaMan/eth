import orjson

from eth_data.live_data_registry.publisher import _prepare_payload
from eth_data.live_data_registry.snapshot_serialization import dumps_snapshot


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


def test_dumps_snapshot_stringifies_nested_large_ints():
    value = 2**128

    decoded = orjson.loads(dumps_snapshot({"values": [1, value]}))

    assert decoded == {"values": [1, str(value)]}
