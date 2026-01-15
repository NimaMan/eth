from __future__ import annotations

import random
from dataclasses import dataclass
from typing import List, Tuple

import pyreth as pr

from ..actions import make_action


@dataclass
class RouteChoice:
    route_kind: str
    pool: str
    fee: int | None = None


class RandomPolicy:
    """
    Naive random policy:
    - With p=0.5, Buy a random stable (USDC/USDT/DAI) with random ETH size.
    - Else, if balance exists, Sell a random stable (half balance or at least 10 units).
    """

    def __init__(self, routes: List[RouteChoice]) -> None:
        self.routes = routes

    def select_action(self, state) -> pr.PyStablecoinAction:
        route = random.choice(self.routes)
        side = random.choice(["Buy", "Sell"]) if (state.usdc_raw + state.usdt_raw + state.dai_raw) > 0 else "Buy"
        token = random.choice(["USDC", "USDT", "DAI"])  # only stables via router

        if side == "Buy":
            # 0.005 to 0.02 ETH in wei, clamped to available ETH
            eth_avail = int(state.eth_wei)
            if eth_avail <= 0:
                # No ETH — try to sell any stable with balance
                for tok, bal in ("USDC", state.usdc_raw), ("USDT", state.usdt_raw), ("DAI", state.dai_raw):
                    if bal > 0:
                        token_sel = tok
                        min_unit = 10 * (10**6 if tok in ("USDC", "USDT") else 10**18)
                        sell_amount = min(bal, max(bal // 2, min_unit))
                        return make_action(route.route_kind, route.pool, "Sell", token_sel, sell_amount, fee=route.fee)
                # Absolute fallback: no balances at all, try a dust buy if any wei exists
                return make_action(route.route_kind, route.pool, "Buy", token, 1, fee=route.fee)
            raw = int(random.uniform(0.005, 0.02) * 10**18)
            eth_amount_wei = min(raw, max(1, eth_avail // 2))
            return make_action(route.route_kind, route.pool, side, token, eth_amount_wei, fee=route.fee)
        else:
            # choose balance for token
            bal = {"USDC": state.usdc_raw, "USDT": state.usdt_raw, "DAI": state.dai_raw}[token]
            if bal <= 0:
                # fallback to buy if no balance
                eth_amount_wei = int(0.01 * 10**18)
                return make_action(route.route_kind, route.pool, "Buy", token, eth_amount_wei, fee=route.fee)
            min_unit = 10 * (10**6 if token in ("USDC", "USDT") else 10**18)
            sell_amount = max(bal // 2, min_unit)
            sell_amount = min(bal, sell_amount)  # never exceed balance
            if sell_amount <= 0:
                # safety fallback
                eth_amount_wei = int(0.01 * 10**18)
                return make_action(route.route_kind, route.pool, "Buy", token, eth_amount_wei, fee=route.fee)
            return make_action(route.route_kind, route.pool, side, token, sell_amount, fee=route.fee)
