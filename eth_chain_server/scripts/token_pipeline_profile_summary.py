#!/usr/bin/env python3
"""Summarize eth_chain_server token pipeline profile logs.

Input is the JSON rolling log written by eth_chain_server:
logs/eth_chain_server/token_pipeline_profile.log.YYYY-MM-DD
"""

from __future__ import annotations

import argparse
import csv
import json
import statistics
import sys
from collections import defaultdict
from pathlib import Path


NUMERIC_FIELDS = {
    "block_apply_wall_us",
    "block_apply_wall_ms",
    "unaccounted_us",
    "unaccounted_ms",
    "take_processor_us",
    "take_processor_ms",
    "token_apply_us",
    "token_apply_ms",
    "state_update_us",
    "state_update_ms",
    "disk_cache_read_us",
    "disk_cache_read_ms",
    "total_us",
    "total_ms",
    "sort_us",
    "sort_ms",
    "token_metadata_us",
    "token_metadata_ms",
    "router_us",
    "router_ms",
    "index_refresh_us",
    "index_refresh_ms",
    "network_update_us",
    "network_update_ms",
    "finalize_us",
    "finalize_ms",
    "applier_candidate_us",
    "applier_candidate_ms",
    "applier_token_state_us",
    "applier_token_state_ms",
    "applier_pool_discovery_us",
    "applier_pool_discovery_ms",
    "applier_pool_update_us",
    "applier_pool_update_ms",
    "applier_simulation_v2_us",
    "applier_simulation_v2_ms",
    "applier_simulation_v3_us",
    "applier_simulation_v3_ms",
    "applier_simulation_v4_us",
    "applier_simulation_v4_ms",
    "applier_report_us",
    "applier_report_ms",
    "transaction_count",
    "processed_transaction_count",
    "applicable_transactions",
    "skipped_transactions",
    "processing_error_transactions",
    "created_token_count",
    "updated_token_count",
    "token_update_count",
    "transaction_error_count",
    "registry_tokens",
    "indexed_tokens",
    "indexed_pools",
    "processed_blocks",
    "applier_candidate_tx_count",
    "applier_candidate_tokens",
    "applier_visited_tokens",
    "applier_token_state_updates",
    "applier_token_control_replays",
    "applier_update_reports",
    "simulation_v2_candidate_pools",
    "simulation_v3_candidate_pools",
    "simulation_v4_candidate_pools",
    "simulation_v2_current_block_pools",
    "simulation_v3_current_block_pools",
    "simulation_v4_current_block_pools",
    "simulated_v2_pools",
    "simulated_v3_pools",
    "simulated_v4_pools",
    "historical_session_creates",
    "historical_branches",
    "historical_session_create_us",
    "historical_session_create_ms",
    "historical_branch_us",
    "historical_branch_ms",
    "live_session_creates",
    "live_branches",
    "live_session_create_us",
    "live_session_create_ms",
    "live_branch_us",
    "live_branch_ms",
    "simulation_total_us",
    "simulation_total_ms",
    "simulation_candidate_pools",
    "simulation_current_block_pools",
    "simulated_pools",
    "simulation_skipped_pools",
    "router_measured_us",
    "router_non_sim_measured_us",
    "router_residual_us",
}

