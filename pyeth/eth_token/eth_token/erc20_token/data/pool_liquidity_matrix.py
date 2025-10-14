"""
Pool Liquidity Matrix
=====================

Centralises cross-pool liquidity and pricing analytics for a given token.

Responsibilities
----------------
- Snapshot the current state of every pool tracked by a :class:`PoolManager`.
- Provide convenience helpers for fetching the “best” executable price
  (buy or sell) across all pools.
- Expose aggregate liquidity metrics (per-denomination and totals) to
  upstream components (e.g. live trackers, signal detectors).
- Offer lightweight simulation of trade impact by applying the AMM’s
  constant-product maths where available.

The goal is to keep pool comparison logic in one place so higher level
features can remain focused on orchestration rather than bespoke maths.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Dict, Iterable, List, Optional, Tuple

from eth_token.erc20_token.pools.base_pool import BasePool
from eth_token.erc20_token.pools.uniswap_v2_pool import UniswapV2Pool
from eth_token.erc20_token.pools.pool_manager import PoolManager


@dataclass
class PoolSnapshot:
    """Immutable snapshot of a single pool’s liquidity characteristics."""

    pool_address: str
    denom_address: str
    protocol: str
    price: float
    denom_reserve: float
    token_reserve: float
    can_buy: bool
    can_sell: bool


class PoolLiquidityMatrix:
    """Aggregates and analyses liquidity across every pool for a token."""

    def __init__(self, pool_manager: PoolManager):
        self._pool_manager = pool_manager

    # ------------------------------------------------------------------
    # Snapshot helpers
    # ------------------------------------------------------------------
    def _build_snapshot(self, pool: BasePool) -> PoolSnapshot:
        price = pool.get_price()
        denom_reserve = pool.get_denom_reserve()
        token_reserve = pool.get_token_reserve()
        return PoolSnapshot(
            pool_address=getattr(pool, "display_address", pool.pool_address),
            denom_address=pool.denom_address,
            protocol=pool.get_protocol(),
            price=price,
            denom_reserve=denom_reserve,
            token_reserve=token_reserve,
            can_buy=pool.can_buy,
            can_sell=pool.can_sell,
        )

    def snapshots(self) -> List[PoolSnapshot]:
        """Return snapshots for all known pools (V2/V3/V4)."""
        pools: Iterable[BasePool] = self._pool_manager.get_all_pools()
        return [self._build_snapshot(pool) for pool in pools]

    # ------------------------------------------------------------------
    # Aggregations
    # ------------------------------------------------------------------
    def total_liquidity_by_denom(self) -> Dict[str, float]:
        """Aggregate denomination liquidity across all pools."""
        aggregates: Dict[str, float] = {}
        for pool in self._pool_manager.get_all_pools():
            aggregates.setdefault(pool.denom_address, 0.0)
            aggregates[pool.denom_address] += pool.get_denom_reserve()
        return aggregates

    def total_token_liquidity(self) -> float:
        """Sum our token reserve across all pools."""
        return sum(pool.get_token_reserve() for pool in self._pool_manager.get_all_pools())

    # ------------------------------------------------------------------
    # Pricing helpers
    # ------------------------------------------------------------------
    def get_best_price(self, *, for_buy: bool) -> Optional[Tuple[PoolSnapshot, float]]:
        """
        Return the pool with the best price for buying or selling our token.

        ``for_buy=True`` means we want the cheapest price denominated in the
        paired token (how much denom we pay per unit). ``for_buy=False`` means
        we want the highest price when selling our token.
        """
        candidates = []
        for pool in self._pool_manager.get_all_pools():
            price = pool.get_price()
            if price <= 0:
                continue
            if for_buy and not pool.can_buy:
                continue
            if not for_buy and not pool.can_sell:
                continue
            snapshot = self._build_snapshot(pool)
            candidates.append((snapshot, price))

        if not candidates:
            return None

        if for_buy:
            return min(candidates, key=lambda x: x[1])
        return max(candidates, key=lambda x: x[1])

    # ------------------------------------------------------------------
    # Trade simulation helpers
    # ------------------------------------------------------------------
    def estimate_swap_output(self, pool: BasePool, amount_in: float, *, buy_token: bool) -> Optional[float]:
        """
        Estimate execution outcome using a simplistic constant-product model.

        Args:
            pool: The pool to simulate against.
            amount_in: Amount of input asset.
            buy_token: True if we are buying the tracked token (input denom),
                False if selling the tracked token.
        """
        if amount_in <= 0:
            return 0.0

        if isinstance(pool, UniswapV2Pool):
            return self._simulate_uniswap_v2(pool, amount_in, buy_token=buy_token)

        # Placeholder for more advanced pool types (V3/V4).
        return None

    def _simulate_uniswap_v2(self, pool: UniswapV2Pool, amount_in: float, *, buy_token: bool) -> float:
        """AMM k=x*y swap simulation for Uniswap v2 style pools."""
        reserve_in = pool.get_denom_reserve() if buy_token else pool.get_token_reserve()
        reserve_out = pool.get_token_reserve() if buy_token else pool.get_denom_reserve()
        if reserve_in <= 0 or reserve_out <= 0:
            return 0.0

        amount_in_with_fee = amount_in * 0.997  # 0.3% fee
        new_reserve_in = reserve_in + amount_in_with_fee
        new_reserve_out = reserve_in * reserve_out / new_reserve_in
        amount_out = reserve_out - new_reserve_out
        return max(amount_out, 0.0)


__all__ = ["PoolLiquidityMatrix", "PoolSnapshot"]
