"""
Pool Chain Data Fetcher

Thin wrappers around PyReth ChainQuery liquidity helpers.
"""

from typing import Optional

from eth_data.pyreth_client import PyrethClient


class PoolChainDataFetcher:
    """Fetch pool liquidity snapshots via PyReth ChainQuery."""

    def __init__(self) -> None:
        client = PyrethClient.instance()
        self._chain_query = client.chain_query()

    @property
    def chain_query(self):
        return self._chain_query

    def get_v2_liquidity(
        self,
        pool_address: str,
        block: Optional[int] = None,
    ):
        return self._chain_query.get_uniswap_v2_liquidity(
            pool_address,
            block=block,
        )

    def get_v3_liquidity(
        self,
        pool_address: str,
        fee_tier: int,
        block: Optional[int] = None,
    ):
        return self._chain_query.get_uniswap_v3_liquidity(
            pool_address,
            int(fee_tier),
            block=block,
        )

    def get_v4_liquidity(
        self,
        pool_manager: str,
        pool_id_hex: str,
        block: Optional[int] = None,
    ):
        return self._chain_query.get_uniswap_v4_liquidity(
            pool_manager,
            pool_id_hex,
            block=block,
        )