TARGET_RANGE = "token_range_apply_profile"
TARGET_BLOCK = "token_block_processor_profile"
TARGET_SESSION = "token_sim_session_profile"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("log", type=Path)
    parser.add_argument("--run-id", help="only include events for this range run id")
    parser.add_argument("--csv", type=Path, help="optional per-block CSV output path")
    parser.add_argument("--top", type=int, default=20)
    args = parser.parse_args()

    blocks: dict[int, dict[str, object]] = defaultdict(dict)
    line_count = 0
    for line in args.log.open():
        line = line.strip()
        if not line:
            continue
        try:
            row = json.loads(line)
        except json.JSONDecodeError:
            continue
        line_count += 1
        target = row.get("target")
        fields = row.get("fields") or {}
        row_run_id = event_run_id(row, fields)
        if args.run_id and row_run_id != args.run_id:
            continue
        if row_run_id and not fields.get("run_id"):
            fields = dict(fields)
            fields["run_id"] = row_run_id
        block_number = fields.get("block_number")
        if block_number is None:
            continue
        block = blocks[int(block_number)]

        if target == TARGET_RANGE:
            copy_fields(block, fields)
        elif target == TARGET_BLOCK:
            copy_fields(block, fields)
        elif target == TARGET_SESSION:
            add_session(block, fields)

    rows = [normalize_block(block_number, data) for block_number, data in blocks.items()]
    rows.sort(key=lambda row: int(row["block_number"]))

    print(f"profile_lines={line_count} blocks={len(rows)}")
    print_summary(rows, "block_apply_wall_us", scale=1_000, unit="ms")
    print_summary(rows, "take_processor_us", scale=1_000, unit="ms")
    print_summary(rows, "token_apply_us", scale=1_000, unit="ms")
    print_summary(rows, "state_update_us", scale=1_000, unit="ms")
    print_summary(rows, "router_us", scale=1_000, unit="ms")
    print_summary(rows, "router_residual_us", scale=1_000, unit="ms")
    print_summary(rows, "applier_candidate_us", scale=1_000, unit="ms")
    print_summary(rows, "applier_pool_discovery_us", scale=1_000, unit="ms")
    print_summary(rows, "simulation_total_us", scale=1_000, unit="ms")
    print_summary(rows, "historical_session_create_us", scale=1_000, unit="ms")
    print_summary(rows, "historical_branch_us", scale=1_000, unit="ms")
    print_summary(rows, "historical_session_creates")
    print_summary(rows, "historical_branches")
    print_summary(rows, "simulation_candidate_pools")
    print_summary(rows, "simulated_pools")

    print()
    print_top(rows, args.top, "block_apply_wall_us")
    print()
    print_top(rows, args.top, "token_apply_us")
    print()
    print_top(rows, args.top, "router_residual_us")

    if args.csv:
        write_csv(args.csv, rows)
        print(f"wrote_csv={args.csv}")
    return 0


def copy_fields(block: dict[str, object], fields: dict[str, object]) -> None:
    for key, value in fields.items():
        if key == "message":
            continue
        block[key] = coerce(value)


def add_session(block: dict[str, object], fields: dict[str, object]) -> None:
    mode = str(fields.get("mode") or "unknown")
    prefix = "live" if mode == "live" else "historical"
    block[f"{prefix}_branches"] = int(block.get(f"{prefix}_branches", 0)) + 1
    if truthy(fields.get("session_created")):
        block[f"{prefix}_session_creates"] = int(block.get(f"{prefix}_session_creates", 0)) + 1
    session_create_us = int(
        coerce(
            fields.get(
                "session_create_us",
                int(coerce(fields.get("session_create_ms", 0)) or 0) * 1_000,
            )
        )
        or 0
    )
    branch_us = int(
        coerce(
            fields.get("branch_us", int(coerce(fields.get("branch_ms", 0)) or 0) * 1_000)
        )
        or 0
    )
    block[f"{prefix}_session_create_us"] = (
        int(block.get(f"{prefix}_session_create_us", 0)) + session_create_us
    )
    block[f"{prefix}_session_create_ms"] = block[f"{prefix}_session_create_us"] // 1_000
    block[f"{prefix}_branch_us"] = int(block.get(f"{prefix}_branch_us", 0)) + branch_us
    block[f"{prefix}_branch_ms"] = block[f"{prefix}_branch_us"] // 1_000


def normalize_block(block_number: int, data: dict[str, object]) -> dict[str, object]:
    row = {"block_number": block_number}
    row.update(data)
    derive_microsecond_fields(row)
    derive_totals(row)
    for key in NUMERIC_FIELDS:
        row.setdefault(key, 0)
    return row


def derive_microsecond_fields(row: dict[str, object]) -> None:
    for key in list(NUMERIC_FIELDS):
        if not key.endswith("_ms"):
            continue
        us_key = f"{key[:-3]}_us"
        if us_key in NUMERIC_FIELDS and not row.get(us_key) and row.get(key):
            row[us_key] = int(float(row[key] or 0)) * 1_000
        if us_key in row and not row.get(key):
            row[key] = int(float(row[us_key] or 0)) // 1_000


