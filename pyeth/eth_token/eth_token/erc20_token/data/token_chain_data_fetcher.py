"""TokenChainDataFetcher: cached PyReth-backed token metadata helper."""

from typing import Optional
from eth_data.utils.pyreth_client import PyrethClient


class TokenChainDataFetcher:
    """Lightweight wrapper around PyReth's chain query for token metadata calls."""

    def __init__(self):
        client = PyrethClient.instance()
        self._chain_query = client.chain_query()

    @property
    def chain_query(self):
        return self._chain_query

    def get_token_decimals(self, token_address: str, block: Optional[int] = None) -> int:
        decimals = self._chain_query.get_token_decimals(token_address, block)
        if decimals is None:
            raise RuntimeError(f"Token {token_address} is missing decimals in PyReth")
        return int(decimals)

    def get_token_symbol(self, token_address: str, block: Optional[int] = None) -> str:
        symbol = self._chain_query.get_token_symbol(token_address, block)
        if symbol is None:
            raise RuntimeError(f"Token {token_address} is missing symbol in PyReth")
        return symbol

    def get_token_name(self, token_address: str, block: Optional[int] = None) -> str:
        name = self._chain_query.get_token_name(token_address, block)
        if name is None:
            raise RuntimeError(f"Token {token_address} is missing name in PyReth")
        return name

    def get_token_metadata(self, token_address: str, block: Optional[int] = None):
        metadata = self._chain_query.get_token_metadata(token_address, block)
        if metadata is None:
            raise RuntimeError(f"Token {token_address} is missing metadata in PyReth")
        return metadata
        
