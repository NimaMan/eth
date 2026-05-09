#!/usr/bin/env python3
"""Summarize eth_token_server token pipeline profile logs.

Input is the JSON rolling log written by eth_token_server:
logs/eth_token_server/token_pipeline_profile.log.YYYY-MM-DD
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
    "block_apply_wall_ms",
    "unaccounted_ms",
    "take_processor_ms",
    "token_apply_ms",
    "state_update_ms",
    "disk_cache_read_ms",
    "total_ms",
    "sort_ms",
    "token_metadata_ms",
    "router_ms",
    "index_refresh_ms",
    "network_update_ms",
    "finalize_ms",
    "applier_candidate_ms",
    "applier_token_state_ms",
    "applier_pool_discovery_ms",
    "applier_pool_update_ms",
    "applier_simulation_v2_ms",
    "applier_simulation_v3_ms",
    "applier_simulation_v4_ms",
    "applier_report_ms",
    "transaction_count",
    "processed_transaction_count",
    "applicable_transactions",
    "registry_tokens",
    "indexed_tokens",
    "indexed_pools",
    "applier_candidate_tokens",
    "applier_visited_tokens",
    "applier_update_reports",
    "simulated_v2_pools",
    "simulated_v3_pools",
    "simulated_v4_pools",
    "historical_session_creates",
    "historical_branches",
    "historical_session_create_ms",
    "historical_branch_ms",
    "live_session_creates",
    "live_branches",
    "live_session_create_ms",
    "live_branch_ms",
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
        if args.run_id and fields.get("run_id") != args.run_id:
            continue
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
    print_summary(rows, "block_apply_wall_ms")
    print_summary(rows, "take_processor_ms")
    print_summary(rows, "token_apply_ms")
    print_summary(rows, "state_update_ms")
    print_summary(rows, "router_ms")
    print_summary(rows, "applier_pool_discovery_ms")
    print_summary(rows, "applier_simulation_v2_ms")
    print_summary(rows, "historical_session_creates")
    print_summary(rows, "historical_branches")

    print()
    print_top(rows, args.top, "block_apply_wall_ms")
    print()
    print_top(rows, args.top, "token_apply_ms")
    print()
    print_top(rows, args.top, "take_processor_ms")

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
    block[f"{prefix}_session_create_ms"] = int(block.get(f"{prefix}_session_create_ms", 0)) + int(
        coerce(fields.get("session_create_ms", 0)) or 0
    )
    block[f"{prefix}_branch_ms"] = int(block.get(f"{prefix}_branch_ms", 0)) + int(
        coerce(fields.get("branch_ms", 0)) or 0
    )


def normalize_block(block_number: int, data: dict[str, object]) -> dict[str, object]:
    row = {"block_number": block_number}
    row.update(data)
    for key in NUMERIC_FIELDS:
        row.setdefault(key, 0)
    return row


def print_summary(rows: list[dict[str, object]], key: str) -> None:
    values = [float(row.get(key) or 0) for row in rows if row.get(key) is not None]
    if not values:
        return
    values.sort()
    print(
        f"{key}: sum={sum(values):.0f} avg={statistics.mean(values):.2f} "
        f"p50={percentile(values, 0.50):.0f} p90={percentile(values, 0.90):.0f} "
        f"p95={percentile(values, 0.95):.0f} p99={percentile(values, 0.99):.0f} "
        f"max={max(values):.0f}"
    )


def print_top(rows: list[dict[str, object]], count: int, key: str) -> None:
    print(f"top_{count}_by_{key}")
    columns = [
        "block_number",
        key,
        "take_processor_ms",
        "token_apply_ms",
        "state_update_ms",
        "router_ms",
        "applier_pool_discovery_ms",
        "applier_simulation_v2_ms",
        "historical_session_creates",
        "historical_branches",
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


if __name__ == "__main__":
    sys.exit(main())
