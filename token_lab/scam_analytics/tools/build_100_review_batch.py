#!/usr/bin/env python3
"""Build token-centric scam review rows from token-server pool state."""

from __future__ import annotations

import argparse
import csv
import json
import math
import time
import urllib.error
import urllib.parse
import urllib.request
from collections import Counter
from pathlib import Path
from typing import Any


DEFAULT_API_BASE = "http://127.0.0.1:8765/eth/tokens/api"
DEFAULT_RUN_ID = "run-2"
ETH_SECONDS_PER_BLOCK = 12.0

REVIEW_COLUMNS = [
    "review_id",
    "chain",
    "source_run_id",
    "source",
    "token",
    "pool",
    "protocol",
    "symbol",
    "label",
    "mechanism",
    "mechanism_label",
    "confidence",
    "pool_created_block",
    "trading_enabled_block",
    "trading_enabled_source",
    "first_observed_trade_block",
    "first_blocked_block",
    "first_drain_block",
    "label_block",
    "blocks_from_trading_enabled_to_label",
    "minutes_from_trading_enabled_to_label",
    "time_to_scam_bucket",
    "price_ratio_to_initial_at_label",
    "liquidity_eth_at_label",
    "liquidity_level_at_label",
    "can_buy",
    "can_sell",
    "has_observed_buy",
    "has_observed_sell",
    "evidence_block",
    "evidence_tx",
    "evidence_summary",
    "evidence_ref",
    "notes",
    "review_status",
]


def main() -> None:
    args = parse_args()
    api = Api(args.api_base)
    progress = api.get(f"/runs/{urllib.parse.quote(args.run_id)}/progress")
    pools = get_nonempty_run_view(
        api,
        args.run_id,
        "pools",
        "pools",
        "indexed_pools",
        args.view_retries,
        args.view_retry_delay_secs,
    )

    rows = build_rows(args.run_id, pools, args.limit, args.review_prefix)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.summary.parent.mkdir(parents=True, exist_ok=True)

    with args.output.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=REVIEW_COLUMNS)
        writer.writeheader()
        writer.writerows(rows)

    args.summary.write_text(render_summary(args.run_id, progress, rows, len(pools)), encoding="utf-8")

    print(f"wrote {len(rows)} review rows to {args.output}")
    print(f"wrote summary to {args.summary}")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--api-base", default=DEFAULT_API_BASE)
    parser.add_argument("--run-id", default=DEFAULT_RUN_ID)
    parser.add_argument("--limit", type=int, default=100, help="maximum rows; 0 exports all")
    parser.add_argument("--review-prefix", default="scam100")
    parser.add_argument(
        "--output",
        type=Path,
        default=Path("token_lab/scam_analytics/labels/scam_pools_100_review.csv"),
    )
    parser.add_argument(
        "--summary",
        type=Path,
        default=Path("token_lab/scam_analytics/artifacts/reports/scam_pools_100_summary.md"),
    )
    parser.add_argument(
        "--view-retries",
        type=int,
        default=30,
        help="retry count for transient empty range views while the active run swaps processors",
    )
    parser.add_argument(
        "--view-retry-delay-secs",
        type=float,
        default=2.0,
        help="delay between transient empty range-view retries",
    )
    return parser.parse_args()


class Api:
    def __init__(self, base_url: str) -> None:
        self.base_url = base_url.rstrip("/")

    def get(self, path: str) -> dict[str, Any]:
        url = self.base_url + "/" + path.lstrip("/")
        request = urllib.request.Request(url, headers={"accept": "application/json"})
        try:
            with urllib.request.urlopen(request, timeout=60) as response:
                raw = response.read()
        except urllib.error.HTTPError as error:
            body = error.read().decode("utf-8", errors="replace")
            raise RuntimeError(f"GET {url} failed with HTTP {error.code}: {body}") from error
        except urllib.error.URLError as error:
            raise RuntimeError(f"GET {url} failed: {error}") from error
        return json.loads(raw.decode("utf-8"))


