#!/usr/bin/env python3
"""Build the mempool-management pool cohort from live trading signals."""

from __future__ import annotations

import argparse
import csv
import json
import os
from datetime import datetime, timezone
from pathlib import Path
from statistics import median
from typing import Any

import psycopg2
import psycopg2.extras


DEFAULT_DATABASE_URL = "postgresql://postgres:postgres@localhost:5432/eth_db"
LIQUIDITY_REMOVAL_COHORT_SQL = """
WITH signal_lifecycle_events AS (
    SELECT
        signal_id,
        lower(pool_identifier) AS pool_key,
        pool_identifier,
        pool_protocol,
        lower(token_address) AS token_key,
        token_address,
        event_kind,
        detection_timestamp,
        pending_tx_hash,
        lower(actor_address) AS actor_key,
        actor_address,
        lower(subject_address) AS subject_key,
        subject_address,
        headline,
        payload
    FROM live_trading.signal_events
    WHERE pool_identifier IS NOT NULL
      AND event_kind IN (
        'trading_enabled',
        'lp_position_approval',
        'liquidity_removal'
      )
),
liquidity_removal_pools AS (
    SELECT
        pool_key,
        min(detection_timestamp) AS first_liquidity_removal_at
    FROM signal_lifecycle_events
    WHERE event_kind = 'liquidity_removal'
    GROUP BY pool_key
),
pool_rollup AS (
    SELECT
        lr.pool_key,
        max(e.pool_identifier) AS pool_identifier,
        max(e.pool_protocol) AS pool_protocol,
        max(e.token_address) AS token_address,
        min(e.detection_timestamp) FILTER (
            WHERE e.event_kind = 'trading_enabled'
        ) AS first_trading_enabled_at,
        min(e.detection_timestamp) FILTER (
            WHERE e.event_kind = 'lp_position_approval'
        ) AS first_lp_approval_at,
        lr.first_liquidity_removal_at,
        count(*) FILTER (
            WHERE e.event_kind = 'trading_enabled'
        ) AS trading_enabled_signal_count,
        count(*) FILTER (
            WHERE e.event_kind = 'lp_position_approval'
        ) AS lp_approval_signal_count,
        count(*) FILTER (
            WHERE e.event_kind = 'liquidity_removal'
        ) AS liquidity_removal_signal_count,
        array_remove(array_agg(DISTINCT e.actor_key) FILTER (
            WHERE e.event_kind = 'trading_enabled'
        ), NULL) AS trading_enabled_actor_keys,
        array_remove(array_agg(DISTINCT e.actor_key) FILTER (
            WHERE e.event_kind = 'lp_position_approval'
        ), NULL) AS lp_approval_actor_keys,
        array_remove(array_agg(DISTINCT e.actor_key) FILTER (
            WHERE e.event_kind = 'liquidity_removal'
        ), NULL) AS liquidity_removal_actor_keys,
        array_remove(array_agg(DISTINCT e.pending_tx_hash) FILTER (
            WHERE e.event_kind = 'trading_enabled'
        ), NULL) AS trading_enabled_tx_hashes,
        array_remove(array_agg(DISTINCT e.pending_tx_hash) FILTER (
            WHERE e.event_kind = 'lp_position_approval'
        ), NULL) AS lp_approval_tx_hashes,
        array_remove(array_agg(DISTINCT e.pending_tx_hash) FILTER (
            WHERE e.event_kind = 'liquidity_removal'
        ), NULL) AS liquidity_removal_tx_hashes,
        jsonb_agg(
            jsonb_build_object(
                'signal_id', e.signal_id,
                'event_kind', e.event_kind,
                'detection_timestamp', e.detection_timestamp,
                'pending_tx_hash', e.pending_tx_hash,
                'actor_address', e.actor_address,
                'subject_address', e.subject_address,
                'headline', e.headline,
                'payload', e.payload
            )
            ORDER BY e.detection_timestamp, e.signal_id
        ) AS evidence_events
    FROM liquidity_removal_pools lr
    JOIN signal_lifecycle_events e
      ON e.pool_key = lr.pool_key
    GROUP BY lr.pool_key, lr.first_liquidity_removal_at
)
SELECT
    pool_key,
    pool_identifier,
    pool_protocol,
    token_address,
    first_trading_enabled_at,
    first_lp_approval_at,
    first_liquidity_removal_at,
    CASE
        WHEN first_trading_enabled_at IS NULL THEN NULL
        ELSE EXTRACT(EPOCH FROM (
            first_liquidity_removal_at - first_trading_enabled_at
        ))::bigint
    END AS seconds_trading_enabled_to_liquidity_removal,
    CASE
        WHEN first_lp_approval_at IS NULL THEN NULL
        ELSE EXTRACT(EPOCH FROM (
            first_liquidity_removal_at - first_lp_approval_at
        ))::bigint
    END AS seconds_lp_approval_to_liquidity_removal,
    trading_enabled_signal_count,
    lp_approval_signal_count,
    liquidity_removal_signal_count,
    trading_enabled_actor_keys,
    lp_approval_actor_keys,
    liquidity_removal_actor_keys,
    trading_enabled_tx_hashes,
    lp_approval_tx_hashes,
    liquidity_removal_tx_hashes,
    first_trading_enabled_at < first_liquidity_removal_at
        AS creator_main_tx_public_mempool,
    first_lp_approval_at < first_liquidity_removal_at
        AS lp_control_tx_public_mempool,
    liquidity_removal_signal_count > 0
        AS removal_tx_public_mempool,
    (
        first_trading_enabled_at < first_liquidity_removal_at
        OR first_lp_approval_at < first_liquidity_removal_at
    ) AS pool_mempool_managed_pre_removal,
    liquidity_removal_signal_count > 0
        AS pool_mempool_exit_signal_available,
    evidence_events
FROM pool_rollup
ORDER BY first_liquidity_removal_at DESC, pool_key;
"""


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Analyze whether liquidity-removal pools had prior public mempool "
            "creator or LP-control management signals."
        )
    )
    parser.add_argument(
        "--database-url",
        default=os.environ.get("DATABASE_URL", DEFAULT_DATABASE_URL),
        help="PostgreSQL URL for the eth database.",
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=Path("risk_atlas/scam_analytics/artifacts/mempool_management"),
        help="Directory for generated CSV and markdown outputs.",
    )
    return parser.parse_args()


