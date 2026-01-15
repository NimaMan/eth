import pytest

from eth_data.live_data_registry import keys
from eth_data.live_data_registry.reader import RedisSnapshotReader


@pytest.mark.integration
def test_fetch_live_token_snapshot():
    """Ensure we can read an existing token snapshot from the live Redis cache."""
    reader = RedisSnapshotReader()

    # Discover any existing token snapshot key
    token_key_iter = reader.redis.scan_iter(match=keys.token_key("*"), count=50)
    token_key = None
    for key in token_key_iter:
        token_key = key
        break

    if not token_key:
        pytest.skip("No token snapshots available in Redis to test against")

    token_address = token_key.split(":")[-1]
    snapshot = reader.get_token_snapshot(token_address)

    assert snapshot is not None, "Expected snapshot for live token address"
    assert snapshot.get("contract_address", "").lower() == token_address.lower()
