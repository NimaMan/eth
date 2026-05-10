#!/usr/bin/env python3
"""Assess pool classification quality against actual outcomes.

Reads pool data from the token server API and correlates risk factors
with performance. Works with both live endpoints and completed range runs.

Usage:
    python classification_assessment.py --api http://127.0.0.1:8765 --source live
    python classification_assessment.py --api http://127.0.0.1:8765 --source range --run latest
"""

from __future__ import annotations

import argparse
import json
import os
import sys
import urllib.error
import urllib.request
from collections import defaultdict
from dataclasses import dataclass, field
from typing import Any, Sequence

from range_triage_utils import (
    as_int,
    finite_number,
    nested_get,
    url_quote,
)

DEFAULT_API_BASE = "http://127.0.0.1:8765"


class ApiClient:
    def __init__(self, base_url: str, timeout_secs: float = 30.0) -> None:
        self.base_url = base_url.rstrip("/")
        self.timeout_secs = timeout_secs

    def get(self, path: str) -> dict[str, Any]:
        url = self.base_url + "/" + path.lstrip("/")
        request = urllib.request.Request(url, headers={"accept": "application/json"})
        try:
            with urllib.request.urlopen(request, timeout=self.timeout_secs) as response:
                raw = response.read()
        except urllib.error.HTTPError as error:
            body = error.read().decode("utf-8", errors="replace")
            raise RuntimeError(f"GET {url} failed with HTTP {error.code}: {body}") from error
        except urllib.error.URLError as error:
            raise RuntimeError(f"GET {url} failed: {error}") from error

        try:
            return json.loads(raw.decode("utf-8"))
        except json.JSONDecodeError as error:
            preview = raw[:240].decode("utf-8", errors="replace")
            raise RuntimeError(f"GET {url} returned invalid JSON: {preview}") from error


@dataclass
class PoolAssessment:
    token_address: str
    pool_address: str
    symbol: str
    protocol: str
    currency: str

    # Classification
    category: str
    eligible_outcome: str | None
    non_eligible_reason: str | None

    # Risk factors
    lp_approved_percentage: float | None
    lp_max_holder_share: float | None
    supply_ratio_status: str
    ownership_renounced: bool | None
    tax_bucket: str
    hidden_mint: bool
    liquidity_removed: bool
    honeypot: bool

    # Performance (from price_ratio_history if available)
    has_price_history: bool
    peak_ratio: float | None
    final_ratio: float | None
    crossed_2x: bool
    crossed_10x: bool
    crossed_100x: bool
    rugged: bool  # final ratio < 0.1x or liquidity_removed


def load_live_pools(client: ApiClient) -> list[dict[str, Any]]:
    data = client.get("/live/pools")
    return list(data.get("pools") or [])


def load_range_pools(client: ApiClient, run_id: str) -> list[dict[str, Any]]:
    data = client.get(f"/runs/{url_quote(run_id)}/pools")
    pools = list(data.get("pools") or [])
    if not pools:
        # Try nested structure
        pools = list(data.get("data", {}).get("pools") or [])
    return pools


def resolve_run_id(client: ApiClient, selector: str) -> str:
    if selector == "active":
        progress = client.get("/runs/active")
        rid = progress.get("id")
        if not rid:
            raise RuntimeError("No active run")
        return str(rid)
    if selector == "latest":
        runs = client.get("/runs")
        run_list = list(runs.get("runs") or [])
        if not run_list:
            raise RuntimeError("No runs available")
        run_list.sort(
            key=lambda r: (
                as_int(r.get("updated_at_unix_secs")) or 0,
                as_int(r.get("started_at_unix_secs")) or 0,
                str(r.get("id") or ""),
            ),
            reverse=True,
        )
        return str(run_list[0].get("id") or "")
    return selector


