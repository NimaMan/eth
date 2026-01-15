"""
Compatibility shim for legacy imports.

The canonical implementation now lives in
``eth_token.erc20_token.network.token_network_builder``.  This module
simply re-exports ``LiveTokenNetworkBuilder`` so older callers can keep
their existing import paths until they migrate.
"""

from eth_token.erc20_token.network.token_network_builder import LiveTokenNetworkBuilder

__all__ = ["LiveTokenNetworkBuilder"]
