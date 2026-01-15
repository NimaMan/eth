"""
Example entrypoint for selecting best venues per USD stable using PriceData
emitted by the Rust eth_prices readers (via a Python binding or JSON feed).

This script is binding-agnostic: it expects a list of dicts shaped like
eth_prices.core.types.PriceData when serialized to JSON.
"""

from typing import Any, Dict, Iterable, List
from .scorer import BestVenueScorer


def run_best_venue_over_batch(batch: Iterable[Dict[str, Any]]) -> List[str]:
    scorer = BestVenueScorer(base_symbol="ETH")
    results = scorer.best_per_token(batch)
    # Pretty lines
    lines: List[str] = []
    for r in sorted(results, key=lambda x: x.token_symbol):
        lines.append(
            f"{r.token_symbol:>5}: {r.tokens_out_per_base:,.2f} per ETH via {r.route_label} (block {r.block_number})"
        )
    return lines


def main() -> None:
    # Placeholder example for when a binding is wired.
    # For now, demonstrate shape expectations with fake items.
    fake_batch = [
        {
            "pair": "ETH/USDC_BUYSIM",
            "price": 4_330_000_000.0,  # raw USDC (6 decimals) per 1 ETH
            "decimals": 18,
            "block_number": 23320000,
            "timestamp": 0,
            "source": {
                "type": "BuySim",
                "route": "UniswapV3 { fee_tier: 500 }",
                "buyer": "0x...",
                "token_out": "0xA0b8...",
                "eth_in_wei": str(10**18),
                "tokens_out_raw": str(4_330_000_000),
                "token0_symbol": "USDC",
                "token1_symbol": "WETH",
                "liquidity_protocol": "UniswapV3",
                "liquidity_pool": "0x88e6...",
            },
        },
        {
            "pair": "ETH/USDT_BUYSIM",
            "price": 4_334_000_000.0,
            "decimals": 18,
            "block_number": 23320000,
            "timestamp": 0,
            "source": {
                "type": "BuySim",
                "route": "UniswapV3 { fee_tier: 3000 }",
                "buyer": "0x...",
                "token_out": "0xdAC1...",
                "eth_in_wei": str(10**18),
                "tokens_out_raw": str(4_334_000_000),
                "token0_symbol": "USDT",
                "token1_symbol": "WETH",
                "liquidity_protocol": "UniswapV3",
                "liquidity_pool": "0x4e68...",
            },
        },
        {
            "pair": "ETH/DAI_BUYSIM",
            "price": 4_326_000_000_000_000_000.0,  # raw DAI (18)
            "decimals": 18,
            "block_number": 23320000,
            "timestamp": 0,
            "source": {
                "type": "BuySim",
                "route": "UniswapV2",
                "buyer": "0x...",
                "token_out": "0x6B17...",
                "eth_in_wei": str(10**18),
                "tokens_out_raw": str(4_326_000_000_000_000_000),
                "token0_symbol": "DAI",
                "token1_symbol": "WETH",
                "liquidity_protocol": "UniswapV2",
                "liquidity_pool": "0xb4e1...",
            },
        },
    ]

    lines = run_best_venue_over_batch(fake_batch)
    for ln in lines:
        print(ln)


if __name__ == "__main__":
    main()

