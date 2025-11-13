import uuid

import pytest

from eth_data.live_data_registry.redis_client import (
    DEFAULT_REDIS_URL,
    get_sync_client,
)


@pytest.mark.integration
def test_live_data_registry_redis_available():
    """
    Ensure the Redis instance backing the live data registry is reachable and writable.
    """
    client = get_sysnc_client()
    assert client.ping(), f"Redis at {DEFAULT_REDIS_URL} did not respond to PING"

    key = f"test:live_data_registry:{uuid.uuid4().hex}"
    value = "redis-ok"
    client.set(key, value, ex=5)
    try:
        assert client.get(key) == value
    finally:
        client.delete(key)
