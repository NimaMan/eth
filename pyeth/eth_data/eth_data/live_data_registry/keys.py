"""
Key helpers for the live data registry.
"""

from __future__ import annotations

def block_key(block_number: int) -> str:
    return f"live:block:{int(block_number)}"


def latest_block_key() -> str:
    return "live:block:latest"


def token_key(token_address: str) -> str:
    addr = token_address.lower()
    return f"live:token:{addr}"


def position_key(portfolio_id: str, token_address: str) -> str:
    """
    Key for position data (portfolio may represent a wallet, strategy, etc.).
    """
    return f"live:position:{portfolio_id}:{token_address.lower()}"


__all__ = ["block_key", "latest_block_key", "token_key", "position_key"]
