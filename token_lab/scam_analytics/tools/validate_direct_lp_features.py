#!/usr/bin/env python3
"""Validate direct-LP scam feature exports for label alignment and leakage."""

from __future__ import annotations

import argparse
import csv
from collections import Counter, defaultdict
from pathlib import Path


DEFAULT_LABELS = Path("token_lab/scam_analytics/labels/scam_pools_all_review.csv")
DEFAULT_FEATURES = Path("token_lab/scam_analytics/features/direct_lp_liquidity_removal_features.csv")
DEFAULT_WINDOWS = Path("token_lab/scam_analytics/features/direct_lp_liquidity_removal_window_features.csv")
DEFAULT_TRAINING = Path("token_lab/scam_analytics/features/direct_lp_liquidity_removal_training_windows.csv")


def main() -> None:
    args = parse_args()
    labels = read_csv(args.labels)
    features = read_csv(args.features)
    windows = read_csv(args.windows)
    training = read_csv(args.training)

    errors: list[str] = []
    missing_required_cols = [
        column
        for column in [
            "activity_density_to_as_of",
            "tx_per_active_block_to_as_of",
            "net_buy_volume_eth_to_as_of",
            "tx_share_last_10_to_total_as_of",
            "active_density_last_10_as_of",
        ]
        if training and column not in training[0]
    ]
    if missing_required_cols:
        errors.append(f"missing training columns: {', '.join(missing_required_cols)}")

    feature_keys = {(row["token"].lower(), row["pool"].lower()) for row in features}
    direct_label_keys = {
        (row["token"].lower(), row["pool"].lower())
        for row in labels
        if row.get("mechanism") == "direct_lp_liquidity_removal"
    }
    missing_features = direct_label_keys - feature_keys
    extra_features = feature_keys - direct_label_keys
    if missing_features:
        errors.append(f"{len(missing_features)} direct-LP labels missing feature rows")
    if extra_features:
        errors.append(f"{len(extra_features)} direct-LP feature rows missing from label snapshot")

    training_errors = validate_training_rows(training)
    feature_errors = validate_feature_rows(features)
    errors.extend(training_errors)
    errors.extend(feature_errors)

    print_summary(labels, features, windows, training)
    if errors:
        for error in errors:
            print(f"ERROR: {error}")
        raise SystemExit(1)
    print("validation: ok")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--labels", type=Path, default=DEFAULT_LABELS)
    parser.add_argument("--features", type=Path, default=DEFAULT_FEATURES)
    parser.add_argument("--windows", type=Path, default=DEFAULT_WINDOWS)
    parser.add_argument("--training", type=Path, default=DEFAULT_TRAINING)
    return parser.parse_args()


def read_csv(path: Path) -> list[dict[str, str]]:
    with path.open(newline="", encoding="utf-8") as handle:
        return list(csv.DictReader(handle))


def validate_training_rows(rows: list[dict[str, str]]) -> list[str]:
    bad_positive_asof = []
    bad_control_asof = []
    bad_approval_after_asof = []
    bad_activity_after_asof = []
    negative_since = defaultdict(list)
    bad_ratios = defaultdict(list)

    for row in rows:
        asof = as_int(row.get("as_of_block"))
        label = as_int(row.get("label_block"))
        observed = as_int(row.get("observed_until_block"))
        if row.get("row_kind") == "positive" and label is not None and asof is not None and asof >= label:
            bad_positive_asof.append(row["row_id"])
        if row.get("row_kind") == "control" and observed is not None and asof is not None and asof > observed:
            bad_control_asof.append(row["row_id"])
        approval = as_int(row.get("lp_last_approval_block_as_of"))
        if approval is not None and asof is not None and approval > asof:
            bad_approval_after_asof.append(row["row_id"])
        first_activity = as_int(row.get("first_activity_block_to_as_of"))
        last_activity = as_int(row.get("last_activity_block_to_as_of"))
        if last_activity is not None and asof is not None and last_activity > asof:
            bad_activity_after_asof.append(row["row_id"])
        if first_activity is not None and last_activity is not None and first_activity > last_activity:
            bad_activity_after_asof.append(row["row_id"])

        for column in (
            "blocks_since_trading_enabled_as_of",
            "blocks_since_pool_created_as_of",
            "blocks_since_token_created_as_of",
            "blocks_since_last_activity_as_of",
        ):
            value = as_int(row.get(column))
            if value is not None and value < 0:
                negative_since[column].append(row["row_id"])

        for column in (
            "activity_density_to_as_of",
            "active_density_last_10_as_of",
            "active_density_last_50_as_of",
            "active_density_last_100_as_of",
            "tx_share_last_10_to_total_as_of",
            "tx_share_last_50_to_total_as_of",
            "tx_share_last_100_to_total_as_of",
        ):
            value = as_float(row.get(column))
            if value is not None and not (0 <= value <= 1.000000001):
                bad_ratios[column].append(row["row_id"])

        for column in (
            "current_owner_is_zero_address",
            "current_owner_is_creator",
            "lp_last_approval_is_router_as_of",
            "lp_last_approval_owner_is_creator_as_of",
            "lp_last_approval_owner_is_current_owner_as_of",
        ):
            value = row.get(column)
            if value not in ("", "true", "false"):
                bad_ratios[f"invalid_boolean:{column}"].append(row["row_id"])

    errors = []
    append_count(errors, "positive rows at/after label block", bad_positive_asof)
    append_count(errors, "control rows after observed_until_block", bad_control_asof)
    append_count(errors, "approval blocks after as_of_block", bad_approval_after_asof)
    append_count(errors, "activity blocks after as_of_block", bad_activity_after_asof)
    for column, row_ids in negative_since.items():
        append_count(errors, f"negative {column}", row_ids)
    for column, row_ids in bad_ratios.items():
        append_count(errors, f"ratio outside 0..1 for {column}", row_ids)
    return errors


