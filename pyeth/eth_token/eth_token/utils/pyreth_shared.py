"""Backward-compatible wrapper around the centralized PyrethClient."""

from typing import Any, Tuple

from eth_data.utils import PyrethClient


def get_pyreth_chain_query() -> Tuple[Any, Any]:
    """Return the shared PyReth instance and its ChainQuery handle."""
    client = PyrethClient.instance()
    return client.pyreth, client.chain_query()
