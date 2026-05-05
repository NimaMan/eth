#!/usr/bin/env python3
"""
Example: Query AMM liquidity via pyreth ChainQuery (no RPC)

Demonstrates:
 - get_uniswap_v2_liquidity(pair_address)
 - get_uniswap_v3_liquidity(pool_address, fee_tier)
 - optionally get_uniswap_v4_liquidity(pool_manager, pool_id)

Requires the Reth DB to be available at the configured path used by PyReth.
"""

import os
import sys

# Prefer local build of pyreth if available (development mode)
_here = os.path.abspath(os.path.dirname(__file__))
_proj_root = os.path.abspath(os.path.join(_here, '..', '..'))
_cand = [
    os.path.join(_proj_root, 'target', 'debug'),
    os.path.join(_proj_root, 'target', 'release'),
]
for _p in _cand:
    if os.path.isdir(_p):
        sys.path.insert(0, _p)

from pyreth import chain_query as pyreth_chain_query


def fmt_addr(a: str) -> str:
    if not a:
        return "-"
    return a[:8] + "…" + a[-4:]


def main():
    q = pyreth_chain_query()

    # Defaults (Ethereum mainnet)
    # Uniswap V2 USDC/WETH pair
    v2_pair = "0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc"
    # Uniswap V3 USDC/WETH 0.05% pool
    v3_pool = "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640"
    v3_fee = 500

    print("== AMM Liquidity via PyReth ChainQuery ==")

    try:
        v2 = q.get_uniswap_v2_liquidity(v2_pair, None)
        print("\nUniswap V2:")
        print(f"  Pair:      {v2.pool}")
        print(f"  token0:    {fmt_addr(v2.token0)} ({v2.token0_symbol})")
        print(f"  token1:    {fmt_addr(v2.token1)} ({v2.token1_symbol})")
        print(f"  reserve0:  {v2.reserve0}")
        print(f"  reserve1:  {v2.reserve1}")
        print(f"  decimals:  token0={v2.token0_decimals} token1={v2.token1_decimals}")
        pf = int(v2.price_1e18) / 1e18 if v2.price_1e18 else None
        if pf is not None:
            print(f"  price:     token1 per 1 token0 = {pf:.8f}")
        print(f"  block:     {v2.block_number}")
    except Exception as e:
        print(f"V2 query failed: {e}")

    try:
        v3 = q.get_uniswap_v3_liquidity(v3_pool, v3_fee, None)
        print("\nUniswap V3:")
        print(f"  Pool:      {v3.pool}")
        print(f"  token0:    {fmt_addr(v3.token0)} ({v3.token0_symbol})")
        print(f"  token1:    {fmt_addr(v3.token1)} ({v3.token1_symbol})")
        print(f"  liquidity: {v3.v3_liquidity}")
        print(f"  tick:      {v3.tick}")
        print(f"  decimals:  token0={v3.token0_decimals} token1={v3.token1_decimals}")
        print(f"  sqrtP96:  {v3.sqrt_price_x96}")
        pf = int(v3.price_1e18) / 1e18 if v3.price_1e18 else None
        if pf is not None:
            print(f"  price:     token1 per 1 token0 = {pf:.8f}")
        print(f"  block:     {v3.block_number}")
    except Exception as e:
        print(f"V3 query failed: {e}")

    # Uniswap V4: use provided args/env or auto-discover a recent pool id from logs
    pool_manager = None
    pool_id = None
    if len(sys.argv) == 3:
        pool_manager = sys.argv[1]
        pool_id = sys.argv[2]
    else:
        pool_manager = os.getenv("V4_POOL_MANAGER")
        pool_id = os.getenv("V4_POOL_ID")

    # If nothing provided, try to auto-discover from recent blocks (fast scan)
    if not (pool_manager and pool_id):
        pool_manager = pool_manager or "0x000000000004444C5DC75cB358380d2E3de08a90"
        try:
            auto_pid = q.find_recent_uniswap_v4_pool_id(pool_manager, 2000)
            if auto_pid:
                pool_id = "0x" + auto_pid
        except Exception as e:
            # Fall through to tip if not found
            pass

    if pool_manager and pool_id:
        try:
            v4 = q.get_uniswap_v4_liquidity(pool_manager, pool_id, None)
            print("\nUniswap V4:")
            print(f"  PoolMgr:   {pool_manager}")
            print(f"  pool_id:   {pool_id}")
            print(f"  liquidity: {v4.v3_liquidity}")
            print(f"  tick:      {v4.tick}")
            print(f"  sqrtP96:  {v4.sqrt_price_x96}")
            print(f"  block:     {v4.block_number}")
        except Exception as e:
            print(f"V4 query failed: {e}")
    else:
        print("\nTip: To test V4, set env V4_POOL_MANAGER & V4_POOL_ID, run with arguments <POOL_MANAGER> <POOL_ID_HEX>, or ensure recent V4 Initialize events exist in your DB for auto-discovery.")


if __name__ == "__main__":
    main()
