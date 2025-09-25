"""
Generate ETH/USD last-10k-blocks time series directly via PyReth bindings.

Outputs a CSV with columns:
  block,timestamp,chainlink_eth_usd,univ2_usdc,univ2_usdt,usdc_weth_reserve,usdc_stable_reserve,usdt_weth_reserve,usdt_stable_reserve

Usage (from repo root, after building pyreth):
  PYTHONPATH=$(pwd) python rust/pyreth/examples/python/eth_usd_last_10k_blocks.py
"""

from __future__ import annotations

import csv
from pathlib import Path

import pyreth


def main() -> None:
    out_dir = Path(__file__).resolve().parents[4] / "artifacts"
    out_dir.mkdir(parents=True, exist_ok=True)
    out_csv = out_dir / "eth_usd_last_10k_blocks_pyreth.csv"

    client = pyreth.PyReth().price_client()
    # Ensure readers are initialized through PyReth (aggregated reader already configured)
    # Fetch last 10k, sample every 100 blocks, include reserves for both pools
    rows = client.get_eth_usd_last_n_blocks(window_blocks=10_000, step_blocks=100, include_reserves=True)

    with out_csv.open("w", newline="") as f:
        w = csv.writer(f)
        w.writerow([
            "block","timestamp","chainlink_eth_usd","univ2_usdc","univ2_usdt",
            "usdc_weth_reserve","usdc_stable_reserve","usdt_weth_reserve","usdt_stable_reserve",
        ])
        for r in rows:
            w.writerow([
                r["block"], r["timestamp"], r.get("chainlink_eth_usd", 0.0), r.get("univ2_usdc", 0.0), r.get("univ2_usdt", 0.0),
                r.get("usdc_weth_reserve", 0.0), r.get("usdc_stable_reserve", 0.0), r.get("usdt_weth_reserve", 0.0), r.get("usdt_stable_reserve", 0.0)
            ])

    print(f"Saved: {out_csv}")


if __name__ == "__main__":
    main()

