"""
Analyze simple 2-hop arbitrage between WETH/USDC and WETH/USDT Uniswap V2 pools
over the last 10k blocks. Uses reserves at sampled blocks and estimates maximum
theoretical profit per sample using a grid search (includes 0.30% fee each leg).

Outputs:
- Gross profit opportunities (USD ~ stables)
- Net-of-gas profit under multiple gas price assumptions
- Daily and monthly extrapolations based on observed window duration

Usage (from repo root):
  PYTHONPATH=$(pwd) STEP_BLOCKS=100 python -m eth_price_leverage.examples.analytics.analyze_last_10k_arbitrage

Env vars:
- STEP_BLOCKS: sample step in blocks (default 100)
- WINDOW_BLOCKS: window size (default 10_000)
"""

from __future__ import annotations

import csv
import math
import os
import statistics
import subprocess
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable, List, Optional, Tuple


@dataclass
class Sample:
    block: int
    ts: int
    cl_eth_usd: float
    usdc_weth_reserve: float
    usdc_stable_reserve: float
    usdt_weth_reserve: float
    usdt_stable_reserve: float
    u2c_price: float
    u2t_price: float


def run_reserves_timeseries() -> List[Sample]:
    repo_root = Path(__file__).resolve().parents[5]
    cargo_toml = repo_root / "rust" / "eth_prices" / "Cargo.toml"
    env = os.environ.copy()
    # Allow caller to set STEP_BLOCKS/WINDOW_BLOCKS; defaults apply in Rust
    cmd = [
        "cargo",
        "run",
        "--manifest-path",
        str(cargo_toml),
        "--example",
        "last_10k_blocks_weth_stables_reserves",
        "--release",
    ]
    proc = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, env=env)
    assert proc.stdout is not None
    reader = csv.reader((line for line in proc.stdout))
    rows: List[Sample] = []
    for fields in reader:
        # Expect: block,timestamp,chainlink,usdc_weth,usdc_stable,usdt_weth,usdt_stable,u2c_price,u2t_price
        if len(fields) != 9:
            continue
        if fields[0] == "block":
            continue
        try:
            block = int(fields[0])
            ts = int(fields[1])
            cl = float(fields[2])
            uw = float(fields[3])
            uc = float(fields[4])
            tw = float(fields[5])
            tt = float(fields[6])
            u2c = float(fields[7])
            u2t = float(fields[8])
        except ValueError:
            continue
        rows.append(Sample(block, ts, cl, uw, uc, tw, tt, u2c, u2t))
    proc.wait()
    if proc.returncode != 0:
        raise SystemExit(f"Rust example exited with code {proc.returncode}")
    return rows


def amount_out(amount_in: float, reserve_in: float, reserve_out: float, gamma: float = 0.997) -> float:
    if amount_in <= 0 or reserve_in <= 0 or reserve_out <= 0:
        return 0.0
    a_in_fee = amount_in * gamma
    return (a_in_fee * reserve_out) / (reserve_in + a_in_fee)


def route_profit_usdc_to_usdt(usdc_in: float, uw: float, uc: float, tw: float, tt: float) -> float:
    # USDC -> WETH on USDC pool, then WETH -> USDT on USDT pool
    eth_out = amount_out(usdc_in, uc, uw)
    usdt_out = amount_out(eth_out, tw, tt)
    return usdt_out - usdc_in


def route_profit_usdt_to_usdc(usdt_in: float, tw: float, tt: float, uw: float, uc: float) -> float:
    # USDT -> WETH on USDT pool, then WETH -> USDC on USDC pool
    eth_out = amount_out(usdt_in, tt, tw)
    usdc_out = amount_out(eth_out, uw, uc)
    return usdc_out - usdt_in


def best_two_hop_profit(uw: float, uc: float, tw: float, tt: float) -> Tuple[float, str, float]:
    """Return (max_profit, direction, optimal_input) where direction in {"USDC->USDT","USDT->USDC"}.
    Simple grid search up to 10% of min stable reserves.
    """
    if min(uw, uc, tw, tt) <= 0:
        return 0.0, "none", 0.0
    cap = 0.10 * min(uc, tt)
    if cap <= 0:
        return 0.0, "none", 0.0
    steps = 64
    best_p, best_dir, best_in = 0.0, "none", 0.0
    # USDC -> USDT
    for i in range(1, steps + 1):
        x = cap * i / steps
        p = route_profit_usdc_to_usdt(x, uw, uc, tw, tt)
        if p > best_p:
            best_p, best_dir, best_in = p, "USDC->USDT", x
    # USDT -> USDC
    for i in range(1, steps + 1):
        x = cap * i / steps
        p = route_profit_usdt_to_usdc(x, tw, tt, uw, uc)
        if p > best_p:
            best_p, best_dir, best_in = p, "USDT->USDC", x
    return best_p, best_dir, best_in