def validate_feature_rows(rows: list[dict[str, str]]) -> list[str]:
    negative_leads = defaultdict(list)
    bad_activity_after_asof = []
    bad_ratios = defaultdict(list)
    for row in rows:
        asof = as_int(row.get("as_of_block"))
        last_activity = as_int(row.get("pre_label_last_activity_block"))
        if last_activity is not None and asof is not None and last_activity > asof:
            bad_activity_after_asof.append(row["row_id"])
        for column in (
            "blocks_lp_first_approval_to_removal",
            "blocks_lp_first_pre_removal_approval_to_removal",
            "blocks_lp_last_pre_removal_approval_to_removal",
            "blocks_lp_last_approval_to_removal",
            "blocks_pre_label_last_activity_to_label",
        ):
            value = as_int(row.get(column))
            if value is not None and value < 0:
                negative_leads[column].append(row["row_id"])
        for column in (
            "pre_label_active_density",
            "pre_label_active_density_last_10",
            "pre_label_active_density_last_100",
            "pre_label_tx_share_last_10",
            "pre_label_tx_share_last_100",
        ):
            value = as_float(row.get(column))
            if value is not None and not (0 <= value <= 1.000000001):
                bad_ratios[column].append(row["row_id"])
        for column in (
            "current_owner_is_zero_address",
            "current_owner_is_creator",
            "lp_first_pre_removal_approval_is_router",
            "lp_last_pre_removal_approval_is_router",
            "lp_last_pre_removal_approval_owner_is_creator",
            "lp_last_pre_removal_approval_owner_is_current_owner",
        ):
            value = row.get(column)
            if value not in ("", "true", "false"):
                bad_ratios[f"invalid_boolean:{column}"].append(row["row_id"])

    errors = []
    append_count(errors, "feature activity blocks after as_of_block", bad_activity_after_asof)
    for column, row_ids in negative_leads.items():
        append_count(errors, f"negative {column}", row_ids)
    for column, row_ids in bad_ratios.items():
        append_count(errors, f"ratio outside 0..1 for {column}", row_ids)
    return errors


def print_summary(
    labels: list[dict[str, str]],
    features: list[dict[str, str]],
    windows: list[dict[str, str]],
    training: list[dict[str, str]],
) -> None:
    print("labels_rows", len(labels))
    print("labels_mechanisms", dict(Counter(row.get("mechanism") for row in labels)))
    print("feature_rows", len(features))
    print("window_rows", len(windows))
    print("training_rows", len(training))
    print("training_row_kind", dict(Counter(row.get("row_kind") for row in training)))
    print("training_target", dict(Counter(row.get("target_removal_within_horizon") for row in training)))
    print(
        "any_pre_removal_approval",
        dict(Counter(row.get("lp_any_approval_pre_removal") for row in features)),
    )
    print(
        "activity_density_nonempty",
        sum(1 for row in training if row.get("activity_density_to_as_of")),
    )


def append_count(errors: list[str], label: str, row_ids: list[str]) -> None:
    if row_ids:
        errors.append(f"{len(row_ids)} {label}")


def as_int(value: str | None) -> int | None:
    if value in ("", None):
        return None
    return int(float(value))


def as_float(value: str | None) -> float | None:
    if value in ("", None):
        return None
    return float(value)


if __name__ == "__main__":
    main()