def get_nonempty_run_view(
    api: Api,
    run_id: str,
    endpoint: str,
    field: str,
    progress_count_field: str,
    retries: int,
    delay_secs: float,
) -> list[dict[str, Any]]:
    path = f"/runs/{urllib.parse.quote(run_id)}/{endpoint}"
    progress_path = f"/runs/{urllib.parse.quote(run_id)}/progress"
    attempts = max(retries, 0) + 1
    last_progress: dict[str, Any] = {}
    for attempt in range(attempts):
        response = api.get(path)
        rows = response.get(field) or []
        if rows:
            return rows
        last_progress = api.get(progress_path)
        expected_count = integer(last_progress.get(progress_count_field)) or 0
        if expected_count == 0:
            return rows
        if attempt < attempts - 1:
            time.sleep(delay_secs)

    raise RuntimeError(
        "{endpoint} view for run {run_id} returned 0 rows after {attempts} attempts, "
        "but progress reports {count} {progress_count_field}; retry after the active block apply "
        "restores the range processor".format(
            endpoint=endpoint,
            run_id=run_id,
            attempts=attempts,
            count=last_progress.get(progress_count_field),
            progress_count_field=progress_count_field,
        )
    )


def build_rows(
    run_id: str,
    pools: list[dict[str, Any]],
    limit: int,
    review_prefix: str = "scam100",
) -> list[dict[str, str]]:
    scam_pools = [pool for pool in pools if pool.get("scam_mechanism")]
    scam_pools.sort(key=pool_sort_key)
    rows = []
    seen: set[tuple[str, str]] = set()

    for pool in scam_pools:
        token = str(pool.get("token_address") or "").lower()
        pool_address = str(pool.get("pool_address") or "").lower()
        key = (token, pool_address)
        if not token or not pool_address or key in seen:
            continue
        seen.add(key)
        rows.append(row_from_pool(len(rows) + 1, run_id, pool, review_prefix))
        if limit > 0 and len(rows) >= limit:
            break

    return rows


def pool_sort_key(pool: dict[str, Any]) -> tuple[int, float, str, str]:
    label_block = label_block_for_pool(pool) or 10**18
    liquidity = number(pool.get("total_liquidity")) or 0.0
    return (
        int(label_block),
        -liquidity,
        str(pool.get("token_address") or ""),
        str(pool.get("pool_address") or ""),
    )


def row_from_pool(
    index: int,
    run_id: str,
    pool: dict[str, Any],
    review_prefix: str = "scam100",
) -> dict[str, str]:
    evidence = pool.get("scam_mechanism_evidence") or {}
    drain = evidence.get("drain") or {}
    trading_enabled_block, trading_enabled_source = trading_enabled(pool)
    label_block = label_block_for_pool(pool)
    first_drain_block = integer(drain.get("block_number")) or integer(pool.get("liquidity_removal_block"))
    first_blocked_block = first_blocked_for_pool(pool, evidence)
    blocks_to_label = diff(label_block, trading_enabled_block)
    minutes_to_label = (
        blocks_to_label * ETH_SECONDS_PER_BLOCK / 60.0 if blocks_to_label is not None else None
    )
    mechanism = str(pool.get("scam_mechanism") or "")
    confidence = confidence_for_pool(pool, evidence)

    return {
        "review_id": f"{review_prefix}-{index:03d}",
        "chain": "ethereum",
        "source_run_id": run_id,
        "source": "token_server_pool_scam_mechanism",
        "token": lower(pool.get("token_address")),
        "pool": lower(pool.get("pool_address")),
        "protocol": str(pool.get("protocol") or ""),
        "symbol": str(pool.get("token_symbol") or ""),
        "label": "scammed_pool",
        "mechanism": mechanism,
        "mechanism_label": str(pool.get("scam_mechanism_label") or ""),
        "confidence": confidence,
        "pool_created_block": text_int(pool.get("creation_block")),
        "trading_enabled_block": text_int(trading_enabled_block),
        "trading_enabled_source": trading_enabled_source,
        "first_observed_trade_block": text_int(trading_enabled_block),
        "first_blocked_block": text_int(first_blocked_block),
        "first_drain_block": text_int(first_drain_block),
        "label_block": text_int(label_block),
        "blocks_from_trading_enabled_to_label": text_int(blocks_to_label),
        "minutes_from_trading_enabled_to_label": number_text(minutes_to_label, digits=2),
        "time_to_scam_bucket": time_bucket(blocks_to_label),
        "price_ratio_to_initial_at_label": number_text(pool.get("price_ratio_to_initial"), digits=8),
        "liquidity_eth_at_label": number_text(pool.get("total_liquidity"), digits=8),
        "liquidity_level_at_label": str(pool.get("liquidity_level") or ""),
        "can_buy": bool_text(pool.get("can_buy")),
        "can_sell": bool_text(pool.get("can_sell")),
        "has_observed_buy": bool_text(pool.get("has_observed_buy")),
        "has_observed_sell": bool_text(pool.get("has_observed_sell")),
        "evidence_block": text_int(evidence_block(evidence, pool)),
        "evidence_tx": evidence_tx(evidence, pool),
        "evidence_summary": evidence_summary(mechanism, evidence, pool),
        "evidence_ref": (
            f"http://127.0.0.1:40019/eth/tokens/{pool.get('token_address')}/"
            f"?run_id={urllib.parse.quote(run_id)}"
        ),
        "notes": "Token-centric label from pool scam mechanism; strategy outcome ignored.",
        "review_status": "draft_needs_manual_review",
    }


