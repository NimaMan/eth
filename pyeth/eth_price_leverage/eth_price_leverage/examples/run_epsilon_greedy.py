#!/usr/bin/env python3
"""
Minimal epsilon-greedy trainer over the Rust env via pyreth.

- Uses DEFAULT_RETH_DATA_DIR by default (override with RETH_DATADIR env).
- Alternates buys/sells to avoid stalling on low ETH.
- Prints a compact CSV-like log to stdout.
"""
import os
import random
from typing import List, Tuple

import pyreth as pr

DEFAULT_RETH_DATA_DIR = os.environ.get("RETH_DATADIR", "/home/nima/.local/share/reth/mainnet")
AGENT = os.environ.get("BAYGUS_TEST_EOA", "0x0C96c602b1b332B8AB2093E5d72D804a24bd5689")
START_BLOCK = int(os.environ.get("START_BLOCK", "23311982"))
STEPS = int(os.environ.get("STEPS", "20"))
TIP_GWEI = int(os.environ.get("TIP_GWEI", "1"))
SLIPPAGE_BPS = int(os.environ.get("SLIPPAGE_BPS", "50"))
BLOCK_STEP = int(os.environ.get("BLOCK_STEP", "1"))
EPSILON = float(os.environ.get("EPSILON", "0.15"))

# Known pools
UNIV2_WETH_USDC = "0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc"
SUSHI_WETH_USDT = "0x06da0fd433C1A5d7a4faa01111c044910A184553"
UNIV3_WETH_USDC_500 = "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640"
UNIV3_WETH_USDC_3000 = "0x8ad599c3A0ff1De082011EFDDc58f1908eb6e6D8"
UNIV2_WETH_DAI = "0xA478c2975Ab1Ea89e8196811F51A7B7Ade33eB11"

# Action space: (route_kind, pool, token, fee_if_v3, decimals)
BUY_ROUTES: List[Tuple[str, str, str, int, int]] = [
    ("univ2", UNIV2_WETH_USDC, "USDC", 0, 6),
    ("sushiv2", SUSHI_WETH_USDT, "USDT", 0, 6),
    ("univ3", UNIV3_WETH_USDC_500, "USDC", 500, 6),
    ("univ3", UNIV3_WETH_USDC_3000, "USDC", 3000, 6),
    ("univ2", UNIV2_WETH_DAI, "DAI", 0, 18),
]

def main() -> None:
    env = pr.PyStablecoinEnv(AGENT, START_BLOCK, DEFAULT_RETH_DATA_DIR, TIP_GWEI, SLIPPAGE_BPS, BLOCK_STEP)
    # Enable per-block cache with default routes; prefetch the working range
    env.enable_cache_default_routes(256)
    env.prefetch_blocks(START_BLOCK, START_BLOCK + STEPS + 1)

    # Q-values per action index (toy bandit on buys only)
    q = [0.0 for _ in BUY_ROUTES]
    n = [0 for _ in BUY_ROUTES]

    print("step,block,action,side,token,eth_in,usd_total,reward")
    side = "Buy"
    for t in range(STEPS):
        chain, pf = env.state()
        block = chain.block

        # Epsilon-greedy selection among BUY routes
        if random.random() < EPSILON:
            idx = random.randrange(len(BUY_ROUTES))
        else:
            idx = max(range(len(q)), key=lambda i: q[i])
        route_kind, pool, token, fee, _qd = BUY_ROUTES[idx]

        # Fetch cached prices for features/logging
        mean_all, median_all, cl = env.get_cached_mean_median(block)
        eth_usd = median_all or mean_all or cl or 0.0

        # Alternate side when ETH is low or when random flip triggers
        eth_wei = int(pf.eth_wei)
        if side == "Buy" and eth_wei < 200_000_000_000_000:  # <0.0002 ETH
            side = "Sell"
        if side == "Sell" and int(pf.usdc_raw) < 10 * 10**6:  # <10 USDC
            side = "Buy"

        if side == "Buy":
            action = pr.PyStablecoinAction(route_kind, pool, "Buy", token, 50_000_000_000_000_000, fee=(fee if route_kind=="univ3" else None))
        else:
            # sell 50 USDC if available; otherwise fall back to no-op via min amount
            if token != "USDC":
                token = "USDC"
                route_kind, pool, fee = "univ3", UNIV3_WETH_USDC_500, 500
            sell_amt = max(50 * 10**6, int(pf.usdc_raw) // 2)
            action = pr.PyStablecoinAction(route_kind, pool, "Sell", token, sell_amt, fee=(fee if route_kind=="univ3" else None))

        out = env.step(action)
        reward = out.reward
        # Update Q
        n[idx] += 1
        alpha = 1.0 / max(1, n[idx])
        q[idx] += alpha * (reward - q[idx])

        usd_total = (int(out.portfolio.eth_wei) / 1e18) * eth_usd + (int(out.portfolio.usdc_raw)/1e6) + (int(out.portfolio.usdt_raw)/1e6) + (int(out.portfolio.dai_raw)/1e18)
        print(f"{t},{block},{route_kind}:{pool[:8]}..,{side},{token},0.05,{usd_total:.2f},{reward:.6f}")

        # Toggle side occasionally
        if random.random() < 0.1:
            side = "Sell" if side == "Buy" else "Buy"

if __name__ == "__main__":
    main()