def load_rows(database_url: str) -> list[dict[str, Any]]:
    with psycopg2.connect(database_url) as conn:
        with conn.cursor(cursor_factory=psycopg2.extras.RealDictCursor) as cursor:
            cursor.execute(LIQUIDITY_REMOVAL_COHORT_SQL)
            return [dict(row) for row in cursor.fetchall()]


def as_bool(value: Any) -> bool:
    return bool(value)


def as_list(value: Any) -> list[str]:
    if value is None:
        return []
    return list(value)


def seconds_values(rows: list[dict[str, Any]], key: str) -> list[int]:
    values: list[int] = []
    for row in rows:
        value = row.get(key)
        if value is not None and value >= 0:
            values.append(int(value))
    return values


def pct(numerator: int, denominator: int) -> str:
    if denominator == 0:
        return "0.0%"
    return f"{(numerator / denominator) * 100:.1f}%"


def summarize(rows: list[dict[str, Any]]) -> dict[str, Any]:
    total = len(rows)
    creator_public = sum(as_bool(row["creator_main_tx_public_mempool"]) for row in rows)
    lp_public = sum(as_bool(row["lp_control_tx_public_mempool"]) for row in rows)
    strict = sum(as_bool(row["pool_mempool_managed_pre_removal"]) for row in rows)
    exit_signal = sum(as_bool(row["pool_mempool_exit_signal_available"]) for row in rows)
    both_creator_and_lp = sum(
        as_bool(row["creator_main_tx_public_mempool"])
        and as_bool(row["lp_control_tx_public_mempool"])
        for row in rows
    )
    only_exit_signal = sum(
        as_bool(row["pool_mempool_exit_signal_available"])
        and not as_bool(row["pool_mempool_managed_pre_removal"])
        for row in rows
    )
    same_actor = 0
    for row in rows:
        removers = set(as_list(row["liquidity_removal_actor_keys"]))
        managers = set(as_list(row["trading_enabled_actor_keys"])) | set(
            as_list(row["lp_approval_actor_keys"])
        )
        if removers and managers and removers & managers:
            same_actor += 1

    lp_windows = seconds_values(rows, "seconds_lp_approval_to_liquidity_removal")
    trading_windows = seconds_values(rows, "seconds_trading_enabled_to_liquidity_removal")

    return {
        "generated_at": datetime.now(timezone.utc).isoformat(timespec="seconds"),
        "liquidity_removal_pools": total,
        "creator_main_tx_public_mempool": creator_public,
        "lp_control_tx_public_mempool": lp_public,
        "pool_mempool_managed_pre_removal": strict,
        "pool_mempool_exit_signal_available": exit_signal,
        "both_creator_and_lp": both_creator_and_lp,
        "only_exit_signal": only_exit_signal,
        "same_manager_and_remover_actor": same_actor,
        "median_seconds_lp_approval_to_liquidity_removal": int(median(lp_windows))
        if lp_windows
        else None,
        "min_seconds_lp_approval_to_liquidity_removal": min(lp_windows)
        if lp_windows
        else None,
        "max_seconds_lp_approval_to_liquidity_removal": max(lp_windows)
        if lp_windows
        else None,
        "median_seconds_trading_enabled_to_liquidity_removal": int(median(trading_windows))
        if trading_windows
        else None,
        "min_seconds_trading_enabled_to_liquidity_removal": min(trading_windows)
        if trading_windows
        else None,
        "max_seconds_trading_enabled_to_liquidity_removal": max(trading_windows)
        if trading_windows
        else None,
    }


