"""Smoke tests for `PoolChainDataFetcher` against canonical stablecoin pools."""

import pytest

from eth_token.erc20_token.pools.numeric import parse_raw_float, parse_raw_int
from eth_token.erc20_token.pools.pool_chain_data_fetcher import PoolChainDataFetcher
from eth_data.chain_utils.common_addresses import (
    DENOM_NAMES_TO_ADDRESS,
    canonicalize_dex_pool_type,
)


USDC = DENOM_NAMES_TO_ADDRESS["USDC"]
WETH = DENOM_NAMES_TO_ADDRESS["WETH"]
USDC_WETH_UNIV2_POOL = "0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc"


def test_get_v2_liquidity_for_usdc_weth_pool() -> None:
    fetcher = PoolChainDataFetcher()
    info = fetcher.get_v2_liquidity(USDC_WETH_UNIV2_POOL)

    if info is None:
        pytest.skip("Uniswap V2 liquidity unavailable via PyReth")

    tokens = {info.get("token0"), info.get("token1")}
    assert tokens == {USDC, WETH}
    assert info.get("protocol") in {
        canonicalize_dex_pool_type("UniswapV2"),
        canonicalize_dex_pool_type("SushiswapV2"),
    }

    reserve0 = info.get("reserve0_scaled") or info.get("reserve0")
    reserve1 = info.get("reserve1_scaled") or info.get("reserve1")
    assert reserve0 is not None
    assert reserve1 is not None


def test_fetch_pool_metadata_for_token_orients_v2_pair() -> None:
    fetcher = PoolChainDataFetcher()
    info = fetcher.fetch_pool_metadata_for_token(
        pool_address=USDC_WETH_UNIV2_POOL,
        token_address=USDC,
        protocol_hint=canonicalize_dex_pool_type("UniswapV2"),
    )

    if info is None:
        pytest.skip("Uniswap V2 metadata unavailable via PyReth")

    assert info["denom_address"] == WETH
    assert info["token1_is_denom"] is True


def test_pool_numeric_helpers_accept_hex_strings() -> None:
    assert parse_raw_int("0xd99a8cec7e20000") == int("d99a8cec7e20000", 16)
    assert parse_raw_float("0x0") == 0.0
