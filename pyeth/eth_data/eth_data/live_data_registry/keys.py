"""
Key helpers for the live data registry.
"""

LATEST_BLOCK_NUMBER_KEY = "eth/live/latest/block_number"
LATEST_BLOCK_HASH_KEY = "eth/live/latest/block_hash"
PROCESSED_BLOCK_STREAM_KEY = "eth/live/blocks"
RECENT_BLOCKS_KEY = "eth/live/recent_blocks"
LATEST_CHAIN_STATE_KEY = "eth/live/latest/chain_state_block_number"
BLOCK_PREFIX = "eth/live/block"
TOKEN_SNAPSHOT_PREFIX = "eth/live/token/snapshot"
TOKEN_INDEX_KEY = "eth/live/token/snapshot/index"
POSITION_PREFIX = "eth/live/position"


def block_meta_key(block_number: int) -> str:
    return f"{BLOCK_PREFIX}/{int(block_number)}/meta"


def block_header_key(block_number: int) -> str:
    return f"{BLOCK_PREFIX}/{int(block_number)}/header"


def latest_block_number_key() -> str:
    return LATEST_BLOCK_NUMBER_KEY


def latest_block_hash_key() -> str:
    return LATEST_BLOCK_HASH_KEY


def processed_block_stream_key() -> str:
    return PROCESSED_BLOCK_STREAM_KEY


def recent_blocks_key() -> str:
    return RECENT_BLOCKS_KEY


def chain_state_snapshot_key(block_number: int) -> str:
    return f"{BLOCK_PREFIX}/{int(block_number)}/chain_state_snapshot"


def latest_chain_state_block_number_key() -> str:
    return LATEST_CHAIN_STATE_KEY


def processed_tx_map_key(block_number: int) -> str:
    return f"{BLOCK_PREFIX}/{int(block_number)}/txs"


def tx_index_key(block_number: int) -> str:
    return f"{BLOCK_PREFIX}/{int(block_number)}/tx_index"


def block_addresses_key(block_number: int) -> str:
    return f"{BLOCK_PREFIX}/{int(block_number)}/addresses"


def token_key(token_address: str) -> str:
    return f"{TOKEN_SNAPSHOT_PREFIX}/{token_address}"


def token_index_key() -> str:
    return TOKEN_INDEX_KEY


def position_key(portfolio_id: str, token_address: str) -> str:
    """
    Key for position data (portfolio may represent a wallet, strategy, etc.).
    """
    return f"{POSITION_PREFIX}/{portfolio_id}/{token_address.lower()}"


__all__ = [
    "block_meta_key",
    "block_header_key",
    "latest_block_number_key",
    "latest_block_hash_key",
    "processed_block_stream_key",
    "recent_blocks_key",
    "chain_state_snapshot_key",
    "latest_chain_state_block_number_key",
    "processed_tx_map_key",
    "tx_index_key",
    "block_addresses_key",
    "token_key",
    "token_index_key",
    "position_key",
]