def compact_json(value: Any) -> str:
    if value is None:
        return ""
    return json.dumps(value, default=str, separators=(",", ":"))


def write_csv(rows: list[dict[str, Any]], output_path: Path) -> None:
    fields = [
        "pool_key",
        "pool_identifier",
        "pool_protocol",
        "token_address",
        "first_trading_enabled_at",
        "first_lp_approval_at",
        "first_liquidity_removal_at",
        "seconds_trading_enabled_to_liquidity_removal",
        "seconds_lp_approval_to_liquidity_removal",
        "trading_enabled_signal_count",
        "lp_approval_signal_count",
        "liquidity_removal_signal_count",
        "trading_enabled_actor_keys",
        "lp_approval_actor_keys",
        "liquidity_removal_actor_keys",
        "trading_enabled_tx_hashes",
        "lp_approval_tx_hashes",
        "liquidity_removal_tx_hashes",
        "creator_main_tx_public_mempool",
        "lp_control_tx_public_mempool",
        "removal_tx_public_mempool",
        "pool_mempool_managed_pre_removal",
        "pool_mempool_exit_signal_available",
        "evidence_events",
    ]
    with output_path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=fields)
        writer.writeheader()
        for row in rows:
            csv_row = {}
            for field in fields:
                value = row.get(field)
                if isinstance(value, (list, dict)):
                    csv_row[field] = compact_json(value)
                elif isinstance(value, datetime):
                    csv_row[field] = value.isoformat()
                else:
                    csv_row[field] = value
            writer.writerow(csv_row)


