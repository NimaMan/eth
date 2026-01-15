"""TokenChainDataFetcher: lightweight PyReth-backed token metadata helper."""

from typing import Optional, List, Any, Dict
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
    ) -> int:
        decimals = self._chain_query.get_token_decimals(token_address, block_number)
        if decimals is None:
            raise RuntimeError(f"Token {token_address} is missing decimals in PyReth")
        return int(decimals)

    def get_token_symbol(
        self,
        token_address: str,
        block_number: Optional[int] = None,
    ) -> str:
        symbol = self._chain_query.get_token_symbol(token_address, block_number)
        if symbol is None:
            raise RuntimeError(f"Token {token_address} is missing symbol in PyReth")
        return symbol

    def get_token_name(
        self,
        token_address: str,
        block_number: Optional[int] = None,
    ) -> str:
        name = self._chain_query.get_token_name(token_address, block_number)
        if name is None:
            raise RuntimeError(f"Token {token_address} is missing name in PyReth")
        return name

    def get_token_total_supply(
        self,
        token_address: str,
        block_number: Optional[int] = None,
    ) -> int:
        total_supply = self._chain_query.get_token_total_supply(token_address, block_number)
        if total_supply is None:
            raise RuntimeError(f"Token {token_address} is missing total supply in PyReth")
        return int(total_supply)

    def get_token_metadata(
        self,
        token_address: str,
        block_number: Optional[int] = None,
        pending_transactions: Optional[List[Dict[str, Any]]] = None,
        gas_block_number: Optional[int] = None,
    ):
        """
        Fetch token metadata (name, symbol, decimals, supply) from the blockchain.
        
        If pending_transactions are provided (list of dicts), they are converted to
        UnsignedTransactions and replayed on top of the block state before querying metadata.
        This is essential for fetching metadata of newly created tokens within the same block
        or for handling intra-block state dependencies (e.g. funding -> deployment).
        """
        return self._chain_query.get_token_metadata(
            token_address,
            block_number,
            pending_transactions or None,
            gas_block_number,
        )
