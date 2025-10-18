"""Convenience exports for the `eth_data` package."""

from pathlib import Path

_PACKAGE_ROOT = Path(__file__).resolve().parent
_NESTED_ROOT = _PACKAGE_ROOT / "eth_data"
if _NESTED_ROOT.exists():
    __path__.append(str(_NESTED_ROOT))

from .eth_data import reth_chain_query

__all__ = ["reth_chain_query"]
