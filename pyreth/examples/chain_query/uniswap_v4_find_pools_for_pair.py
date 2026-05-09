#!/usr/bin/env python3
"""
Find Uniswap v4 PoolIds for a token pair using PyReth (no RPC).

Looks up Initialize events on the singleton PoolManager and returns
the most recent PoolIds per pair with basic config (fee, tickSpacing, hooks).

Usage: python uniswap_v4_find_pools_for_pair.py [TOKEN_A] [TOKEN_B] [BLOCKS_BACK] [MAX_RESULTS]
Defaults to USDC/WETH, 20000 blocks, 5 results.
"""

import sys
from pyreth import chain_query

PM = "0x000000000004444C5DC75cB358380d2E3de08a90"
USDC = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
USDT = "0xdAC17F958D2ee523a2206206994597C13D831ec7"
WETH = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"


def run(pair_a, pair_b, blocks_back=20000, max_results=5):
    q = chain_query()
    pools = q.find_uniswap_v4_pools_for_pair(PM, pair_a, pair_b, int(blocks_back), int(max_results))
    print(f"Pair: {pair_a} / {pair_b}")
    print(f"Found: {len(pools)}")
    for i, p in enumerate(pools, 1):
        print(f"[{i}] pool_id={p['pool_id']} fee={p['fee']} tickSpacing={p['tick_spacing']} hooks={p['hooks']} block={p['block_number']}")
    return pools


def main():
    if len(sys.argv) >= 3:
        token_a, token_b = sys.argv[1], sys.argv[2]
    else:
        token_a, token_b = USDC, WETH
    blocks_back = int(sys.argv[3]) if len(sys.argv) >= 4 else 20000
    max_results = int(sys.argv[4]) if len(sys.argv) >= 5 else 5

    print("== Uniswap V4 PoolId Finder ==")
    run(token_a, token_b, blocks_back, max_results)

    # Also showcase USDT/WETH if default
    if (token_a, token_b) == (USDC, WETH):
        print()
        run(USDT, WETH, blocks_back, max_results)


if __name__ == "__main__":
    main()
