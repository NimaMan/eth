"""Thin wrappers around PyReth ChainQuery liquidity helpers."""

from typing import Any, Dict, Optional

from pyreth import chain_query

from eth_data.chain_utils.common_addresses import canonicalize_dex_pool_type
from eth_token.erc20_token.pools.addresses import checksum_address


class PoolChainDataFetcher:
    """Fetch pool liquidity snapshots via PyReth ChainQuery."""

    def __init__(self) -> None:
        self._chain_query = chain_query()

    @property
    def chain_query(self):
        return self._chain_query

    def get_v2_liquidity(
        self,
        pool_address: str,
        block: Optional[int] = None,
    ):
        return _pool_liquidity_to_dict(self._chain_query.get_uniswap_v2_liquidity(
            pool_address,
            block=block,
        ))

    def get_v3_liquidity(
        self,
        pool_address: str,
        fee_tier: int,
        block: Optional[int] = None,
    ):
        return _pool_liquidity_to_dict(self._chain_query.get_uniswap_v3_liquidity(
            pool_address,
            int(fee_tier),
            block=block,
        ))

    def get_v4_liquidity(
        self,
        pool_manager: str,
        pool_id_hex: str,
        block: Optional[int] = None,
    ):
        return _pool_liquidity_to_dict(self._chain_query.get_uniswap_v4_liquidity(
            pool_manager,
            pool_id_hex,
            block=block,
        ))

    def fetch_pool_metadata_for_token(
        self,
        *,
        pool_address: str,
        token_address: str,
        protocol_hint: str,
        block_number: Optional[int] = None,
        block_header: Optional[str] = None,
        fee_tier: Optional[int] = None,
    ) -> Optional[Dict[str, Any]]:
        """Return token orientation metadata for a pool touching ``token_address``."""
        _ = block_header  # PyReth ChainQuery uses block numbers for these view calls.
        protocol = (protocol_hint or "").lower()
        if "v3" in protocol:
            info = self.get_v3_liquidity(pool_address, fee_tier or 3000, block_number)
        else:
            info = self.get_v2_liquidity(pool_address, block_number)
        if not info:
            return None

        token0 = info.get("token0")
        token1 = info.get("token1")
        wanted = checksum_address(token_address)
        if not token0 or not token1 or not wanted:
            return None

        if token0 == wanted:
            return {
                "pool_address": pool_address,
                "protocol": info.get("protocol"),
                "token_address": wanted,
                "denom_address": token1,
                "token1_is_denom": True,
                "fee": fee_tier or 3000,
            }
        if token1 == wanted:
            return {
                "pool_address": pool_address,
                "protocol": info.get("protocol"),
                "token_address": wanted,
                "denom_address": token0,
                "token1_is_denom": False,
                "fee": fee_tier or 3000,
            }
        return None


_POOL_LIQUIDITY_FIELDS = (
    "protocol",
    "pool",
    "pool_id",
    "token0",
    "token1",
    "token0_symbol",
    "token1_symbol",
    "token0_decimals",
    "token1_decimals",
    "reserve0_raw",
    "reserve1_raw",
    "reserve0_scaled",
    "reserve1_scaled",
    "v3_liquidity",
    "tick",
    "sqrt_price_x96",
    "price_1e18",
    "block_number",
)


def _pool_liquidity_to_dict(info) -> Optional[Dict[str, Any]]:
    if info is None:
        return None
    if isinstance(info, dict):
        data = dict(info)
    else:
        data = {field: getattr(info, field, None) for field in _POOL_LIQUIDITY_FIELDS}
    data["protocol"] = canonicalize_dex_pool_type(data.get("protocol"))
    for key in ("pool", "token0", "token1"):
        data[key] = checksum_address(data.get(key))
    return data
