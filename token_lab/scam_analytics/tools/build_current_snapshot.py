#!/usr/bin/env python3
"""Build scam-label and direct-LP feature exports from one range-view snapshot."""

from __future__ import annotations

import argparse
import csv
from pathlib import Path

import build_100_review_batch as labels
import build_direct_lp_features as direct_lp


DEFAULT_RUN_ID = "run-2"
DEFAULT_API_BASE = "http://127.0.0.1:8765/eth/tokens/api"


def main() -> None:
    args = parse_args()
    api = direct_lp.Api(args.api_base, args.detail_retries, args.detail_retry_delay_secs)
    progress = api.get(f"/runs/{args.run_id}/progress")
    pools = direct_lp.get_nonempty_run_view(
        api,
        args.run_id,
        "pools",
        "pools",
        "indexed_pools",
        args.view_retries,
        args.view_retry_delay_secs,
    )
    token_summaries = direct_lp.get_nonempty_run_view(
        api,
        args.run_id,
        "tokens",
        "tokens",
        "indexed_tokens",
        args.view_retries,
        args.view_retry_delay_secs,
    )
    token_by_address = {
        direct_lp.lower(token.get("contract_address")): token
        for token in token_summaries
        if token.get("contract_address")
    }

    label_rows = labels.build_rows(args.run_id, pools, args.limit, args.review_prefix)
    direct_rows = direct_lp.build_rows(args.run_id, api, pools, token_by_address)
    window_rows = direct_lp.build_window_rows(args.run_id, api, pools, token_by_address, args.window_offsets)
    control_window_rows = direct_lp.build_control_window_rows(
        args.run_id,
        api,
        pools,
        token_by_address,
        args.window_offsets,
        progress,
    )
    training_window_rows = window_rows + control_window_rows

    write_csv(args.label_output, labels.REVIEW_COLUMNS, label_rows)
    args.label_summary.parent.mkdir(parents=True, exist_ok=True)
    args.label_summary.write_text(
        labels.render_summary(args.run_id, progress, label_rows, len(pools)),
        encoding="utf-8",
    )

    write_csv(args.direct_output, direct_lp.FEATURE_COLUMNS, direct_rows)
    write_csv(args.window_output, direct_lp.WINDOW_COLUMNS, window_rows)
    write_csv(args.training_output, direct_lp.WINDOW_COLUMNS, training_window_rows)
    args.direct_summary.parent.mkdir(parents=True, exist_ok=True)
    args.direct_summary.write_text(
        direct_lp.render_summary(args.run_id, progress, direct_rows, window_rows, control_window_rows),
        encoding="utf-8",
    )

    print(f"snapshot pools: {len(pools)}")
    print(f"wrote {len(label_rows)} review rows to {args.label_output}")
    print(f"wrote {len(direct_rows)} direct-LP feature rows to {args.direct_output}")
    print(f"wrote {len(window_rows)} direct-LP window rows to {args.window_output}")
    print(f"wrote {len(training_window_rows)} direct-LP training window rows to {args.training_output}")
    print(f"wrote summaries to {args.label_summary} and {args.direct_summary}")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--api-base", default=DEFAULT_API_BASE)
    parser.add_argument("--run-id", default=DEFAULT_RUN_ID)
    parser.add_argument("--limit", type=int, default=0, help="maximum label rows; 0 exports all")
    parser.add_argument("--review-prefix", default="scamall")
    parser.add_argument(
        "--label-output",
        type=Path,
        default=Path("token_lab/scam_analytics/labels/scam_pools_all_review.csv"),
    )
    parser.add_argument(
        "--label-summary",
        type=Path,
        default=Path("token_lab/scam_analytics/artifacts/reports/scam_pools_all_summary.md"),
    )
    parser.add_argument(
        "--direct-output",
        type=Path,
        default=Path("token_lab/scam_analytics/features/direct_lp_liquidity_removal_features.csv"),
    )
    parser.add_argument(
        "--direct-summary",
        type=Path,
        default=Path(
            "token_lab/scam_analytics/artifacts/reports/direct_lp_liquidity_removal_features_summary.md"
        ),
    )
    parser.add_argument(
        "--window-output",
        type=Path,
        default=Path("token_lab/scam_analytics/features/direct_lp_liquidity_removal_window_features.csv"),
    )
    parser.add_argument(
        "--training-output",
        type=Path,
        default=Path("token_lab/scam_analytics/features/direct_lp_liquidity_removal_training_windows.csv"),
    )
    parser.add_argument("--window-offsets", default="1,10,50,100,500")
    parser.add_argument("--view-retries", type=int, default=30)
    parser.add_argument("--view-retry-delay-secs", type=float, default=2.0)
    parser.add_argument("--detail-retries", type=int, default=0)
    parser.add_argument("--detail-retry-delay-secs", type=float, default=0.5)
    args = parser.parse_args()
    args.window_offsets = direct_lp.parse_window_offsets(args.window_offsets)
    return args


def write_csv(path: Path, columns: list[str], rows: list[dict[str, str]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=columns)
        writer.writeheader()
        writer.writerows(rows)


if __name__ == "__main__":
    main()