def summarize(rows: List[Sample]) -> None:
    if not rows:
        print("No data rows")
        return
    t0, t1 = rows[0].ts, rows[-1].ts
    duration = max(1, t1 - t0)
    hours = duration / 3600.0
    print(f"Window: {len(rows)} samples spanning ~{hours:.2f} hours")

    # Basic spread stats vs Chainlink
    def rel_diff(a: float, b: float) -> float:
        if a <= 0 or b <= 0:
            return 0.0
        return abs(a - b) / b

    deltas_u2c = [rel_diff(r.u2c_price, r.cl_eth_usd) for r in rows if r.cl_eth_usd > 0 and r.u2c_price > 0]
    deltas_u2t = [rel_diff(r.u2t_price, r.cl_eth_usd) for r in rows if r.cl_eth_usd > 0 and r.u2t_price > 0]
    if deltas_u2c and deltas_u2t:
        p50_c, p95_c, p99_c = statistics.median(deltas_u2c), statistics.quantiles(deltas_u2c, n=100)[94], statistics.quantiles(deltas_u2c, n=100)[98]
        p50_t, p95_t, p99_t = statistics.median(deltas_u2t), statistics.quantiles(deltas_u2t, n=100)[94], statistics.quantiles(deltas_u2t, n=100)[98]
        print("DEX vs Chainlink relative deviation:")
        print(f"  USDC pool: median={p50_c*100:.3f}% | p95={p95_c*100:.3f}% | p99={p99_c*100:.3f}%")
        print(f"  USDT pool: median={p50_t*100:.3f}% | p95={p95_t*100:.3f}% | p99={p99_t*100:.3f}%")

    opps = []
    gross_total = 0.0
    per_sample = []
    for r in rows:
        p, direction, x = best_two_hop_profit(r.usdc_weth_reserve, r.usdc_stable_reserve, r.usdt_weth_reserve, r.usdt_stable_reserve)
        if p > 0:
            opps.append((r.block, r.ts, p, direction, x, r.cl_eth_usd))
            gross_total += p
            per_sample.append(p)

    if not opps:
        print("No gross-positive arbitrage opportunities in sampled data.")
        return

    print(f"Gross-positive samples: {len(opps)} / {len(rows)} ({100.0*len(opps)/len(rows):.1f}%)")
    print(f"Gross profit sum: ${gross_total:,.2f} (assuming 1 USDC ~= 1 USDT)")
    print(f"Gross profit per day (normalized): ${gross_total * 86400.0 / duration:,.2f}")
    print(f"Gross profit per 30-day month:   ${gross_total * 86400.0 / duration * 30:,.2f}")
    if per_sample:
        print(f"Median gross per profitable sample: ${statistics.median(per_sample):.2f}")

    # Net-of-gas estimates for a few gas price scenarios
    print("\nNet-of-gas estimates (2 swaps, ~240k gas):")
    gas_units = 240_000
    for gwei in (10.0, 20.0, 30.0, 50.0):
        net = 0.0
        prof_n = 0
        for _b, _ts, p, _dir, _x, cl in opps:
            # Gas USD = gas * gwei * 1e-9 ETH/gwei * ETH/USD
            if cl <= 0:
                continue
            gas_usd = gas_units * gwei * 1e-9 * cl
            if p > gas_usd:
                net += p - gas_usd
                prof_n += 1
        daily = net * 86400.0 / duration
        monthly = daily * 30
        print(f"  {gwei:>4.0f} gwei: net=${net:,.2f} | daily=${daily:,.2f} | 30d=${monthly:,.2f} | profitable_samples={prof_n}")


def main() -> None:
    rows = run_reserves_timeseries()
    summarize(rows)


if __name__ == "__main__":
    main()
