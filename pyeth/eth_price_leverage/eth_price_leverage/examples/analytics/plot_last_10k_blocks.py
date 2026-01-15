"""
Generate a figure over the last 10k blocks for:
- Chainlink ETH/USD
- Uniswap V2 WETH/USDC (USDC per ETH)
- Uniswap V2 WETH/USDT (USDT per ETH)

Usage (from repo root):
  PYTHONPATH=$(pwd) python -m eth_price_leverage.examples.analytics.plot_last_10k_blocks
"""

from __future__ import annotations

import csv
import subprocess
from pathlib import Path
from typing import List, Tuple

import matplotlib.pyplot as plt


def run_rust_timeseries() -> List[Tuple[int, int, float, float]]:
    repo_root = Path(__file__).resolve().parents[5]
    cargo_toml = repo_root / "rust" / "eth_prices" / "Cargo.toml"
    cmd = [
        "cargo", "run", "--manifest-path", str(cargo_toml),
        "--example", "last_10k_blocks_eth_usd_timeseries", "--release",
    ]
    proc = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    rows: List[Tuple[int, int, float, float]] = []
    assert proc.stdout is not None
    reader = csv.reader((line for line in proc.stdout))
    header_seen = False
    for fields in reader:
        # Expect: block, timestamp, chainlink, univ2_usdc, univ2_usdt
        if len(fields) != 5:
            continue
        if fields[0] == "block":
            header_seen = True
            continue
        try:
            block = int(fields[0])
            ts = int(fields[1])
            cl = float(fields[2])
            u2c = float(fields[3])
            u2t = float(fields[4])
        except ValueError:
            continue
        rows.append((block, ts, cl, u2c, u2t))
    proc.wait()
    if proc.returncode != 0:
        raise SystemExit(f"Rust example exited with code {proc.returncode}")
    if not header_seen:
        raise SystemExit("Did not receive CSV header; ensure the example ran correctly")
    return rows


def plot(rows: List[Tuple[int, int, float, float]]) -> Path:
    if not rows:
        raise SystemExit("No rows to plot")
    xs = [r[0] for r in rows]  # block numbers
    cl = [r[2] for r in rows]
    u2c = [r[3] for r in rows]
    u2t = [r[4] for r in rows]

    fig, ax = plt.subplots(figsize=(11, 6))
    ax.plot(xs, cl, label="Chainlink ETH/USD", color="#3b82f6", linewidth=1.8)
    ax.plot(xs, u2c, label="Uniswap V2 USDC per ETH", color="#10b981", alpha=0.8)
    ax.plot(xs, u2t, label="Uniswap V2 USDT per ETH", color="#f59e0b", alpha=0.8)

    ax.set_title("ETH/USD — last 10k blocks (sampled)")
    ax.set_xlabel("Block number")
    ax.set_ylabel("Quote per 1 ETH")
    ax.grid(True, linestyle=":", alpha=0.4)
    ax.legend(loc="best")

    fig.tight_layout()
    out_dir = Path(__file__).resolve().parents[5] / "artifacts"
    out_dir.mkdir(parents=True, exist_ok=True)
    out_path = out_dir / "eth_usd_last_10k_blocks.png"
    fig.savefig(out_path, dpi=160)
    plt.close(fig)
    return out_path


def main() -> None:
    rows = run_rust_timeseries()
    out = plot(rows)
    print(f"Saved figure: {out}")


if __name__ == "__main__":
    main()

