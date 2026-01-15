from __future__ import annotations

import json
import os
from dataclasses import dataclass
from typing import Optional, Tuple

import pyreth as pr


@dataclass
class ChainSnapshot:
    block: int
    base_fee_wei: int


@dataclass
class PortfolioState:
    eth_wei: int
    usdc_raw: int
    usdt_raw: int
    dai_raw: int


@dataclass
class StepOutput:
    chain: ChainSnapshot
    portfolio: PortfolioState
    reward: float
    info: dict


class StablecoinEnv:
    """
    Thin Pythonic wrapper around pyreth.PyStablecoinEnv.
    - Converts U256 string fields into Python ints.
    - Parses info JSON payload into dict.
    - Exposes convenience helpers for caching routes and prefetching.
    """

    def __init__(
        self,
        agent: str,
        start_block: int,
        reth_datadir: Optional[str] = None,
        tip_gwei: int = 1,
        slippage_bps: int = 50,
        block_step: int = 1,
    ) -> None:
        if reth_datadir is None:
            reth_datadir = os.environ.get("RETH_DATADIR", "/home/nima/.local/share/reth/mainnet")
        self._env = pr.PyStablecoinEnv(
            agent,
            start_block,
            reth_datadir=reth_datadir,
            tip_gwei=tip_gwei,
            slippage_bps=slippage_bps,
            block_step=block_step,
        )

    def state(self) -> Tuple[ChainSnapshot, PortfolioState]:
        c, p = self._env.state()
        return (
            ChainSnapshot(block=int(c.block), base_fee_wei=int(c.base_fee_wei)),
            PortfolioState(
                eth_wei=int(p.eth_wei),
                usdc_raw=int(p.usdc_raw),
                usdt_raw=int(p.usdt_raw),
                dai_raw=int(p.dai_raw),
            ),
        )

    def step(self, action: pr.PyStablecoinAction) -> StepOutput:
        out = self._env.step(action)
        info = {}
        if out.info:
            try:
                info = json.loads(out.info)
            except Exception:
                info = {"raw": out.info}
        return StepOutput(
            chain=ChainSnapshot(block=int(out.chain.block), base_fee_wei=int(out.chain.base_fee_wei)),
            portfolio=PortfolioState(
                eth_wei=int(out.portfolio.eth_wei),
                usdc_raw=int(out.portfolio.usdc_raw),
                usdt_raw=int(out.portfolio.usdt_raw),
                dai_raw=int(out.portfolio.dai_raw),
            ),
            reward=float(out.reward),
            info=info,
        )

    # Pass-through helpers to advanced features
    def enable_cache_default_routes(self, capacity: int = 256) -> None:
        self._env.enable_cache_default_routes(int(capacity))

    def prefetch_blocks(self, start_block: int, end_block: int) -> None:
        self._env.prefetch_blocks(int(start_block), int(end_block))

    def get_cached_mean_median(self, block: int) -> Tuple[Optional[float], Optional[float], Optional[float]]:
        return self._env.get_cached_mean_median(int(block))

