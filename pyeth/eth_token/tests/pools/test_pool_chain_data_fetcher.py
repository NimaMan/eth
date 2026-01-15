"""Smoke tests for `PoolChainDataFetcher` against canonical stablecoin pools."""

import pytest

from eth_token.erc20_token.pools.pool_chain_data_fetcher import PoolChainDataFetcher


USDC = "0xA0b86991c6218b36c1d19d4a2e9Eb0cE3606eB48"
WETH = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"
USDC_WETH_UNIV2_POOL = "0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc"


def test_get_v2_liquidity_for_usdc_weth_pool() -> None:
    fetcher = PoolChainDataFetcher()
    info = fetcher.get_v2_liquidity(USDC_WETH_UNIV2_POOL)

    if info is None:
        pytest.skip("Uniswap V2 liquidity unavailable via PyReth")

    tokens = {info.get("token0"), info.get("token1")}
    assert tokens == {USDC, WETH}
    assert info.get("protocol") in {"UniswapV2", "SushiswapV2"}

    reserve0 = info.get("reserve0_scaled") or info.get("reserve0")
    reserve1 = info.get("reserve1_scaled") or info.get("reserve1")
    assert reserve0 is not None
    assert reserve1 is not None
