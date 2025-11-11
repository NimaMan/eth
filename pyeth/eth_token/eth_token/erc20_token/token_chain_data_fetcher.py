"""TokenChainDataFetcher: lightweight PyReth-backed token metadata helper."""

from typing import Optional
from eth_data.pyreth_client import PyrethClient


class TokenChainDataFetcher:
    """Thin wrapper around PyReth chain-query methods for ERC20 metadata."""

    def __init__(self) -> None:
        client = PyrethClient.instance()
        self._chain_query = client.chain_query()

    @property
    def chain_query(self):
        return self._chain_query

    def get_token_decimals(
        self,
        token_address: str,
        block_number: Optional[int] = None,
        block_header: Optional[str] = None,
    ) -> int:
        decimals = self._chain_query.get_token_decimals(
            token_address,
            block_number,
            block_header,
        )
        if decimals is None:
            raise RuntimeError(f"Token {token_address} is missing decimals in PyReth")
        return int(decimals)

    def get_token_symbol(
        self,
        token_address: str,
        block_number: Optional[int] = None,
        block_header: Optional[str] = None,
    ) -> str:
        symbol = self._chain_query.get_token_symbol(
            token_address,
            block_number,
            block_header,
        )
        if symbol is None:
            raise RuntimeError(f"Token {token_address} is missing symbol in PyReth")
        return symbol

    def get_token_name(
        self,
        token_address: str,
        block_number: Optional[int] = None,
        block_header: Optional[str] = None,
    ) -> str:
        name = self._chain_query.get_token_name(
            token_address,
            block_number,
            block_header,
        )
        if name is None:
            raise RuntimeError(f"Token {token_address} is missing name in PyReth")
        return name

    def get_token_total_supply(
        self,
        token_address: str,
        block_number: Optional[int] = None,
        block_header: Optional[str] = None,
    ) -> int:
        total_supply = self._chain_query.get_token_total_supply(
            token_address,
            block_number,
            block_header,
        )
        if total_supply is None:
            raise RuntimeError(f"Token {token_address} is missing total supply in PyReth")
        return int(total_supply)

    def get_token_metadata(
        self,
        token_address: str,
        block_number: Optional[int] = None,
        block_header: Optional[str] = None,
    ):
        metadata = self._chain_query.get_token_metadata(
            token_address,
            block_number,
            block_header,
        )
        if metadata is None:
            raise RuntimeError(f"Token {token_address} is missing metadata in PyReth")
        return metadata