def trading_enabled(pool: dict[str, Any]) -> tuple[int | None, str]:
    status = pool.get("trading_status") or {}
    for source, value in (
        ("trading_status.block", status.get("block")),
        ("can_buy_block", pool.get("can_buy_block")),
        ("pool_creation_fallback", pool.get("creation_block")),
    ):
        block = integer(value)
        if block is not None:
            return block, source
    return None, ""


def label_block_for_pool(pool: dict[str, Any]) -> int | None:
    evidence = pool.get("scam_mechanism_evidence") or {}
    drain = evidence.get("drain") or {}
    return (
        integer(drain.get("block_number"))
        or integer(pool.get("liquidity_removal_block"))
        or integer(pool.get("latest_block_number"))
    )


def confidence_for_pool(pool: dict[str, Any], evidence: dict[str, Any]) -> str:
    drain = evidence.get("drain") or {}
    burn = evidence.get("burn_event")
    suspicious_pair_transfer = evidence.get("suspicious_pair_token_out_transfer")
    swap = evidence.get("swap_event")
    mechanism = str(pool.get("scam_mechanism") or "")

    if mechanism == "direct_lp_liquidity_removal" and drain and burn:
        return "verified"
    if mechanism == "pair_balance_backdoor_drain" and drain and suspicious_pair_transfer:
        return "verified"
    if mechanism in {"reserve_dump_drain", "privileged_seller_reserve_drain"} and drain and swap:
        return "probable"
    if drain:
        return "probable"
    return "needs_mechanism_review"


def first_blocked_for_pool(pool: dict[str, Any], evidence: dict[str, Any]) -> int | None:
    mechanism = str(pool.get("scam_mechanism") or "")
    risk_label = str(pool.get("risk_label") or "")
    failure_reason = str(evidence.get("last_trading_failure_reason") or "")
    if mechanism in {"sell_blocked_honeypot", "privileged_seller_reserve_drain"}:
        return label_block_for_pool(pool)
    if risk_label == "cannot_sell" and failure_reason:
        return label_block_for_pool(pool)
    return None


def evidence_block(evidence: dict[str, Any], pool: dict[str, Any]) -> int | None:
    drain = evidence.get("drain") or {}
    burn = evidence.get("burn_event") or {}
    pair = evidence.get("suspicious_pair_token_out_transfer") or {}
    swap = evidence.get("swap_event") or {}
    return (
        integer(drain.get("block_number"))
        or integer(burn.get("block"))
        or integer(pair.get("block_number"))
        or integer(swap.get("block"))
        or integer(swap.get("block_number"))
        or integer(pool.get("latest_block_number"))
    )


def evidence_tx(evidence: dict[str, Any], pool: dict[str, Any]) -> str:
    drain = evidence.get("drain") or {}
    burn = evidence.get("burn_event") or {}
    pair = evidence.get("suspicious_pair_token_out_transfer") or {}
    swap = evidence.get("swap_event") or {}
    return str(
        drain.get("tx_hash")
        or burn.get("tx_hash")
        or pair.get("tx_hash")
        or swap.get("tx_hash")
        or pool.get("liquidity_removal_tx_hash")
        or ""
    )