def derive_totals(row: dict[str, object]) -> None:
    simulation_total_us = sum(
        int(row.get(key) or 0)
        for key in (
            "applier_simulation_v2_us",
            "applier_simulation_v3_us",
            "applier_simulation_v4_us",
        )
    )
    row["simulation_total_us"] = simulation_total_us
    row["simulation_total_ms"] = simulation_total_us // 1_000

    simulation_candidate_pools = sum(
        int(row.get(key) or 0)
        for key in (
            "simulation_v2_candidate_pools",
            "simulation_v3_candidate_pools",
            "simulation_v4_candidate_pools",
        )
    )
    row["simulation_candidate_pools"] = simulation_candidate_pools
    row["simulation_current_block_pools"] = sum(
        int(row.get(key) or 0)
        for key in (
            "simulation_v2_current_block_pools",
            "simulation_v3_current_block_pools",
            "simulation_v4_current_block_pools",
        )
    )
    simulated_pools = sum(
        int(row.get(key) or 0)
        for key in ("simulated_v2_pools", "simulated_v3_pools", "simulated_v4_pools")
    )
    row["simulated_pools"] = simulated_pools
    row["simulation_skipped_pools"] = max(0, simulation_candidate_pools - simulated_pools)

    router_non_sim_measured_us = sum(
        int(row.get(key) or 0)
        for key in (
            "applier_candidate_us",
            "applier_token_state_us",
            "applier_pool_discovery_us",
            "applier_pool_update_us",
            "applier_report_us",
        )
    )
    router_measured_us = router_non_sim_measured_us
    row["router_measured_us"] = router_measured_us
    row["router_non_sim_measured_us"] = router_non_sim_measured_us
    row["router_residual_us"] = int(row.get("router_us") or 0) - router_measured_us


def print_summary(
    rows: list[dict[str, object]],
    key: str,
    *,
    scale: float = 1,
    unit: str = "",
) -> None:
    values = [float(row.get(key) or 0) for row in rows if row.get(key) is not None]
    if not values:
        return
    values.sort()
    display_key = f"{key}_as_{unit}" if unit else key
    print(
        f"{display_key}: sum={sum(values) / scale:.0f} avg={statistics.mean(values) / scale:.2f} "
        f"p50={percentile(values, 0.50) / scale:.0f} p90={percentile(values, 0.90) / scale:.0f} "
        f"p95={percentile(values, 0.95) / scale:.0f} p99={percentile(values, 0.99) / scale:.0f} "
        f"max={max(values) / scale:.0f}"
    )


def print_top(rows: list[dict[str, object]], count: int, key: str) -> None:
    print(f"top_{count}_by_{key}")
    columns = [
        "block_number",
        key,
        "take_processor_us",
        "token_apply_us",
        "state_update_us",
        "router_us",
        "simulation_total_us",
        "router_residual_us",
        "applier_candidate_us",
        "applier_pool_discovery_us",
        "historical_branch_us",
        "historical_session_creates",
        "historical_branches",
        "simulation_candidate_pools",
        "simulated_pools",
        "registry_tokens",
        "indexed_pools",
    ]
    print(",".join(columns))
    for row in sorted(rows, key=lambda item: float(item.get(key) or 0), reverse=True)[:count]:
        print(",".join(str(row.get(column, "")) for column in columns))


def write_csv(path: Path, rows: list[dict[str, object]]) -> None:
    columns = ["block_number"] + sorted(NUMERIC_FIELDS)
    with path.open("w", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=columns, extrasaction="ignore")
        writer.writeheader()
        writer.writerows(rows)


def percentile(values: list[float], q: float) -> float:
    if not values:
        return 0
    index = int(round((len(values) - 1) * q))
    return values[index]


def coerce(value: object) -> object:
    if isinstance(value, (int, float, bool)) or value is None:
        return value
    text = str(value)
    if text.isdigit():
        return int(text)
    try:
        return float(text)
    except ValueError:
        return text


def truthy(value: object) -> bool:
    if isinstance(value, bool):
        return value
    return str(value).lower() in {"1", "true", "yes"}


def event_run_id(row: dict[str, object], fields: dict[str, object]) -> str | None:
    run_id = fields.get("run_id")
    if run_id:
        return str(run_id)

    span = row.get("span")
    if isinstance(span, dict):
        span_fields = span.get("fields") or {}
        if span_fields.get("run_id"):
            return str(span_fields["run_id"])

    spans = row.get("spans")
    if isinstance(spans, list):
        for span_item in reversed(spans):
            if not isinstance(span_item, dict):
                continue
            span_fields = span_item.get("fields") or {}
            if span_fields.get("run_id"):
                return str(span_fields["run_id"])
    return None


if __name__ == "__main__":
    sys.exit(main())
