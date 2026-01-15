"""
Best-of venue example using the Rust binding via `pyreth`.

Steps:
- Create a shared PyReth instance (opens local Reth DB once)
- Get a price client (AggregatedPriceReader under the hood)
- Fetch prices for a generic pair (e.g., ETH/USD)
- Convert to dicts and feed into BestVenueScorer
"""

from typing import Any, Dict, List

try:
    import pyreth  # type: ignore
except Exception as e:  # pragma: no cover
    raise SystemExit("pyreth module is not available. Build/install the Rust bindings first.") from e

from eth_price_leverage.policies.best_venue import BestVenueScorer


def to_dicts(prices: Dict[str, Any]) -> List[Dict[str, Any]]:
    out: List[Dict[str, Any]] = []
    for src, obj in prices.items():
        # obj is a PyPriceData or a Python exception object on error
        d: Dict[str, Any]
        try:
            d = obj.to_dict()  # type: ignore[attr-defined]
        except Exception:
            # Skip errors
            continue
        # Normalize source_info if needed
        si = d.pop("source_info", {}) or {}
        # Upgrade into unified source dict
        # Note: Aggregated reader uses state-based readers, not BuySim; keys differ.
        d["source"] = {
            "route": f"{si.get('protocol','Unknown')}:{si.get('pool_address','')}",
            "token0_symbol": si.get("token0"),
            "token1_symbol": si.get("token1"),
            # Keep protocol/pool fields so scorer can report
            "liquidity_protocol": si.get("protocol"),
            "liquidity_pool": si.get("pool_address"),
        }
        out.append(d)
    return out


def main() -> None:
    reth = pyreth.PyReth()
    client = reth.price_client()

    # Initialize common AMMs and oracles
    client.with_uniswap_v2()
    client.with_uniswap_v3()
    client.with_sushiswap()
    client.with_curve()
    client.with_balancer()
    # client.with_chainlink()  # optional; async in binding is limited

    # Fetch prices for a generic pair; mapper translates per protocol
    all_prices = client.get_all_prices("ETH/USD")  # sync AMMs
    # For protocols needing async (Curve/Balancer), prefer get_all_prices via binding (already wrapped)

    batch = to_dicts(all_prices)
    scorer = BestVenueScorer(base_symbol="ETH")
    results = scorer.best_per_token(batch)
    for r in sorted(results, key=lambda x: x.token_symbol):
        print(
            f"{r.token_symbol:>5}: {r.tokens_out_per_base:,.6f} per ETH via {r.route_label} (block {r.block_number})"
        )


if __name__ == "__main__":
    main()

