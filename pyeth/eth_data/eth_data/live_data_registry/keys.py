"""
Key helpers for the live data registry.
"""

def block_header_key(block_number: int) -> str:
    return f"block:block_header:{int(block_number)}"


def latest_block_number_key() -> str:
    return "block:latest_block_number"


def chain_state_snapshot_key(block_number: int) -> str:
    return f"block:chain_state_snapshot:{int(block_number)}"


def latest_chain_state_block_number_key() -> str:
    return "block:latest_chain_state_block_number"


def processed_tx_map_key(block_number: int) -> str:
    return f"block:processed_transactions:{int(block_number)}"


def token_key(token_address: str) -> str:
    return f"token:snapshot:{token_address}"


def position_key(portfolio_id: str, token_address: str) -> str:
    """
    Key for position data (portfolio may represent a wallet, strategy, etc.).
    """
    return f"position:{portfolio_id}:{token_address.lower()}"


__all__ = [
    "block_header_key",
    "latest_block_number_key",
    "chain_state_snapshot_key",
    "latest_chain_state_block_number_key",
    "processed_tx_map_key",
    "token_key",
    "position_key",
]
