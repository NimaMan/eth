#!/usr/bin/env python3
from __future__ import annotations

import os
import pyreth

from eth_price_leverage.envs.stablecoin_env import StablecoinEnv


def main() -> None:
    # Resolve latest block using the shared instance, then clear to avoid DB conflicts
    reth = pyreth.PyReth(); q = reth.chain_query(); latest = q.get_latest_block(); del q, reth; pyreth.clear_singleton()
    start = latest - 10
    print(f"latest={latest} start={start}")

    agent_addr = os.environ.get("BAYGUS_TEST_EOA", "0x0C96c602b1b332B8AB2093E5d72D804a24bd5689")
    reth_datadir = os.environ.get("RETH_DATADIR", "/home/nima/.local/share/reth/mainnet")

    env = StablecoinEnv(agent=agent_addr, start_block=start, reth_datadir=reth_datadir, tip_gwei=1, slippage_bps=50, block_step=1)

    labels = env._env.default_discrete_labels(include_sells=True, include_noop=True)
    actions = env._env.build_default_discrete_actions(include_sells=True, include_noop=True, fixed_buy_wei=10**18)
    print(f"N actions = {len(actions)}\n  " + "\n  ".join(f"{i}: {lab}" for i, lab in enumerate(labels)))

    # Try one action per route sequentially
    for i, act in enumerate(actions):
        try:
            out = env.step(act)
            print(f"idx={i} reward={out.reward:.6f} info={out.info}")
        except Exception as e:
            print(f"idx={i} error: {e}")


if __name__ == "__main__":
    main()
