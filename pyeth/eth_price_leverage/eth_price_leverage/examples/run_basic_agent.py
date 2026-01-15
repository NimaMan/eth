#!/usr/bin/env python3
"""
Run a minimal training loop using the RandomPolicy on the Rust-backed env.

Env vars:
  - RETH_DATADIR      (default: /home/nima/.local/share/reth/mainnet)
  - BAYGUS_TEST_EOA   (default: demo EOA)
  - START_BLOCK       (default: 23311982)
"""
from __future__ import annotations

import os

from eth_price_leverage.envs.stablecoin_env import StablecoinEnv
from eth_price_leverage.policies.random_policy import RandomPolicy, RouteChoice
from eth_price_leverage.agents.basic_agent import Agent
from eth_price_leverage.actions import UNIV3_WETH_USDC_500


def main() -> None:
    agent_addr = os.environ.get("BAYGUS_TEST_EOA", "0x0C96c602b1b332B8AB2093E5d72D804a24bd5689")
    start_block = int(os.environ.get("START_BLOCK", "23311982"))
    reth_datadir = os.environ.get("RETH_DATADIR", "/home/nima/.local/share/reth/mainnet")

    env = StablecoinEnv(agent=agent_addr, start_block=start_block, reth_datadir=reth_datadir, tip_gwei=1, slippage_bps=50, block_step=1)
    # Optional: enable per-block cache for features (quotes + Chainlink)
    # env.enable_cache_default_routes(256)

    policy = RandomPolicy(routes=[RouteChoice("univ3", UNIV3_WETH_USDC_500, 500)])
    agent = Agent(env=env, policy=policy)

    # Run a few steps
    for i in range(3):
        tr = agent.step()
        print(f"step={i} reward={tr.reward:.6f} info={tr.info}")


if __name__ == "__main__":
    main()