def evidence_summary(mechanism: str, evidence: dict[str, Any], pool: dict[str, Any]) -> str:
    parts = [f"mechanism={mechanism}"]
    drain = evidence.get("drain") or {}
    if drain:
        ratio = number_text(drain.get("denom_reserve_ratio_to_max"), digits=6)
        parts.append(
            "drain "
            f"block={drain.get('block_number')} "
            f"reserve_ratio_to_max={ratio} "
            f"source={drain.get('source') or ''}"
        )
    if evidence.get("burn_event"):
        parts.append("burn_event_present=true")
    if evidence.get("suspicious_pair_token_out_transfer"):
        parts.append("pair_token_out_transfer_present=true")
    if evidence.get("last_trading_failure_reason"):
        parts.append("sell_failure_present=true")
    parts.append(f"liquidity={number_text(pool.get('total_liquidity'), digits=6)}")
    return "; ".join(parts)


def render_summary(
    run_id: str,
    progress: dict[str, Any],
    rows: list[dict[str, str]],
    pool_count: int,
) -> str:
    mechanisms = Counter(row["mechanism"] for row in rows)
    confidence = Counter(row["confidence"] for row in rows)
    buckets = Counter(row["time_to_scam_bucket"] for row in rows)
    protocols = Counter(row["protocol"] for row in rows)

    lines = [
        "# Scam Pools Review Summary",
        "",
        f"Source run: `{run_id}`",
        "",
        "This is a token-centric scam review export. Labels come from",
        "token-server pool `scam_mechanism` evidence. Strategy entry, exit,",
        "and PnL are intentionally ignored. Use `--limit 100` for the first",
        "manual-review batch, or `--limit 0` for all currently available rows.",
        "",
        "## Source Progress",
        "",
        f"- Range: `{progress.get('start_block')}..{progress.get('end_block')}`",
        f"- Status at export: `{progress.get('status')}`",
        f"- Blocks processed at export: `{progress.get('blocks_processed')}` / `{progress.get('total_blocks')}`",
        f"- Pools available at export: `{pool_count}`",
        f"- Rows exported: `{len(rows)}`",
        "",
        "## Mechanisms",
        "",
        table(mechanisms, "Mechanism"),
        "",
        "## Confidence",
        "",
        table(confidence, "Confidence"),
        "",
        "## Time To Scam Buckets",
        "",
        table(buckets, "Bucket"),
        "",
        "## Protocols",
        "",
        table(protocols, "Protocol"),
        "",
        "## Manual Review Notes",
        "",
        "- `verified` means the token-server evidence includes the expected chain",
        "  artifact for that mechanism, such as LP burn/removal or pair-balance",
        "  transfer evidence.",
        "- `probable` means the reserve-drain evidence is clear but the exact actor",
        "  mechanism should still be manually checked.",
        "- This batch is a draft review artifact. Do not treat it as a final",
        "  production label set until the 100 rows are manually audited.",
        "",
    ]
    return "\n".join(lines)


def table(counter: Counter[str], label: str) -> str:
    lines = [f"| {label} | Count |", "| --- | ---: |"]
    for key, value in sorted(counter.items(), key=lambda item: (-item[1], item[0])):
        lines.append(f"| `{key or 'unknown'}` | {value} |")
    return "\n".join(lines)


def time_bucket(blocks: int | None) -> str:
    if blocks is None:
        return "unknown"
    if blocks == 0:
        return "same_block"
    if 1 <= blocks <= 10:
        return "fast_1_to_10"
    if 11 <= blocks <= 100:
        return "early_11_to_100"
    if 101 <= blocks <= 1000:
        return "delayed_101_to_1000"
    return "late_gt_1000"


def diff(left: int | None, right: int | None) -> int | None:
    if left is None or right is None:
        return None
    return left - right


def lower(value: Any) -> str:
    return str(value or "").lower()


def bool_text(value: Any) -> str:
    if value is None:
        return ""
    return "true" if bool(value) else "false"


def text_int(value: Any) -> str:
    parsed = integer(value)
    return "" if parsed is None else str(parsed)


def integer(value: Any) -> int | None:
    if value is None or value == "":
        return None
    try:
        return int(value)
    except (TypeError, ValueError):
        return None


def number(value: Any) -> float | None:
    if value is None or value == "":
        return None
    try:
        parsed = float(value)
    except (TypeError, ValueError):
        return None
    if not math.isfinite(parsed):
        return None
    return parsed


def number_text(value: Any, digits: int) -> str:
    parsed = number(value)
    if parsed is None:
        return ""
    return f"{parsed:.{digits}g}"


if __name__ == "__main__":
    main()