def assess_pool(pool: dict[str, Any]) -> PoolAssessment:
    classification = pool.get("pool_classification") or {}
    price_history = pool.get("price_ratio_history") or []

    ratios = [
        finite_number(point.get("ratio"))
        for point in price_history
        if isinstance(point, dict)
    ]
    ratios = [r for r in ratios if r is not None and r > 0]

    peak = max(ratios) if ratios else None
    final = ratios[-1] if ratios else None
    rugged = bool(pool.get("liquidity_removed")) or (final is not None and final < 0.1)

    # LP max holder share
    lp_holders = pool.get("lp_holders") or []
    max_share = None
    for holder in lp_holders:
        share = finite_number(holder.get("share"))
        if share is not None:
            if max_share is None or share > max_share:
                max_share = share

    return PoolAssessment(
        token_address=pool.get("token_address") or "",
        pool_address=pool.get("pool_address") or "",
        symbol=pool.get("token_symbol") or pool.get("symbol") or "-",
        protocol=pool.get("protocol") or "-",
        currency=pool.get("currency") or "-",
        category=classification.get("category") or "unknown",
        eligible_outcome=classification.get("eligible_outcome"),
        non_eligible_reason=classification.get("reason_key"),
        lp_approved_percentage=finite_number(pool.get("lp_approved_percentage")),
        lp_max_holder_share=max_share,
        supply_ratio_status=str(pool.get("supply_ratio_status") or "unknown"),
        ownership_renounced=nested_get(pool, "token", "ownership_renounced")
        if nested_get(pool, "token", "ownership_renounced") is not None
        else None,
        tax_bucket=str(pool.get("tax_bucket") or "unknown"),
        hidden_mint=bool(pool.get("hidden_mint")),
        liquidity_removed=bool(pool.get("liquidity_removed")),
        honeypot=bool(pool.get("honeypot")),
        has_price_history=len(price_history) > 0,
        peak_ratio=peak,
        final_ratio=final,
        crossed_2x=any(r >= 2.0 for r in ratios) if ratios else False,
        crossed_10x=any(r >= 10.0 for r in ratios) if ratios else False,
        crossed_100x=any(r >= 100.0 for r in ratios) if ratios else False,
        rugged=rugged,
    )


