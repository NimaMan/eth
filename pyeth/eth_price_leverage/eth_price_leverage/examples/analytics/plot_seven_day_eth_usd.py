"""
Generate a figure of the last 7 days ETH/USD using:
- Chainlink ETH/USD
- Uniswap V2 WETH/USDC (USDC per ETH)
- Uniswap V2 WETH/USDT (USDT per ETH)

It shells out to the Rust example that streams CSV and plots it with matplotlib.

Usage (from repo root):
  PYTHONPATH=$(pwd) python -m eth_price_leverage.examples.analytics.plot_seven_day_eth_usd
"""

from __future__ import annotations

import csv
import subprocess
from datetime import datetime, timezone
from pathlib import Path
from typing import List, Tuple

import matplotlib.pyplot as plt


def run_rust_timeseries() -> List[Tuple[int, int, float, float, float]]:
    repo_root = Path(__file__).resolve().parents[5]
    # rust/eth_prices/Cargo.toml relative to repo root
    cargo_toml = repo_root / "rust" / "eth_prices" / "Cargo.toml"
    cmd = [
        "cargo", "run", "--manifest-path", str(cargo_toml),
        "--example", "seven_day_eth_usd_timeseries", "--release",
    ]
    proc = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)

    rows: List[Tuple[int, int, float, float, float]] = []
    assert proc.stdout is not None
    reader = csv.reader((line for line in proc.stdout))

    header_seen = False
    for fields in reader:
        # Filter out non-CSV debug lines (e.g., lines not having 5 fields)
        if len(fields) != 5:
            continue
        if fields[0] == "timestamp":
            header_seen = True
            continue
        try:
            ts = int(fields[0])
            block = int(fields[1])
            chainlink = float(fields[2])
            usdc = float(fields[3])
            usdt = float(fields[4])
        except ValueError:
            continue
        rows.append((ts, block, chainlink, usdc, usdt))

    proc.wait()
    if proc.returncode != 0:
        raise SystemExit(f"Rust example exited with code {proc.returncode}")
    if not header_seen:
        raise SystemExit("Did not receive CSV header; ensure the example ran correctly")
    return rows


def plot(rows: List[Tuple[int, int, float, float, float]]) -> Path:
    if not rows:
        raise SystemExit("No rows to plot")
    # Convert timestamps
    xs = [datetime.fromtimestamp(r[0], tz=timezone.utc) for r in rows]
    cl = [r[2] for r in rows]
    u2c = [r[3] for r in rows]
    u2t = [r[4] for r in rows]

    fig, ax = plt.subplots(figsize=(11, 6))
    ax.plot(xs, cl, label="Chainlink ETH/USD", color="#3b82f6", linewidth=2.0)
    ax.plot(xs, u2c, label="Uniswap V2 USDC per ETH", color="#10b981", alpha=0.8)
    ax.plot(xs, u2t, label="Uniswap V2 USDT per ETH", color="#f59e0b", alpha=0.8)

    ax.set_title("ETH/USD — last 7 days")
    ax.set_xlabel("Time (UTC)")
    ax.set_ylabel("Quote per 1 ETH")
    ax.grid(True, linestyle=":", alpha=0.4)
    ax.legend(loc="best")

    fig.autofmt_xdate()
    out_dir = Path(__file__).resolve().parents[5] / "artifacts"
    out_dir.mkdir(parents=True, exist_ok=True)
    out_path = out_dir / "eth_usd_last_7_days.png"
    fig.tight_layout()
    fig.savefig(out_path, dpi=160)
    plt.close(fig)
    return out_path


def main() -> None:
    rows = run_rust_timeseries()
    out = plot(rows)
    print(f"Saved figure: {out}")


if __name__ == "__main__":
    main()

