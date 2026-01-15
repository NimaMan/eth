from dataclasses import dataclass
from typing import Any, Dict, Iterable, List, Optional, Tuple


@dataclass
class BestVenueResult:
    token_symbol: str
    route_label: str
    tokens_out_per_base: float
    block_number: int
    # Optional diagnostics
    protocol: Optional[str] = None
    pool_address: Optional[str] = None
    token0_symbol: Optional[str] = None
    token1_symbol: Optional[str] = None
    reserve0: Optional[str] = None
    reserve1: Optional[str] = None
    v3_liquidity: Optional[str] = None
    tick: Optional[int] = None


class BestVenueScorer:
    """
    Selects the best venue per token from a batch of PriceData-like dicts.

    Expectations for each item (aligns with eth_prices PriceData JSON):
      - item["pair"]: str (e.g., "ETH/0xa0b8..._BUYSIM")
      - item["price"]: float (quote smallest units per 1 base)
      - item["block_number"]: int
      - item["source"]: dict with at least {"route": str}
        If source["type"] == "BuySim", may include optional liquidity context
        fields added by our Rust integration.
    """

    def __init__(self, base_symbol: str = "ETH", gas_tip_gwei: float = 0.0) -> None:
        self.base_symbol = base_symbol
        self.gas_tip_gwei = gas_tip_gwei

    def _extract_token_symbol(self, item: Dict[str, Any]) -> str:
        # For BuySim we stash token_out only in source; examples also carry a readable symbol in route.
        src = item.get("source", {})
        # If we have token symbols from liquidity snapshot, prefer those
        t0s = src.get("token0_symbol")
        t1s = src.get("token1_symbol")
        # Common stablecoins are quote tokens; pick the known stable symbol if present
        for sym in (t0s, t1s):
            if sym in {"USDC", "USDT", "DAI", "FRAX", "LUSD", "TUSD", "sUSD", "BUSD", "PYUSD", "GUSD"}:
                return sym
        # Fallback: parse from route label when it contains something like UniswapV2/WETH->USDC
        route = str(src.get("route", ""))
        for sym in ("USDC", "USDT", "DAI", "FRAX", "LUSD", "TUSD", "sUSD", "BUSD", "PYUSD", "GUSD"):
            if sym in route:
                return sym
        # Last resort: use checksum token_out address suffix
        return src.get("token_out", "UNKNOWN")

    def _route_label(self, item: Dict[str, Any]) -> str:
        src = item.get("source", {})
        return str(src.get("route", "UNKNOWN_ROUTE"))

    def _score(self, item: Dict[str, Any]) -> float:
        # Baseline: tokens_out per 1 base (already scaled to smallest unit)
        return float(item.get("price", 0.0))

    def best_per_token(self, items: Iterable[Dict[str, Any]]) -> List[BestVenueResult]:
        best: Dict[str, Tuple[float, Dict[str, Any]]] = {}
        for it in items:
            token = self._extract_token_symbol(it)
            sc = self._score(it)
            if token not in best or sc > best[token][0]:
                best[token] = (sc, it)

        results: List[BestVenueResult] = []
        for token, (score, it) in best.items():
            src = it.get("source", {})
            results.append(
                BestVenueResult(
                    token_symbol=token,
                    route_label=self._route_label(it),
                    tokens_out_per_base=score,
                    block_number=int(it.get("block_number", 0)),
                    protocol=src.get("liquidity_protocol"),
                    pool_address=src.get("liquidity_pool"),
                    token0_symbol=src.get("token0_symbol"),
                    token1_symbol=src.get("token1_symbol"),
                    reserve0=src.get("reserve0"),
                    reserve1=src.get("reserve1"),
                    v3_liquidity=src.get("v3_liquidity"),
                    tick=src.get("tick"),
                )
            )
        return results