def write_summary(rows: list[dict[str, Any]], summary: dict[str, Any], output_path: Path) -> None:
    total = int(summary["liquidity_removal_pools"])
    recent = rows[:10]
    lines = [
        "# Mempool Management Summary",
        "",
        f"Generated: `{summary['generated_at']}`",
        "",
        "## Counts",
        "",
        f"- Liquidity-removal pools: `{total}`",
        "- Strict mempool-managed pools: "
        f"`{summary['pool_mempool_managed_pre_removal']}` "
        f"({pct(summary['pool_mempool_managed_pre_removal'], total)})",
        "- Creator main tx public before removal: "
        f"`{summary['creator_main_tx_public_mempool']}` "
        f"({pct(summary['creator_main_tx_public_mempool'], total)})",
        "- LP-control tx public before removal: "
        f"`{summary['lp_control_tx_public_mempool']}` "
        f"({pct(summary['lp_control_tx_public_mempool'], total)})",
        "- Both creator and LP-control public before removal: "
        f"`{summary['both_creator_and_lp']}` "
        f"({pct(summary['both_creator_and_lp'], total)})",
        "- Same manager/remover actor overlap: "
        f"`{summary['same_manager_and_remover_actor']}` "
        f"({pct(summary['same_manager_and_remover_actor'], total)})",
        "- Exit signal available from liquidity-removal mempool event: "
        f"`{summary['pool_mempool_exit_signal_available']}` "
        f"({pct(summary['pool_mempool_exit_signal_available'], total)})",
        "- Only removal signal, no prior public management signal: "
        f"`{summary['only_exit_signal']}` "
        f"({pct(summary['only_exit_signal'], total)})",
        "",
        "## Timing",
        "",
        "- Median seconds from LP approval to liquidity removal: "
        f"`{summary['median_seconds_lp_approval_to_liquidity_removal']}`",
        "- Min/max seconds from LP approval to liquidity removal: "
        f"`{summary['min_seconds_lp_approval_to_liquidity_removal']}` / "
        f"`{summary['max_seconds_lp_approval_to_liquidity_removal']}`",
        "- Median seconds from trading enabled to liquidity removal: "
        f"`{summary['median_seconds_trading_enabled_to_liquidity_removal']}`",
        "- Min/max seconds from trading enabled to liquidity removal: "
        f"`{summary['min_seconds_trading_enabled_to_liquidity_removal']}` / "
        f"`{summary['max_seconds_trading_enabled_to_liquidity_removal']}`",
        "",
        "## Latest Pools",
        "",
        "| First removal | Pool | Strict flag | LP window seconds | Creator window seconds |",
        "| --- | --- | --- | ---: | ---: |",
    ]
    for row in recent:
        lines.append(
            "| "
            f"{row['first_liquidity_removal_at']} | "
            f"`{row['pool_key']}` | "
            f"`{row['pool_mempool_managed_pre_removal']}` | "
            f"{row['seconds_lp_approval_to_liquidity_removal']} | "
            f"{row['seconds_trading_enabled_to_liquidity_removal']} |"
        )
    lines.extend(
        [
            "",
            "## Interpretation",
            "",
            "Use `pool_mempool_managed_pre_removal` as the persistent behavior flag. "
            "`pool_mempool_exit_signal_available` is an event-level fact: it means "
            "the actual removal tx was seen in the mempool, not that the pool was "
            "known to be public-mempool managed before that event.",
            "",
        ]
    )
    output_path.write_text("\n".join(lines), encoding="utf-8")


def main() -> None:
    args = parse_args()
    rows = load_rows(args.database_url)
    summary = summarize(rows)

    args.output_dir.mkdir(parents=True, exist_ok=True)
    csv_path = args.output_dir / "pool_mempool_management.csv"
    summary_path = args.output_dir / "latest_summary.md"
    write_csv(rows, csv_path)
    write_summary(rows, summary, summary_path)

    print(f"rows: {len(rows)}")
    print(f"strict_mempool_managed: {summary['pool_mempool_managed_pre_removal']}")
    print(f"exit_signal_available: {summary['pool_mempool_exit_signal_available']}")
    print(f"wrote: {csv_path}")
    print(f"wrote: {summary_path}")


if __name__ == "__main__":
    main()
