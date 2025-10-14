"""
Fetch pool basics and liquidity using PoolChainDataFetcher (PyReth-backed).

Requires: local Reth DB for PyReth, or will fall back to Web3 for pool discovery.
"""

from pprint import pprint
from eth_token.erc20_token.pools.pool_chain_data_fetcher import PoolChainDataFetcher


def main():
    fetcher = PoolChainDataFetcher()

    v2_pool = "0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc"
    v3_pool = "0x8ad599c3a0ff1de082011efddc58f1908eb6e6d8"

    print("\n=== Uniswap V2 (USDC/WETH) ===")
    info_v2 = fetcher.discover_v2_pool(v2_pool)
    print("[discover_v2_pool]")
    pprint(info_v2)
    print("[get_v2_liquidity]")
    liq_v2 = fetcher.get_v2_liquidity(v2_pool)
    pprint(liq_v2)

    print("\n=== Uniswap V3 (USDC/WETH 0.3%) ===")
    info_v3 = fetcher.discover_v3_pool(v3_pool)
    print("[discover_v3_pool]")
    pprint(info_v3)
    print("[get_v3_liquidity]")
    liq_v3 = fetcher.get_v3_liquidity(v3_pool, fee_tier=3000)
    pprint(liq_v3)


if __name__ == "__main__":
    main()