def print_summary(assessments: list[PoolAssessment]) -> None:
    total = len(assessments)
    if total == 0:
        print("No pools to assess.")
        return

    # Classification distribution
    categories = defaultdict(int)
    outcomes = defaultdict(int)
    for a in assessments:
        categories[a.category] += 1
        if a.eligible_outcome:
            outcomes[a.eligible_outcome] += 1
        elif a.non_eligible_reason:
            outcomes[a.non_eligible_reason] += 1

    print(f"\n=== Classification Distribution ({total} pools) ===")
    for cat, count in sorted(categories.items(), key=lambda x: -x[1]):
        pct = count / total * 100
        print(f"  {cat}: {count} ({pct:.1f}%)")

    print(f"\n=== Outcomes ===")
    for out, count in sorted(outcomes.items(), key=lambda x: -x[1]):
        pct = count / total * 100
        print(f"  {out}: {count} ({pct:.1f}%)")

    # Risk factor presence
    print(f"\n=== Risk Factor Presence ===")
    factors = {
        "lp_approved > 0": sum(1 for a in assessments if (a.lp_approved_percentage or 0) > 0),
        "lp_approved >= 20%": sum(1 for a in assessments if (a.lp_approved_percentage or 0) >= 20),
        "lp_max_share >= 50%": sum(1 for a in assessments if (a.lp_max_holder_share or 0) >= 50),
        "lp_max_share >= 90%": sum(1 for a in assessments if (a.lp_max_holder_share or 0) >= 90),
        "supply_ratio inconsistent": sum(1 for a in assessments if a.supply_ratio_status == "inconsistent"),
        "ownership not renounced": sum(1 for a in assessments if a.ownership_renounced is False),
        "hidden_mint": sum(1 for a in assessments if a.hidden_mint),
        "liquidity_removed": sum(1 for a in assessments if a.liquidity_removed),
        "honeypot": sum(1 for a in assessments if a.honeypot),
        "extreme_tax": sum(1 for a in assessments if a.tax_bucket == "extreme_tax"),
        "high_tax": sum(1 for a in assessments if a.tax_bucket == "high_tax"),
        "has_price_history": sum(1 for a in assessments if a.has_price_history),
        "rugged": sum(1 for a in assessments if a.rugged),
    }
    for label, count in factors.items():
        pct = count / total * 100
        print(f"  {label}: {count} ({pct:.1f}%)")

    # Performance by category (only for pools with price history)
    with_history = [a for a in assessments if a.has_price_history]
    if with_history:
        print(f"\n=== Performance (pools with price history: {len(with_history)}) ===")
        for cat in ["eligible_active", "eligible_risk", "ineligible"]:
            subset = [a for a in with_history if a.category == cat]
            if not subset:
                continue
            winners_2x = sum(1 for a in subset if a.crossed_2x)
            winners_10x = sum(1 for a in subset if a.crossed_10x)
            rugged_count = sum(1 for a in subset if a.rugged)
            avg_peak = sum(a.peak_ratio for a in subset if a.peak_ratio is not None) / len(subset)
            print(f"  {cat} ({len(subset)}):")
            print(f"    winners >=2x: {winners_2x} ({winners_2x/len(subset)*100:.1f}%)")
            print(f"    winners >=10x: {winners_10x} ({winners_10x/len(subset)*100:.1f}%)")
            print(f"    rugged: {rugged_count} ({rugged_count/len(subset)*100:.1f}%)")
            print(f"    avg peak ratio: {avg_peak:.2f}x")
    else:
        print(f"\n=== Performance ===")
        print("  No price history available in this dataset.")

    # Risk factor correlation (only for pools with price history)
    if with_history:
        print(f"\n=== Risk Factor Correlation (pools with price history) ===")
        risk_checks = [
            ("liquidity_removed", lambda a: a.liquidity_removed),
            ("honeypot", lambda a: a.honeypot),
            ("hidden_mint", lambda a: a.hidden_mint),
            ("extreme_tax", lambda a: a.tax_bucket == "extreme_tax"),
            ("high_tax", lambda a: a.tax_bucket == "high_tax"),
            ("lp_approved >= 20%", lambda a: (a.lp_approved_percentage or 0) >= 20),
            ("lp_max_share >= 90%", lambda a: (a.lp_max_holder_share or 0) >= 90),
            ("supply_ratio inconsistent", lambda a: a.supply_ratio_status == "inconsistent"),
            ("ownership not renounced", lambda a: a.ownership_renounced is False),
        ]
        for label, check in risk_checks:
            positive = [a for a in with_history if check(a)]
            negative = [a for a in with_history if not check(a)]
            if not positive:
                continue
            pos_rugged = sum(1 for a in positive if a.rugged) / len(positive)
            neg_rugged = sum(1 for a in negative if a.rugged) / len(negative) if negative else 0
            pos_winners = sum(1 for a in positive if a.crossed_2x) / len(positive)
            neg_winners = sum(1 for a in negative if a.crossed_2x) / len(negative) if negative else 0
            print(f"  {label}:")
            print(f"    count: {len(positive)}")
            print(f"    rugged: {pos_rugged*100:.1f}% (vs {neg_rugged*100:.1f}% without)")
            print(f"    winners >=2x: {pos_winners*100:.1f}% (vs {neg_winners*100:.1f}% without)")


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Assess pool classification quality.")
    parser.add_argument("--api", default=os.environ.get("ETH_TOKEN_API", DEFAULT_API_BASE))
    parser.add_argument("--source", choices=("live", "range"), default="live")
    parser.add_argument("--run", default="latest", help="run id for --source=range")
    args = parser.parse_args(argv or sys.argv[1:])

    client = ApiClient(args.api)

    try:
        if args.source == "live":
            pools = load_live_pools(client)
        else:
            run_id = resolve_run_id(client, args.run)
            print(f"Loading range run: {run_id}")
            pools = load_range_pools(client, run_id)
    except RuntimeError as e:
        print(f"Error: {e}", file=sys.stderr)
        return 1

    assessments = [assess_pool(p) for p in pools]
    print_summary(assessments)
    return 0


if __name__ == "__main__":
    sys.exit(main())
