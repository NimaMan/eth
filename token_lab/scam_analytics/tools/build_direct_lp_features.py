#!/usr/bin/env python3
"""Build a direct-LP-removal feature table from token-server range state."""

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

from build_100_review_batch import (
    ETH_SECONDS_PER_BLOCK,
    confidence_for_pool,
    label_block_for_pool,
    number,
    text_int,
    time_bucket,
    trading_enabled,
)


DEFAULT_API_BASE = "http://127.0.0.1:8765/eth/tokens/api"
DEFAULT_RUN_ID = "run-2"
ZERO_ADDRESS = "0x0000000000000000000000000000000000000000"

FEATURE_COLUMNS = [
    "row_id",
    "chain",
    "source_run_id",
    "token",
    "pool",
    "protocol",
    "symbol",
    "target_mechanism",
    "confidence",
    "as_of_block",
    "label_block",
    "trading_enabled_block",
    "blocks_from_trading_enabled_to_removal",
    "minutes_from_trading_enabled_to_removal",
    "time_to_scam_bucket",
    "pool_created_block",
    "token_created_block",
    "blocks_token_created_to_removal",
    "creator_address",
    "current_owner",
    "ownership_renounced_current",
    "current_owner_is_zero_address",
    "current_owner_is_creator",
    "token_total_supply_scaled",
    "pool_token_supply_percent_at_label",
    "price_ratio_to_initial_at_label",
    "liquidity_eth_at_label",
    "liquidity_level_at_label",
    "max_liquidity_eth_before_removal",
    "denom_reserve_ratio_to_max_at_removal",
    "denom_reserve_at_removal",
    "token_reserve_at_removal",
    "buy_tax_at_label",
    "sell_tax_at_label",
    "tax_bucket_at_label",
    "can_buy_at_label",
    "can_sell_at_label",
    "has_observed_buy",
    "has_observed_sell",
    "lp_supply_known",
    "lp_supply_status",
    "lp_total_supply_current",
    "lp_holder_count_current",
    "lp_transfer_count_current",
    "lp_approval_count_current",
    "lp_holders_with_approvals_current",
    "lp_approved_percentage_current",
    "lp_total_approved_to_routers_current",
    "lp_first_approval_block",
    "lp_last_approval_block",
    "lp_any_approval_pre_removal",
    "lp_approval_count_pre_removal",
    "lp_approval_count_same_block",
    "lp_first_pre_removal_approval_block",
    "lp_last_pre_removal_approval_block",
    "lp_first_pre_removal_approval_owner",
    "lp_first_pre_removal_approval_spender",
    "lp_first_pre_removal_approval_is_router",
    "lp_last_pre_removal_approval_owner",
    "lp_last_pre_removal_approval_spender",
    "lp_last_pre_removal_approval_is_router",
    "lp_last_pre_removal_approval_owner_is_creator",
    "lp_last_pre_removal_approval_owner_is_current_owner",
    "blocks_lp_first_approval_to_removal",
    "blocks_lp_first_pre_removal_approval_to_removal",
    "blocks_lp_last_pre_removal_approval_to_removal",
    "blocks_trading_enabled_to_lp_first_approval",
    "blocks_trading_enabled_to_lp_first_pre_removal_approval",
    "blocks_trading_enabled_to_lp_last_pre_removal_approval",
    "blocks_pool_created_to_lp_first_approval",
    "blocks_pool_created_to_lp_first_pre_removal_approval",
    "blocks_pool_created_to_lp_last_pre_removal_approval",
    "lp_last_approval_pre_removal",
    "blocks_lp_last_approval_to_removal",
    "lp_last_approval_owner",
    "lp_last_approval_spender",
    "lp_last_approval_is_router",
    "lp_same_tx_approval_and_removal",
    "top_lp_holder_share_current",
    "top_liquidity_position_share_current",
    "pre_label_activity_blocks",
    "pre_label_tx_count",
    "pre_label_token_transfer_count",
    "pre_label_denom_transfer_count",
    "pre_label_buy_volume_eth",
    "pre_label_sell_volume_eth",
    "pre_label_bribe_eth",
    "pre_label_first_activity_block",
    "pre_label_last_activity_block",
    "blocks_pre_label_last_activity_to_label",
    "pre_label_active_density",
    "pre_label_tx_per_active_block",
    "pre_label_net_buy_volume_eth",
    "pre_label_buy_sell_volume_ratio",
    "pre_label_denom_token_transfer_ratio",
    "pre_label_active_blocks_last_10",
    "pre_label_tx_count_last_10",
    "pre_label_active_blocks_last_100",
    "pre_label_tx_count_last_100",
    "pre_label_active_density_last_10",
    "pre_label_active_density_last_100",
    "pre_label_tx_share_last_10",
    "pre_label_tx_share_last_100",
    "network_address_count_current",
    "network_node_count_current",
    "network_edge_count_current",
    "network_feature_scope",
    "activity_feature_scope",
    "lp_feature_scope",
    "removal_tx",
    "burn_event_present",
    "evidence_summary",
]

WINDOW_COLUMNS = [
    "row_id",
    "row_kind",
    "chain",
    "source_run_id",
    "token",
    "pool",
    "protocol",
    "symbol",
    "target_mechanism",
    "label_block",
    "as_of_block",
    "blocks_before_removal",
    "prediction_horizon_blocks",
    "target_removal_within_horizon",
    "observed_until_block",
    "control_reason",
    "trading_enabled_block",
    "pool_created_block",
    "token_created_block",
    "blocks_since_trading_enabled_as_of",
    "blocks_since_pool_created_as_of",
    "blocks_since_token_created_as_of",
    "creator_address",
    "current_owner",
    "ownership_renounced_current",
    "current_owner_is_zero_address",
    "current_owner_is_creator",
    "token_total_supply_scaled",
    "liquidity_eth_as_of",
    "price_ratio_to_initial_as_of",
    "max_liquidity_eth_before_as_of",
    "liquidity_drawdown_from_peak_as_of",
    "buy_tax_current",
    "sell_tax_current",
    "tax_bucket_current",
    "lp_approval_count_as_of",
    "lp_first_approval_block_as_of",
    "lp_last_approval_block_as_of",
    "lp_last_approval_owner_as_of",
    "lp_last_approval_spender_as_of",
    "lp_last_approval_is_router_as_of",
    "lp_last_approval_owner_is_creator_as_of",
    "lp_last_approval_owner_is_current_owner_as_of",
    "blocks_first_lp_approval_to_as_of",
    "blocks_last_lp_approval_to_as_of",
    "blocks_last_lp_approval_to_removal",
    "lp_approved_holder_count_as_of",
    "lp_approved_spender_count_as_of",
    "lp_router_approval_seen_as_of",
    "lp_max_approval_amount_as_of",
    "lp_approval_feature_scope",
    "activity_blocks_to_as_of",
    "tx_count_to_as_of",
    "token_transfer_count_to_as_of",
    "denom_transfer_count_to_as_of",
    "buy_volume_eth_to_as_of",
    "sell_volume_eth_to_as_of",
    "bribe_eth_to_as_of",
    "first_activity_block_to_as_of",
    "last_activity_block_to_as_of",
    "blocks_since_last_activity_as_of",
    "activity_density_to_as_of",
    "tx_per_active_block_to_as_of",
    "net_buy_volume_eth_to_as_of",
    "buy_sell_volume_ratio_to_as_of",
    "denom_token_transfer_ratio_to_as_of",
    "active_blocks_last_10_as_of",
    "tx_count_last_10_as_of",
    "active_blocks_last_50_as_of",
    "tx_count_last_50_as_of",
    "active_blocks_last_100_as_of",
    "tx_count_last_100_as_of",
    "active_density_last_10_as_of",
    "active_density_last_50_as_of",
    "active_density_last_100_as_of",
    "tx_share_last_10_to_total_as_of",
    "tx_share_last_50_to_total_as_of",
    "tx_share_last_100_to_total_as_of",
    "network_address_count_current",
    "network_node_count_current",
    "network_edge_count_current",
    "network_feature_scope",
    "activity_feature_scope",
]


def main() -> None:
    args = parse_args()
    api = Api(args.api_base, args.detail_retries, args.detail_retry_delay_secs)
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
    token_summaries = get_nonempty_run_view(
        api,
        args.run_id,
        "tokens",
        "tokens",
        "indexed_tokens",
        args.view_retries,
        args.view_retry_delay_secs,
    )
    token_by_address = {
        lower(token.get("contract_address")): token
        for token in token_summaries
        if token.get("contract_address")
    }

    rows = build_rows(args.run_id, api, pools, token_by_address)
    window_rows = build_window_rows(args.run_id, api, pools, token_by_address, args.window_offsets)
    control_window_rows = build_control_window_rows(
        args.run_id,
        api,
        pools,
        token_by_address,
        args.window_offsets,
        progress,
    )
    training_window_rows = window_rows + control_window_rows
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.summary.parent.mkdir(parents=True, exist_ok=True)
    args.window_output.parent.mkdir(parents=True, exist_ok=True)
    args.training_output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=FEATURE_COLUMNS)
        writer.writeheader()
        writer.writerows(rows)
    with args.window_output.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=WINDOW_COLUMNS)
        writer.writeheader()
        writer.writerows(window_rows)
    with args.training_output.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=WINDOW_COLUMNS)
        writer.writeheader()
        writer.writerows(training_window_rows)

    args.summary.write_text(
        render_summary(args.run_id, progress, rows, window_rows, control_window_rows),
        encoding="utf-8",
    )
    print(f"wrote {len(rows)} direct-LP feature rows to {args.output}")
    print(f"wrote {len(window_rows)} direct-LP window rows to {args.window_output}")
    print(f"wrote {len(training_window_rows)} direct-LP training window rows to {args.training_output}")
    print(f"wrote summary to {args.summary}")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--api-base", default=DEFAULT_API_BASE)
    parser.add_argument("--run-id", default=DEFAULT_RUN_ID)
    parser.add_argument(
        "--output",
        type=Path,
        default=Path("token_lab/scam_analytics/features/direct_lp_liquidity_removal_features.csv"),
    )
    parser.add_argument(
        "--summary",
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
    parser.add_argument(
        "--window-offsets",
        default="1,10,50,100,500",
        help="comma-separated blocks before removal to export as training windows",
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
    parser.add_argument(
        "--detail-retries",
        type=int,
        default=0,
        help="retry count for transient token-detail 404s while the active run swaps processors",
    )
    parser.add_argument(
        "--detail-retry-delay-secs",
        type=float,
        default=0.5,
        help="delay between transient token-detail retries",
    )
    args = parser.parse_args()
    args.window_offsets = parse_window_offsets(args.window_offsets)
    return args


def parse_window_offsets(raw: str) -> list[int]:
    offsets = []
    for item in raw.split(","):
        item = item.strip()
        if not item:
            continue
        value = int(item)
        if value <= 0:
            raise argparse.ArgumentTypeError("window offsets must be positive")
        offsets.append(value)
    return sorted(set(offsets))


class Api:
    def __init__(
        self,
        base_url: str,
        token_detail_retries: int = 0,
        token_detail_retry_delay_secs: float = 0.5,
    ) -> None:
        self.base_url = base_url.rstrip("/")
        self._detail_cache: dict[str, dict[str, Any]] = {}
        self.token_detail_retries = max(token_detail_retries, 0)
        self.token_detail_retry_delay_secs = max(token_detail_retry_delay_secs, 0.0)

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

    def token_detail(self, run_id: str, token: str) -> dict[str, Any]:
        key = lower(token)
        if key not in self._detail_cache:
            self._detail_cache[key] = self.fetch_token_detail_with_retries(run_id, token)
        return self._detail_cache[key]

    def fetch_token_detail_with_retries(self, run_id: str, token: str) -> dict[str, Any]:
        variants = [token]
        upper_token = token.upper()
        if upper_token != token:
            variants.append(upper_token)
        attempts = self.token_detail_retries + 1
        last_error: RuntimeError | None = None
        for attempt in range(attempts):
            for variant in variants:
                try:
                    return self.get(
                        f"/runs/{urllib.parse.quote(run_id)}/tokens/{urllib.parse.quote(variant)}"
                    )
                except RuntimeError as error:
                    if "HTTP 404" not in str(error):
                        raise
                    last_error = error
            if attempt < attempts - 1:
                time.sleep(self.token_detail_retry_delay_secs)
        return {
            "summary": {},
            "network": {},
            "pools": [],
            "token_detail_missing": True,
            "token_detail_error": str(last_error) if last_error else "token detail not found",
        }


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
    api: Api,
    pools: list[dict[str, Any]],
    token_by_address: dict[str, dict[str, Any]],
) -> list[dict[str, str]]:
    selected = [
        pool
        for pool in pools
        if pool.get("scam_mechanism") == "direct_lp_liquidity_removal"
    ]
    selected.sort(key=lambda pool: (label_block_for_pool(pool) or 10**18, lower(pool.get("token_address"))))
    rows = []
    seen: set[tuple[str, str]] = set()
    for pool in selected:
        token = lower(pool.get("token_address"))
        pool_address = lower(pool.get("pool_address"))
        if not token or not pool_address or (token, pool_address) in seen:
            continue
        seen.add((token, pool_address))
        token_summary = token_by_address.get(token, {})
        token_detail = api.token_detail(run_id, token)
        rows.append(row_from_pool(len(rows) + 1, run_id, pool, token_summary, token_detail))
    return rows


def build_window_rows(
    run_id: str,
    api: Api,
    pools: list[dict[str, Any]],
    token_by_address: dict[str, dict[str, Any]],
    offsets: list[int],
) -> list[dict[str, str]]:
    selected = [
        pool
        for pool in pools
        if pool.get("scam_mechanism") == "direct_lp_liquidity_removal"
    ]
    selected.sort(key=lambda pool: (label_block_for_pool(pool) or 10**18, lower(pool.get("token_address"))))
    rows = []
    seen: set[tuple[str, str]] = set()
    for pool in selected:
        token = lower(pool.get("token_address"))
        pool_address = lower(pool.get("pool_address"))
        if not token or not pool_address or (token, pool_address) in seen:
            continue
        seen.add((token, pool_address))
        token_summary = token_by_address.get(token, {})
        token_detail = api.token_detail(run_id, token)
        for offset in offsets:
            row = window_row_from_pool(
                len(rows) + 1,
                run_id,
                pool,
                token_summary,
                token_detail,
                offset,
            )
            if row:
                rows.append(row)
    return rows


def build_control_window_rows(
    run_id: str,
    api: Api,
    pools: list[dict[str, Any]],
    token_by_address: dict[str, dict[str, Any]],
    offsets: list[int],
    progress: dict[str, Any],
) -> list[dict[str, str]]:
    current_block = integer(progress.get("current_block")) or integer(progress.get("end_block"))
    control_reason = control_reason_for_progress(progress)
    selected = [pool for pool in pools if is_direct_lp_control_candidate(pool)]
    selected.sort(key=lambda pool: (lower(pool.get("token_address")), lower(pool.get("pool_address"))))
    rows = []
    seen: set[tuple[str, str]] = set()
    for pool in selected:
        token = lower(pool.get("token_address"))
        pool_address = lower(pool.get("pool_address"))
        if not token or not pool_address or (token, pool_address) in seen:
            continue
        seen.add((token, pool_address))
        token_summary = token_by_address.get(token, {})
        token_detail = api.token_detail(run_id, token)
        observed_until = observed_until_block(pool, current_block)
        for offset in offsets:
            row = control_window_row_from_pool(
                len(rows) + 1,
                run_id,
                pool,
                token_summary,
                token_detail,
                offset,
                observed_until,
                control_reason,
            )
            if row:
                rows.append(row)
    return rows


def is_direct_lp_control_candidate(pool: dict[str, Any]) -> bool:
    if pool.get("scam_mechanism") or pool.get("is_scam"):
        return False
    if pool.get("trading_enabled") is not True:
        return False
    if pool.get("can_buy") is not True or pool.get("can_sell") is not True:
        return False
    if str(pool.get("liquidity_level") or "").lower() != "liquid":
        return False
    return True


def observed_until_block(pool: dict[str, Any], current_block: int | None) -> int | None:
    candidates = [
        integer(pool.get("latest_block_number")),
        integer((pool.get("runtime_state") or {}).get("last_update_block")),
        current_block,
    ]
    candidates = [candidate for candidate in candidates if candidate is not None]
    return min(candidates) if candidates else None


def control_reason_for_progress(progress: dict[str, Any]) -> str:
    if str(progress.get("status") or "").lower() == "completed":
        return "no_scam_mechanism_trading_liquid_observed_through_completed_range"
    return "no_scam_mechanism_trading_liquid_observed_negative_so_far"


def row_from_pool(
    index: int,
    run_id: str,
    pool: dict[str, Any],
    token_summary: dict[str, Any],
    token_detail: dict[str, Any],
) -> dict[str, str]:
    pool = pool_with_detail(pool, token_detail)
    evidence = pool.get("scam_mechanism_evidence") or {}
    drain = evidence.get("drain") or {}
    burn_event = evidence.get("burn_event") or {}
    token_detail_summary = token_detail.get("summary") or {}
    network = token_detail.get("network") or {}
    trading_enabled_block, _ = trading_enabled(pool)
    label_block = label_block_for_pool(pool)
    as_of_block = label_block - 1 if label_block is not None and label_block > 0 else None
    blocks_to_removal = safe_diff(label_block, trading_enabled_block)
    minutes_to_removal = (
        blocks_to_removal * ETH_SECONDS_PER_BLOCK / 60.0 if blocks_to_removal is not None else None
    )
    lp_events = approval_events(pool)
    lp_pre_removal_events = [
        event
        for event in lp_events
        if label_block is not None and event["block"] < label_block
    ]
    lp_same_block_events = [
        event
        for event in lp_events
        if label_block is not None and event["block"] == label_block
    ]
    lp_first_approval = lp_events[0] if lp_events else {}
    lp_last_approval = lp_events[-1] if lp_events else {}
    lp_first_pre_removal_approval = lp_pre_removal_events[0] if lp_pre_removal_events else {}
    lp_last_pre_removal_approval = lp_pre_removal_events[-1] if lp_pre_removal_events else {}
    lp_first_approval_block = integer(lp_first_approval.get("block"))
    lp_last_approval_block = integer(lp_last_approval.get("block"))
    lp_first_pre_removal_approval_block = integer(lp_first_pre_removal_approval.get("block"))
    lp_last_pre_removal_approval_block = integer(lp_last_pre_removal_approval.get("block"))
    creator_address = lower(token_summary.get("creator_address"))
    current_owner = lower(token_summary.get("current_owner"))
    pool_created_block = integer(pool.get("creation_block"))
    token_created_block = integer(token_summary.get("creation_block"))
    lp_last_approval_pre = (
        lp_last_approval_block is not None and label_block is not None and lp_last_approval_block < label_block
    )
    lp_same_tx_approval = bool(lp_same_block_events)
    pre_activity = pre_label_activity(token_detail_summary, as_of_block)
    last_10 = activity_window(token_detail_summary, label_block, 10)
    last_100 = activity_window(token_detail_summary, label_block, 100)
    top_lp_share = top_lp_holder_share(pool)
    top_position_share = top_liquidity_position_share(pool)
    blocks_token_to_removal = safe_diff(label_block, token_created_block)
    pre_label_activity_start = earliest_block(
        integer(pre_activity["first_activity_block"]),
        trading_enabled_block,
    )
    pre_label_span = positive_span(as_of_block, pre_label_activity_start)

    return {
        "row_id": f"directlp-{index:04d}",
        "chain": "ethereum",
        "source_run_id": run_id,
        "token": lower(pool.get("token_address")),
        "pool": lower(pool.get("pool_address")),
        "protocol": text(pool.get("protocol")),
        "symbol": text(pool.get("token_symbol")),
        "target_mechanism": "direct_lp_liquidity_removal",
        "confidence": confidence_for_pool(pool, evidence),
        "as_of_block": text_int(as_of_block),
        "label_block": text_int(label_block),
        "trading_enabled_block": text_int(trading_enabled_block),
        "blocks_from_trading_enabled_to_removal": text_int(blocks_to_removal),
        "minutes_from_trading_enabled_to_removal": num(minutes_to_removal),
        "time_to_scam_bucket": time_bucket(blocks_to_removal),
        "pool_created_block": text_int(pool_created_block),
        "token_created_block": text_int(token_created_block),
        "blocks_token_created_to_removal": text_int(blocks_token_to_removal),
        "creator_address": creator_address,
        "current_owner": current_owner,
        "ownership_renounced_current": bool_text(token_summary.get("ownership_renounced")),
        "current_owner_is_zero_address": is_zero_address_text(current_owner),
        "current_owner_is_creator": address_match_text(current_owner, creator_address),
        "token_total_supply_scaled": num(pool.get("token_total_supply_scaled")),
        "pool_token_supply_percent_at_label": num(pool.get("pooled_token_supply_percent")),
        "price_ratio_to_initial_at_label": num(pool.get("price_ratio_to_initial")),
        "liquidity_eth_at_label": num(pool.get("total_liquidity")),
        "liquidity_level_at_label": text(pool.get("liquidity_level")),
        "max_liquidity_eth_before_removal": num(drain.get("max_denom_reserve_before")),
        "denom_reserve_ratio_to_max_at_removal": num(drain.get("denom_reserve_ratio_to_max")),
        "denom_reserve_at_removal": num(drain.get("denom_reserve")),
        "token_reserve_at_removal": num(drain.get("token_reserve")),
        "buy_tax_at_label": num(pool.get("buy_tax")),
        "sell_tax_at_label": num(pool.get("sell_tax")),
        "tax_bucket_at_label": text(pool.get("tax_bucket")),
        "can_buy_at_label": bool_text(pool.get("can_buy")),
        "can_sell_at_label": bool_text(pool.get("can_sell")),
        "has_observed_buy": bool_text(pool.get("has_observed_buy")),
        "has_observed_sell": bool_text(pool.get("has_observed_sell")),
        "lp_supply_known": bool_text(pool.get("lp_supply_known")),
        "lp_supply_status": text(pool.get("lp_supply_status")),
        "lp_total_supply_current": num(pool.get("lp_total_supply")),
        "lp_holder_count_current": text_int(pool.get("lp_holder_count")),
        "lp_transfer_count_current": text_int(pool.get("lp_transfer_count")),
        "lp_approval_count_current": text_int(pool.get("lp_approval_count")),
        "lp_holders_with_approvals_current": text_int(len(pool.get("lp_holders_with_approvals") or [])),
        "lp_approved_percentage_current": num(pool.get("lp_approved_percentage")),
        "lp_total_approved_to_routers_current": num(pool.get("lp_total_approved_to_routers")),
        "lp_first_approval_block": text_int(lp_first_approval_block),
        "lp_last_approval_block": text_int(lp_last_approval_block),
        "lp_any_approval_pre_removal": bool_text(bool(lp_pre_removal_events)),
        "lp_approval_count_pre_removal": text_int(len(lp_pre_removal_events)),
        "lp_approval_count_same_block": text_int(len(lp_same_block_events)),
        "lp_first_pre_removal_approval_block": text_int(lp_first_pre_removal_approval_block),
        "lp_last_pre_removal_approval_block": text_int(lp_last_pre_removal_approval_block),
        "lp_first_pre_removal_approval_owner": lower(lp_first_pre_removal_approval.get("owner")),
        "lp_first_pre_removal_approval_spender": lower(lp_first_pre_removal_approval.get("spender")),
        "lp_first_pre_removal_approval_is_router": bool_text(lp_first_pre_removal_approval.get("is_router")),
        "lp_last_pre_removal_approval_owner": lower(lp_last_pre_removal_approval.get("owner")),
        "lp_last_pre_removal_approval_spender": lower(lp_last_pre_removal_approval.get("spender")),
        "lp_last_pre_removal_approval_is_router": bool_text(lp_last_pre_removal_approval.get("is_router")),
        "lp_last_pre_removal_approval_owner_is_creator": address_match_text(
            lp_last_pre_removal_approval.get("owner"),
            creator_address,
        ),
        "lp_last_pre_removal_approval_owner_is_current_owner": address_match_text(
            lp_last_pre_removal_approval.get("owner"),
            current_owner,
        ),
        "blocks_lp_first_approval_to_removal": text_int(
            safe_diff(label_block, lp_first_approval_block)
            if lp_first_approval_block is not None
            and label_block is not None
            and lp_first_approval_block <= label_block
            else None
        ),
        "blocks_lp_first_pre_removal_approval_to_removal": text_int(
            safe_diff(label_block, lp_first_pre_removal_approval_block)
        ),
        "blocks_lp_last_pre_removal_approval_to_removal": text_int(
            safe_diff(label_block, lp_last_pre_removal_approval_block)
        ),
        "blocks_trading_enabled_to_lp_first_approval": text_int(
            safe_diff(lp_first_approval_block, trading_enabled_block)
        ),
        "blocks_trading_enabled_to_lp_first_pre_removal_approval": text_int(
            safe_diff(lp_first_pre_removal_approval_block, trading_enabled_block)
        ),
        "blocks_trading_enabled_to_lp_last_pre_removal_approval": text_int(
            safe_diff(lp_last_pre_removal_approval_block, trading_enabled_block)
        ),
        "blocks_pool_created_to_lp_first_approval": text_int(safe_diff(lp_first_approval_block, pool_created_block)),
        "blocks_pool_created_to_lp_first_pre_removal_approval": text_int(
            safe_diff(lp_first_pre_removal_approval_block, pool_created_block)
        ),
        "blocks_pool_created_to_lp_last_pre_removal_approval": text_int(
            safe_diff(lp_last_pre_removal_approval_block, pool_created_block)
        ),
        "lp_last_approval_pre_removal": bool_text(lp_last_approval_pre),
        "blocks_lp_last_approval_to_removal": text_int(
            safe_diff(label_block, lp_last_approval_block) if lp_last_approval_pre else None
        ),
        "lp_last_approval_owner": lower(lp_last_approval.get("owner")),
        "lp_last_approval_spender": lower(lp_last_approval.get("spender")),
        "lp_last_approval_is_router": bool_text(lp_last_approval.get("is_router")),
        "lp_same_tx_approval_and_removal": bool_text(lp_same_tx_approval),
        "top_lp_holder_share_current": num(top_lp_share),
        "top_liquidity_position_share_current": num(top_position_share),
        "pre_label_activity_blocks": text_int(pre_activity["activity_blocks"]),
        "pre_label_tx_count": text_int(pre_activity["tx_count"]),
        "pre_label_token_transfer_count": text_int(pre_activity["token_transfer_count"]),
        "pre_label_denom_transfer_count": text_int(pre_activity["denom_transfer_count"]),
        "pre_label_buy_volume_eth": num(pre_activity["buy_volume_eth"]),
        "pre_label_sell_volume_eth": num(pre_activity["sell_volume_eth"]),
        "pre_label_bribe_eth": num(pre_activity["bribe_eth"]),
        "pre_label_first_activity_block": text_int(pre_activity["first_activity_block"]),
        "pre_label_last_activity_block": text_int(pre_activity["last_activity_block"]),
        "blocks_pre_label_last_activity_to_label": text_int(
            safe_diff(label_block, integer(pre_activity["last_activity_block"]))
        ),
        "pre_label_active_density": num(safe_ratio(pre_activity["activity_blocks"], pre_label_span)),
        "pre_label_tx_per_active_block": num(
            safe_ratio(pre_activity["tx_count"], pre_activity["activity_blocks"])
        ),
        "pre_label_net_buy_volume_eth": num(pre_activity["buy_volume_eth"] - pre_activity["sell_volume_eth"]),
        "pre_label_buy_sell_volume_ratio": num(
            safe_ratio(pre_activity["buy_volume_eth"], pre_activity["sell_volume_eth"])
        ),
        "pre_label_denom_token_transfer_ratio": num(
            safe_ratio(pre_activity["denom_transfer_count"], pre_activity["token_transfer_count"])
        ),
        "pre_label_active_blocks_last_10": text_int(last_10["activity_blocks"]),
        "pre_label_tx_count_last_10": text_int(last_10["tx_count"]),
        "pre_label_active_blocks_last_100": text_int(last_100["activity_blocks"]),
        "pre_label_tx_count_last_100": text_int(last_100["tx_count"]),
        "pre_label_active_density_last_10": num(safe_ratio(last_10["activity_blocks"], 10)),
        "pre_label_active_density_last_100": num(safe_ratio(last_100["activity_blocks"], 100)),
        "pre_label_tx_share_last_10": num(safe_ratio(last_10["tx_count"], pre_activity["tx_count"])),
        "pre_label_tx_share_last_100": num(safe_ratio(last_100["tx_count"], pre_activity["tx_count"])),
        "network_address_count_current": text_int(network.get("address_count")),
        "network_node_count_current": text_int(network.get("node_count")),
        "network_edge_count_current": text_int(network.get("edge_count")),
        "network_feature_scope": "current_token_detail_not_time_sliced",
        "activity_feature_scope": "recent_block_activity_filtered_to_as_of_block",
        "lp_feature_scope": lp_feature_scope(pool, label_block, lp_last_approval_block),
        "removal_tx": text(drain.get("tx_hash") or burn_event.get("tx_hash") or pool.get("liquidity_removal_tx_hash")),
        "burn_event_present": bool_text(bool(burn_event)),
        "evidence_summary": evidence_summary(pool, drain, burn_event),
    }


def window_row_from_pool(
    index: int,
    run_id: str,
    pool: dict[str, Any],
    token_summary: dict[str, Any],
    token_detail: dict[str, Any],
    offset: int,
) -> dict[str, str] | None:
    pool = pool_with_detail(pool, token_detail)
    label_block = label_block_for_pool(pool)
    if label_block is None or label_block <= offset:
        return None
    as_of_block = label_block - offset
    trading_enabled_block, _ = trading_enabled(pool)
    pool_created_block = integer(pool.get("creation_block"))
    token_created_block = integer(token_summary.get("creation_block"))
    earliest_blocks = [
        block
        for block in [trading_enabled_block, pool_created_block, token_created_block]
        if block is not None
    ]
    if earliest_blocks and as_of_block < max(earliest_blocks):
        return None
    token_detail_summary = token_detail.get("summary") or {}
    network = token_detail.get("network") or {}
    activity_to_as_of = pre_label_activity(token_detail_summary, as_of_block)
    last_10 = activity_window_ending(token_detail_summary, as_of_block, 10)
    last_50 = activity_window_ending(token_detail_summary, as_of_block, 50)
    last_100 = activity_window_ending(token_detail_summary, as_of_block, 100)
    lp_events = approval_events(pool)
    lp_events_as_of = [event for event in lp_events if event["block"] <= as_of_block]
    first_approval_block = min((event["block"] for event in lp_events_as_of), default=None)
    last_approval_block = max((event["block"] for event in lp_events_as_of), default=None)
    last_approval_event = lp_events_as_of[-1] if lp_events_as_of else {}
    holders = {event["owner"] for event in lp_events_as_of if event["owner"]}
    spenders = {event["spender"] for event in lp_events_as_of if event["spender"]}
    max_approval = max((event["amount"] for event in lp_events_as_of if event["amount"] is not None), default=None)
    liquidity_as_of = liquidity_value_as_of(pool, as_of_block)
    max_liquidity_as_of = max_liquidity_before_as_of(pool, as_of_block)
    price_ratio_as_of = price_ratio_value_as_of(pool, as_of_block)
    drawdown = None
    if liquidity_as_of is not None and max_liquidity_as_of and max_liquidity_as_of > 0:
        drawdown = 1.0 - (liquidity_as_of / max_liquidity_as_of)

    return {
        "row_id": f"directlp-window-{index:05d}",
        "row_kind": "positive",
        "chain": "ethereum",
        "source_run_id": run_id,
        "token": lower(pool.get("token_address")),
        "pool": lower(pool.get("pool_address")),
        "protocol": text(pool.get("protocol")),
        "symbol": text(pool.get("token_symbol")),
        "target_mechanism": "direct_lp_liquidity_removal",
        "label_block": text_int(label_block),
        "as_of_block": text_int(as_of_block),
        "blocks_before_removal": text_int(offset),
        "prediction_horizon_blocks": text_int(offset),
        "target_removal_within_horizon": "true",
        "observed_until_block": text_int(label_block),
        "control_reason": "",
        "trading_enabled_block": text_int(trading_enabled_block),
        "pool_created_block": text_int(pool_created_block),
        "token_created_block": text_int(token_created_block),
        "blocks_since_trading_enabled_as_of": text_int(safe_diff(as_of_block, trading_enabled_block)),
        "blocks_since_pool_created_as_of": text_int(safe_diff(as_of_block, pool_created_block)),
        "blocks_since_token_created_as_of": text_int(safe_diff(as_of_block, token_created_block)),
        "creator_address": lower(token_summary.get("creator_address")),
        "current_owner": lower(token_summary.get("current_owner")),
        "ownership_renounced_current": bool_text(token_summary.get("ownership_renounced")),
        "current_owner_is_zero_address": is_zero_address_text(token_summary.get("current_owner")),
        "current_owner_is_creator": address_match_text(
            token_summary.get("current_owner"),
            token_summary.get("creator_address"),
        ),
        "token_total_supply_scaled": num(pool.get("token_total_supply_scaled")),
        "liquidity_eth_as_of": num(liquidity_as_of),
        "price_ratio_to_initial_as_of": num(price_ratio_as_of),
        "max_liquidity_eth_before_as_of": num(max_liquidity_as_of),
        "liquidity_drawdown_from_peak_as_of": num(drawdown),
        "buy_tax_current": num(pool.get("buy_tax")),
        "sell_tax_current": num(pool.get("sell_tax")),
        "tax_bucket_current": text(pool.get("tax_bucket")),
        "lp_approval_count_as_of": text_int(len(lp_events_as_of)),
        "lp_first_approval_block_as_of": text_int(first_approval_block),
        "lp_last_approval_block_as_of": text_int(last_approval_block),
        "lp_last_approval_owner_as_of": lower(last_approval_event.get("owner")),
        "lp_last_approval_spender_as_of": lower(last_approval_event.get("spender")),
        "lp_last_approval_is_router_as_of": bool_text(last_approval_event.get("is_router")),
        "lp_last_approval_owner_is_creator_as_of": address_match_text(
            last_approval_event.get("owner"),
            token_summary.get("creator_address"),
        ),
        "lp_last_approval_owner_is_current_owner_as_of": address_match_text(
            last_approval_event.get("owner"),
            token_summary.get("current_owner"),
        ),
        "blocks_first_lp_approval_to_as_of": text_int(safe_diff(as_of_block, first_approval_block)),
        "blocks_last_lp_approval_to_as_of": text_int(safe_diff(as_of_block, last_approval_block)),
        "blocks_last_lp_approval_to_removal": text_int(safe_diff(label_block, last_approval_block)),
        "lp_approved_holder_count_as_of": text_int(len(holders)),
        "lp_approved_spender_count_as_of": text_int(len(spenders)),
        "lp_router_approval_seen_as_of": bool_text(any(event["is_router"] for event in lp_events_as_of)),
        "lp_max_approval_amount_as_of": num(max_approval),
        "lp_approval_feature_scope": "available_approval_events_filtered_to_as_of_block",
        "activity_blocks_to_as_of": text_int(activity_to_as_of["activity_blocks"]),
        "tx_count_to_as_of": text_int(activity_to_as_of["tx_count"]),
        "token_transfer_count_to_as_of": text_int(activity_to_as_of["token_transfer_count"]),
        "denom_transfer_count_to_as_of": text_int(activity_to_as_of["denom_transfer_count"]),
        "buy_volume_eth_to_as_of": num(activity_to_as_of["buy_volume_eth"]),
        "sell_volume_eth_to_as_of": num(activity_to_as_of["sell_volume_eth"]),
        "bribe_eth_to_as_of": num(activity_to_as_of["bribe_eth"]),
        **activity_shape_fields(activity_to_as_of, last_10, last_50, last_100, as_of_block, trading_enabled_block),
        "active_blocks_last_10_as_of": text_int(last_10["activity_blocks"]),
        "tx_count_last_10_as_of": text_int(last_10["tx_count"]),
        "active_blocks_last_50_as_of": text_int(last_50["activity_blocks"]),
        "tx_count_last_50_as_of": text_int(last_50["tx_count"]),
        "active_blocks_last_100_as_of": text_int(last_100["activity_blocks"]),
        "tx_count_last_100_as_of": text_int(last_100["tx_count"]),
        "active_density_last_10_as_of": num(safe_ratio(last_10["activity_blocks"], 10)),
        "active_density_last_50_as_of": num(safe_ratio(last_50["activity_blocks"], 50)),
        "active_density_last_100_as_of": num(safe_ratio(last_100["activity_blocks"], 100)),
        "tx_share_last_10_to_total_as_of": num(safe_ratio(last_10["tx_count"], activity_to_as_of["tx_count"])),
        "tx_share_last_50_to_total_as_of": num(safe_ratio(last_50["tx_count"], activity_to_as_of["tx_count"])),
        "tx_share_last_100_to_total_as_of": num(safe_ratio(last_100["tx_count"], activity_to_as_of["tx_count"])),
        "network_address_count_current": text_int(network.get("address_count")),
        "network_node_count_current": text_int(network.get("node_count")),
        "network_edge_count_current": text_int(network.get("edge_count")),
        "network_feature_scope": "current_token_detail_not_time_sliced",
        "activity_feature_scope": "recent_block_activity_filtered_to_as_of_block",
    }


def control_window_row_from_pool(
    index: int,
    run_id: str,
    pool: dict[str, Any],
    token_summary: dict[str, Any],
    token_detail: dict[str, Any],
    horizon: int,
    observed_until: int | None,
    control_reason: str,
) -> dict[str, str] | None:
    pool = pool_with_detail(pool, token_detail)
    if observed_until is None or observed_until <= horizon:
        return None
    as_of_block = observed_until - horizon
    trading_enabled_block, _ = trading_enabled(pool)
    pool_created_block = integer(pool.get("creation_block"))
    token_created_block = integer(token_summary.get("creation_block"))
    earliest_blocks = [
        block
        for block in [trading_enabled_block, pool_created_block, token_created_block]
        if block is not None
    ]
    if earliest_blocks and as_of_block < max(earliest_blocks):
        return None

    token_detail_summary = token_detail.get("summary") or {}
    network = token_detail.get("network") or {}
    activity_to_as_of = pre_label_activity(token_detail_summary, as_of_block)
    last_10 = activity_window_ending(token_detail_summary, as_of_block, 10)
    last_50 = activity_window_ending(token_detail_summary, as_of_block, 50)
    last_100 = activity_window_ending(token_detail_summary, as_of_block, 100)
    lp_events = approval_events(pool)
    lp_events_as_of = [event for event in lp_events if event["block"] <= as_of_block]
    first_approval_block = min((event["block"] for event in lp_events_as_of), default=None)
    last_approval_block = max((event["block"] for event in lp_events_as_of), default=None)
    last_approval_event = lp_events_as_of[-1] if lp_events_as_of else {}
    holders = {event["owner"] for event in lp_events_as_of if event["owner"]}
    spenders = {event["spender"] for event in lp_events_as_of if event["spender"]}
    max_approval = max((event["amount"] for event in lp_events_as_of if event["amount"] is not None), default=None)
    liquidity_as_of = liquidity_value_as_of(pool, as_of_block)
    max_liquidity_as_of = max_liquidity_before_as_of(pool, as_of_block)
    price_ratio_as_of = price_ratio_value_as_of(pool, as_of_block)
    drawdown = None
    if liquidity_as_of is not None and max_liquidity_as_of and max_liquidity_as_of > 0:
        drawdown = 1.0 - (liquidity_as_of / max_liquidity_as_of)

    return {
        "row_id": f"directlp-control-{index:05d}",
        "row_kind": "control",
        "chain": "ethereum",
        "source_run_id": run_id,
        "token": lower(pool.get("token_address")),
        "pool": lower(pool.get("pool_address")),
        "protocol": text(pool.get("protocol")),
        "symbol": text(pool.get("token_symbol")),
        "target_mechanism": "direct_lp_liquidity_removal",
        "label_block": "",
        "as_of_block": text_int(as_of_block),
        "blocks_before_removal": "",
        "prediction_horizon_blocks": text_int(horizon),
        "target_removal_within_horizon": "false",
        "observed_until_block": text_int(observed_until),
        "control_reason": control_reason,
        "trading_enabled_block": text_int(trading_enabled_block),
        "pool_created_block": text_int(pool_created_block),
        "token_created_block": text_int(token_created_block),
        "blocks_since_trading_enabled_as_of": text_int(safe_diff(as_of_block, trading_enabled_block)),
        "blocks_since_pool_created_as_of": text_int(safe_diff(as_of_block, pool_created_block)),
        "blocks_since_token_created_as_of": text_int(safe_diff(as_of_block, token_created_block)),
        "creator_address": lower(token_summary.get("creator_address")),
        "current_owner": lower(token_summary.get("current_owner")),
        "ownership_renounced_current": bool_text(token_summary.get("ownership_renounced")),
        "current_owner_is_zero_address": is_zero_address_text(token_summary.get("current_owner")),
        "current_owner_is_creator": address_match_text(
            token_summary.get("current_owner"),
            token_summary.get("creator_address"),
        ),
        "token_total_supply_scaled": num(pool.get("token_total_supply_scaled")),
        "liquidity_eth_as_of": num(liquidity_as_of),
        "price_ratio_to_initial_as_of": num(price_ratio_as_of),
        "max_liquidity_eth_before_as_of": num(max_liquidity_as_of),
        "liquidity_drawdown_from_peak_as_of": num(drawdown),
        "buy_tax_current": num(pool.get("buy_tax")),
        "sell_tax_current": num(pool.get("sell_tax")),
        "tax_bucket_current": text(pool.get("tax_bucket")),
        "lp_approval_count_as_of": text_int(len(lp_events_as_of)),
        "lp_first_approval_block_as_of": text_int(first_approval_block),
        "lp_last_approval_block_as_of": text_int(last_approval_block),
        "lp_last_approval_owner_as_of": lower(last_approval_event.get("owner")),
        "lp_last_approval_spender_as_of": lower(last_approval_event.get("spender")),
        "lp_last_approval_is_router_as_of": bool_text(last_approval_event.get("is_router")),
        "lp_last_approval_owner_is_creator_as_of": address_match_text(
            last_approval_event.get("owner"),
            token_summary.get("creator_address"),
        ),
        "lp_last_approval_owner_is_current_owner_as_of": address_match_text(
            last_approval_event.get("owner"),
            token_summary.get("current_owner"),
        ),
        "blocks_first_lp_approval_to_as_of": text_int(safe_diff(as_of_block, first_approval_block)),
        "blocks_last_lp_approval_to_as_of": text_int(safe_diff(as_of_block, last_approval_block)),
        "blocks_last_lp_approval_to_removal": "",
        "lp_approved_holder_count_as_of": text_int(len(holders)),
        "lp_approved_spender_count_as_of": text_int(len(spenders)),
        "lp_router_approval_seen_as_of": bool_text(any(event["is_router"] for event in lp_events_as_of)),
        "lp_max_approval_amount_as_of": num(max_approval),
        "lp_approval_feature_scope": "available_approval_events_filtered_to_as_of_block",
        "activity_blocks_to_as_of": text_int(activity_to_as_of["activity_blocks"]),
        "tx_count_to_as_of": text_int(activity_to_as_of["tx_count"]),
        "token_transfer_count_to_as_of": text_int(activity_to_as_of["token_transfer_count"]),
        "denom_transfer_count_to_as_of": text_int(activity_to_as_of["denom_transfer_count"]),
        "buy_volume_eth_to_as_of": num(activity_to_as_of["buy_volume_eth"]),
        "sell_volume_eth_to_as_of": num(activity_to_as_of["sell_volume_eth"]),
        "bribe_eth_to_as_of": num(activity_to_as_of["bribe_eth"]),
        **activity_shape_fields(activity_to_as_of, last_10, last_50, last_100, as_of_block, trading_enabled_block),
        "active_blocks_last_10_as_of": text_int(last_10["activity_blocks"]),
        "tx_count_last_10_as_of": text_int(last_10["tx_count"]),
        "active_blocks_last_50_as_of": text_int(last_50["activity_blocks"]),
        "tx_count_last_50_as_of": text_int(last_50["tx_count"]),
        "active_blocks_last_100_as_of": text_int(last_100["activity_blocks"]),
        "tx_count_last_100_as_of": text_int(last_100["tx_count"]),
        "active_density_last_10_as_of": num(safe_ratio(last_10["activity_blocks"], 10)),
        "active_density_last_50_as_of": num(safe_ratio(last_50["activity_blocks"], 50)),
        "active_density_last_100_as_of": num(safe_ratio(last_100["activity_blocks"], 100)),
        "tx_share_last_10_to_total_as_of": num(safe_ratio(last_10["tx_count"], activity_to_as_of["tx_count"])),
        "tx_share_last_50_to_total_as_of": num(safe_ratio(last_50["tx_count"], activity_to_as_of["tx_count"])),
        "tx_share_last_100_to_total_as_of": num(safe_ratio(last_100["tx_count"], activity_to_as_of["tx_count"])),
        "network_address_count_current": text_int(network.get("address_count")),
        "network_node_count_current": text_int(network.get("node_count")),
        "network_edge_count_current": text_int(network.get("edge_count")),
        "network_feature_scope": "current_token_detail_not_time_sliced",
        "activity_feature_scope": "recent_block_activity_filtered_to_as_of_block",
    }


def pool_with_detail(pool: dict[str, Any], token_detail: dict[str, Any]) -> dict[str, Any]:
    pool_address = lower(pool.get("pool_address"))
    pool_id = lower(pool.get("pool_id"))
    for candidate in token_detail.get("pools") or []:
        candidate_address = lower(candidate.get("pool_address"))
        candidate_id = lower(candidate.get("pool_id"))
        if (pool_address and candidate_address == pool_address) or (pool_id and candidate_id == pool_id):
            merged = dict(pool)
            merged.update(candidate)
            return merged
    return pool


def pre_label_activity(summary: dict[str, Any], as_of_block: int | None) -> dict[str, float | int]:
    rows = []
    for activity in summary.get("recent_block_activity") or []:
        block = integer(activity.get("block_number"))
        if block is None or as_of_block is None or block <= as_of_block:
            rows.append(activity)
    return aggregate_activity(rows)


def activity_window(summary: dict[str, Any], label_block: int | None, window_blocks: int) -> dict[str, float | int]:
    if label_block is None:
        return aggregate_activity([])
    start = label_block - window_blocks
    rows = []
    for activity in summary.get("recent_block_activity") or []:
        block = integer(activity.get("block_number"))
        if block is not None and start <= block < label_block:
            rows.append(activity)
    return aggregate_activity(rows)


def activity_window_ending(
    summary: dict[str, Any],
    as_of_block: int | None,
    window_blocks: int,
) -> dict[str, float | int]:
    if as_of_block is None:
        return aggregate_activity([])
    start = as_of_block - window_blocks + 1
    rows = []
    for activity in summary.get("recent_block_activity") or []:
        block = integer(activity.get("block_number"))
        if block is not None and start <= block <= as_of_block:
            rows.append(activity)
    return aggregate_activity(rows)


def aggregate_activity(rows: list[dict[str, Any]]) -> dict[str, float | int]:
    buy_volume = 0.0
    sell_volume = 0.0
    activity_block_numbers = [
        block
        for block in (integer(row.get("block_number")) for row in rows)
        if block is not None
    ]
    unique_activity_blocks = set(activity_block_numbers)
    for row in rows:
        buy_volume += sum_volume(row.get("buy_volume_by_denom") or {})
        sell_volume += sum_volume(row.get("sell_volume_by_denom") or {})
    return {
        "activity_blocks": len(unique_activity_blocks),
        "first_activity_block": min(unique_activity_blocks) if unique_activity_blocks else None,
        "last_activity_block": max(unique_activity_blocks) if unique_activity_blocks else None,
        "tx_count": sum(integer(row.get("num_tx")) or 0 for row in rows),
        "token_transfer_count": sum(integer(row.get("token_transfer_count")) or 0 for row in rows),
        "denom_transfer_count": sum(integer(row.get("denom_transfer_count")) or 0 for row in rows),
        "buy_volume_eth": buy_volume,
        "sell_volume_eth": sell_volume,
        "bribe_eth": sum(number(row.get("total_bribe_eth")) or 0.0 for row in rows),
    }


def activity_shape_fields(
    activity_to_as_of: dict[str, float | int],
    _last_10: dict[str, float | int],
    _last_50: dict[str, float | int],
    _last_100: dict[str, float | int],
    as_of_block: int | None,
    trading_enabled_block: int | None,
) -> dict[str, str]:
    last_activity_block = integer(activity_to_as_of["last_activity_block"])
    activity_start = earliest_block(
        integer(activity_to_as_of["first_activity_block"]),
        trading_enabled_block,
    )
    span = positive_span(as_of_block, activity_start)
    return {
        "first_activity_block_to_as_of": text_int(activity_to_as_of["first_activity_block"]),
        "last_activity_block_to_as_of": text_int(last_activity_block),
        "blocks_since_last_activity_as_of": text_int(safe_diff(as_of_block, last_activity_block)),
        "activity_density_to_as_of": num(safe_ratio(activity_to_as_of["activity_blocks"], span)),
        "tx_per_active_block_to_as_of": num(
            safe_ratio(activity_to_as_of["tx_count"], activity_to_as_of["activity_blocks"])
        ),
        "net_buy_volume_eth_to_as_of": num(
            activity_to_as_of["buy_volume_eth"] - activity_to_as_of["sell_volume_eth"]
        ),
        "buy_sell_volume_ratio_to_as_of": num(
            safe_ratio(activity_to_as_of["buy_volume_eth"], activity_to_as_of["sell_volume_eth"])
        ),
        "denom_token_transfer_ratio_to_as_of": num(
            safe_ratio(activity_to_as_of["denom_transfer_count"], activity_to_as_of["token_transfer_count"])
        ),
    }


def sum_volume(volumes: dict[str, Any]) -> float:
    total = 0.0
    for value in volumes.values():
        total += number(value) or 0.0
    return total


def top_lp_holder_share(pool: dict[str, Any]) -> float | None:
    shares = []
    for holder in pool.get("lp_holders") or []:
        share = number(holder.get("share"))
        if share is not None:
            shares.append(share)
    return max(shares) if shares else None


def top_liquidity_position_share(pool: dict[str, Any]) -> float | None:
    shares = []
    for position in pool.get("liquidity_positions") or []:
        share = number(position.get("position_share_pct"))
        if share is not None:
            shares.append(share)
    return max(shares) if shares else None


def approval_events(pool: dict[str, Any]) -> list[dict[str, Any]]:
    events = []
    approval_holder_fallbacks = [
        lower(holder)
        for holder in pool.get("lp_holders_with_approvals") or []
        if lower(holder)
    ]
    for holder in pool.get("lp_holders") or []:
        owner = lower(holder.get("address"))
        approvals = holder.get("approvals") or {}
        for spender, approval in approvals.items():
            block = integer(approval.get("block_number"))
            if block is None:
                continue
            events.append(
                {
                    "owner": owner,
                    "spender": lower(spender),
                    "block": block,
                    "amount": number(approval.get("amount")),
                    "is_router": bool(approval.get("is_router")),
                }
            )
    last = pool.get("lp_last_approval") or {}
    last_block = integer(last.get("block_number")) or integer(pool.get("lp_last_approval_block"))
    if last_block is not None:
        owner = lower(last.get("owner"))
        if not owner and len(approval_holder_fallbacks) == 1:
            owner = approval_holder_fallbacks[0]
        events.append(
            {
                "owner": owner,
                "spender": lower(last.get("spender")),
                "block": last_block,
                "amount": number(last.get("amount")),
                "is_router": bool(last.get("is_router")),
            }
        )
    deduped = {
        (event["owner"], event["spender"], event["block"], event["amount"]): event
        for event in events
    }
    events = list(deduped.values())
    events.sort(key=lambda event: (event["block"], event["owner"], event["spender"]))
    return events


def liquidity_value_as_of(pool: dict[str, Any], as_of_block: int | None) -> float | None:
    value = None
    for point in pool.get("liquidity_history") or []:
        block = integer(point.get("block_number"))
        if block is not None and as_of_block is not None and block <= as_of_block:
            value = number(point.get("liquidity"))
    return value


def max_liquidity_before_as_of(pool: dict[str, Any], as_of_block: int | None) -> float | None:
    values = []
    for point in pool.get("liquidity_history") or []:
        block = integer(point.get("block_number"))
        value = number(point.get("liquidity"))
        if block is not None and as_of_block is not None and block <= as_of_block and value is not None:
            values.append(value)
    return max(values) if values else None


def price_ratio_value_as_of(pool: dict[str, Any], as_of_block: int | None) -> float | None:
    value = None
    for point in pool.get("price_ratio_history") or []:
        block = integer(point.get("block_number"))
        if block is not None and as_of_block is not None and block <= as_of_block:
            value = number(point.get("ratio"))
    return value


def lp_feature_scope(pool: dict[str, Any], label_block: int | None, lp_last_approval_block: int | None) -> str:
    if label_block is None:
        return "missing_label_block"
    if lp_last_approval_block is None:
        return "current_lp_state_no_approval_observed"
    if lp_last_approval_block < label_block:
        return "current_lp_state_has_pre_removal_approval"
    if lp_last_approval_block == label_block:
        return "current_lp_state_approval_same_block_as_removal"
    return "current_lp_state_approval_after_removal"


def evidence_summary(pool: dict[str, Any], drain: dict[str, Any], burn_event: dict[str, Any]) -> str:
    parts = [
        f"removal_block={label_block_for_pool(pool)}",
        f"burn_event_present={bool(burn_event)}",
        f"max_liquidity_before={num(drain.get('max_denom_reserve_before'))}",
        f"reserve_ratio_to_max={num(drain.get('denom_reserve_ratio_to_max'))}",
    ]
    return "; ".join(parts)


def render_summary(
    run_id: str,
    progress: dict[str, Any],
    rows: list[dict[str, str]],
    window_rows: list[dict[str, str]],
    control_window_rows: list[dict[str, str]],
) -> str:
    confidence = Counter(row["confidence"] for row in rows)
    protocols = Counter(row["protocol"] for row in rows)
    buckets = Counter(row["time_to_scam_bucket"] for row in rows)
    lp_scope = Counter(row["lp_feature_scope"] for row in rows)
    any_pre_approval = Counter(row["lp_any_approval_pre_removal"] or "unknown" for row in rows)
    pre_approval = Counter(row["lp_last_approval_pre_removal"] or "unknown" for row in rows)
    same_block_approval = Counter(row["lp_same_tx_approval_and_removal"] or "unknown" for row in rows)
    first_approval_lead = Counter(
        bucket_or_unknown(row["blocks_lp_first_approval_to_removal"]) for row in rows
    )
    last_pre_approval_lead = Counter(
        bucket_or_unknown(row["blocks_lp_last_pre_removal_approval_to_removal"]) for row in rows
    )
    network_scope = Counter(row["network_feature_scope"] for row in rows)
    window_offsets = Counter(row["blocks_before_removal"] for row in window_rows)
    window_lp_approval = Counter(row["lp_approval_count_as_of"] != "0" for row in window_rows)
    training_rows = window_rows + control_window_rows
    row_kinds = Counter(row["row_kind"] for row in training_rows)
    training_targets = Counter(row["target_removal_within_horizon"] for row in training_rows)
    control_horizons = Counter(row["prediction_horizon_blocks"] for row in control_window_rows)
    control_reasons = Counter(row["control_reason"] for row in control_window_rows)
    completed_export = str(progress.get("status") or "").lower() == "completed"
    next_feature_work = [
        "- Add time-sliced LP holder and approval state at `as_of_block`.",
        "- Add time-sliced token-network graph features instead of current detail",
        "  graph counts.",
        "- Add matched-control sampling and class-balance policy after the full",
        "  range build completes.",
        "- Add fully time-sliced liquidity, tax, and authority histories instead of",
        "  current-state fields where noted by scope columns.",
    ]
    if not completed_export:
        next_feature_work.insert(
            2,
            "- Promote observed-negative controls after the token range build completes.",
        )
    return "\n".join(
        [
            "# Direct LP Liquidity Removal Feature Summary",
            "",
            f"Source run: `{run_id}`",
            "",
            "Rows are one pool each for `direct_lp_liquidity_removal`. The label",
            "block is the liquidity-removal/reserve-drain block. `as_of_block` is",
            "`label_block - 1`; activity features are filtered to that block.",
            "Current LP and network fields are marked with scope columns because",
            "the token-server summary is not fully time-sliced yet.",
            "",
            "## Source Progress",
            "",
            f"- Range: `{progress.get('start_block')}..{progress.get('end_block')}`",
            f"- Status at export: `{progress.get('status')}`",
            f"- Blocks processed at export: `{progress.get('blocks_processed')}` / `{progress.get('total_blocks')}`",
            f"- Rows exported: `{len(rows)}`",
            f"- Window rows exported: `{len(window_rows)}`",
            f"- Control window rows exported: `{len(control_window_rows)}`",
            "",
            "## Time To Removal",
            "",
            table(buckets, "Bucket"),
            "",
            "## LP Approval Timing",
            "",
            table(any_pre_approval, "Any Pre-Removal Approval"),
            "",
            table(pre_approval, "Pre-Removal Approval"),
            "",
            table(same_block_approval, "Same-Block Approval"),
            "",
            table(first_approval_lead, "First Approval Lead To Removal"),
            "",
            table(last_pre_approval_lead, "Last Pre-Removal Approval Lead"),
            "",
            stats_table(
                rows,
                [
                    ("Trading Enabled To Removal", "blocks_from_trading_enabled_to_removal"),
                    ("First Pre-Removal LP Approval To Removal", "blocks_lp_first_pre_removal_approval_to_removal"),
                    (
                        "Last Pre-Removal LP Approval To Removal",
                        "blocks_lp_last_pre_removal_approval_to_removal",
                    ),
                    (
                        "Trading Enabled To First Pre-Removal LP Approval",
                        "blocks_trading_enabled_to_lp_first_pre_removal_approval",
                    ),
                    (
                        "Trading Enabled To Last Pre-Removal LP Approval",
                        "blocks_trading_enabled_to_lp_last_pre_removal_approval",
                    ),
                ],
            ),
            "",
            "Negative trading-to-approval values mean the LP approval was already",
            "visible before the selected `trading_enabled_block`.",
            "",
            "## LP Feature Scope",
            "",
            table(lp_scope, "Scope"),
            "",
            "## Confidence",
            "",
            table(confidence, "Confidence"),
            "",
            "## Protocols",
            "",
            table(protocols, "Protocol"),
            "",
            "## Network Feature Scope",
            "",
            table(network_scope, "Scope"),
            "",
            "## Feature Coverage",
            "",
            coverage_table(
                rows,
                [
                    ("Price Ratio At Label", "price_ratio_to_initial_at_label"),
                    ("Liquidity At Label", "liquidity_eth_at_label"),
                    ("First Pre-Removal LP Approval", "lp_first_pre_removal_approval_block"),
                    ("Last Approval Owner Is Creator", "lp_last_pre_removal_approval_owner_is_creator"),
                    ("Last Approval Owner Is Current Owner", "lp_last_pre_removal_approval_owner_is_current_owner"),
                    ("Pre-Label Activity Tx Count", "pre_label_tx_count"),
                    ("Pre-Label Activity Density", "pre_label_active_density"),
                    ("Pre-Label Buy/Sell Volume Ratio", "pre_label_buy_sell_volume_ratio"),
                    ("Current Network Address Count", "network_address_count_current"),
                ],
            ),
            "",
            "## Window Rows",
            "",
            table(window_offsets, "Blocks Before Removal"),
            "",
            table(Counter({str(key).lower(): value for key, value in window_lp_approval.items()}), "LP Approval Seen As Of"),
            "",
            "## Training Windows",
            "",
            table(row_kinds, "Row Kind"),
            "",
            table(training_targets, "Target Removal Within Horizon"),
            "",
            table(control_horizons, "Control Horizon Blocks"),
            "",
            table(control_reasons, "Control Reason"),
            "",
            "## Horizon Signal Availability",
            "",
            horizon_signal_table(training_rows),
            "",
            "## Training Window Feature Coverage",
            "",
            coverage_table(
                training_rows,
                [
                    ("Liquidity As Of", "liquidity_eth_as_of"),
                    ("Price Ratio As Of", "price_ratio_to_initial_as_of"),
                    ("LP Last Approval As Of", "lp_last_approval_block_as_of"),
                    ("LP Approval Owner Is Creator As Of", "lp_last_approval_owner_is_creator_as_of"),
                    (
                        "LP Approval Owner Is Current Owner As Of",
                        "lp_last_approval_owner_is_current_owner_as_of",
                    ),
                    ("Activity Density As Of", "activity_density_to_as_of"),
                    ("Tx Share Last 10", "tx_share_last_10_to_total_as_of"),
                    ("Current Network Address Count", "network_address_count_current"),
                ],
            ),
            "",
            "## Next Feature Work",
            "",
            *next_feature_work,
            "",
        ]
    )


def table(counter: Counter[str], label: str) -> str:
    lines = [f"| {label} | Count |", "| --- | ---: |"]
    for key, value in sorted(counter.items(), key=lambda item: (-item[1], item[0])):
        lines.append(f"| `{key or 'unknown'}` | {value} |")
    return "\n".join(lines)


def stats_table(rows: list[dict[str, str]], specs: list[tuple[str, str]]) -> str:
    lines = [
        "| Metric | Count | Min | P25 | Median | P75 | Max |",
        "| --- | ---: | ---: | ---: | ---: | ---: | ---: |",
    ]
    for label, column in specs:
        values = sorted(number(row.get(column)) for row in rows if number(row.get(column)) is not None)
        if not values:
            lines.append(f"| {label} | 0 |  |  |  |  |  |")
            continue
        lines.append(
            "| {label} | {count} | {min_value} | {p25} | {median} | {p75} | {max_value} |".format(
                label=label,
                count=len(values),
                min_value=num(values[0]),
                p25=num(quantile(values, 0.25)),
                median=num(quantile(values, 0.50)),
                p75=num(quantile(values, 0.75)),
                max_value=num(values[-1]),
            )
        )
    return "\n".join(lines)


def coverage_table(rows: list[dict[str, str]], specs: list[tuple[str, str]]) -> str:
    lines = [
        "| Field | Rows | Non-Empty | Non-Zero | Non-Empty % |",
        "| --- | ---: | ---: | ---: | ---: |",
    ]
    total = len(rows)
    for label, column in specs:
        non_empty = [row.get(column) for row in rows if row.get(column) not in ("", None)]
        non_zero = 0
        for value in non_empty:
            parsed = number(value)
            if parsed is None:
                non_zero += 1
            elif parsed != 0:
                non_zero += 1
        lines.append(
            f"| {label} | {total} | {len(non_empty)} | {non_zero} | {pct(len(non_empty), total)} |"
        )
    return "\n".join(lines)


def horizon_signal_table(rows: list[dict[str, str]]) -> str:
    grouped: dict[tuple[str, int], list[dict[str, str]]] = {}
    for row in rows:
        horizon = integer(row.get("prediction_horizon_blocks"))
        if horizon is None:
            continue
        grouped.setdefault((row.get("row_kind") or "unknown", horizon), []).append(row)

    lines = [
        "| Row Kind | Horizon Blocks | Rows | LP Approval Seen | Approval Seen % | Median Activity Density | Median Tx Last 10 | Median Tx Last 100 | Median Net Buy ETH |",
        "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
    ]
    for (row_kind, horizon), group in sorted(grouped.items(), key=lambda item: (item[0][0], item[0][1])):
        approval_seen = sum((integer(row.get("lp_approval_count_as_of")) or 0) > 0 for row in group)
        activity_density = sorted(
            number(row.get("activity_density_to_as_of"))
            for row in group
            if number(row.get("activity_density_to_as_of")) is not None
        )
        last_10 = sorted(
            number(row.get("tx_count_last_10_as_of"))
            for row in group
            if number(row.get("tx_count_last_10_as_of")) is not None
        )
        last_100 = sorted(
            number(row.get("tx_count_last_100_as_of"))
            for row in group
            if number(row.get("tx_count_last_100_as_of")) is not None
        )
        net_buy = sorted(
            number(row.get("net_buy_volume_eth_to_as_of"))
            for row in group
            if number(row.get("net_buy_volume_eth_to_as_of")) is not None
        )
        lines.append(
            "| `{row_kind}` | {horizon} | {count} | {approval_seen} | {approval_pct} | {activity_density} | {last_10} | {last_100} | {net_buy} |".format(
                row_kind=row_kind,
                horizon=horizon,
                count=len(group),
                approval_seen=approval_seen,
                approval_pct=pct(approval_seen, len(group)),
                activity_density=num(quantile(activity_density, 0.50)) if activity_density else "",
                last_10=num(quantile(last_10, 0.50)) if last_10 else "",
                last_100=num(quantile(last_100, 0.50)) if last_100 else "",
                net_buy=num(quantile(net_buy, 0.50)) if net_buy else "",
            )
        )
    return "\n".join(lines)


def quantile(values: list[float], q: float) -> float:
    if not values:
        return math.nan
    if len(values) == 1:
        return values[0]
    position = (len(values) - 1) * q
    lower_index = math.floor(position)
    upper_index = math.ceil(position)
    if lower_index == upper_index:
        return values[int(position)]
    lower = values[lower_index]
    upper = values[upper_index]
    return lower + (upper - lower) * (position - lower_index)


def bucket_or_unknown(value: Any) -> str:
    parsed = integer(value)
    if parsed is None:
        return "unknown"
    return time_bucket(parsed)


def pct(part: int, total: int) -> str:
    if total == 0:
        return ""
    return f"{(part / total) * 100.0:.1f}%"


def positive_span(end_block: int | None, start_block: int | None) -> int | None:
    if end_block is None or start_block is None or end_block < start_block:
        return None
    return end_block - start_block + 1


def earliest_block(*blocks: int | None) -> int | None:
    known = [block for block in blocks if block is not None]
    return min(known) if known else None


def safe_ratio(numerator: Any, denominator: Any) -> float | None:
    parsed_numerator = number(numerator)
    parsed_denominator = number(denominator)
    if parsed_numerator is None or parsed_denominator is None or parsed_denominator == 0:
        return None
    return parsed_numerator / parsed_denominator


def safe_diff(left: int | None, right: int | None) -> int | None:
    if left is None or right is None:
        return None
    return left - right


def lower(value: Any) -> str:
    return str(value or "").lower()


def text(value: Any) -> str:
    return "" if value is None else str(value)


def bool_text(value: Any) -> str:
    if value is None:
        return ""
    return "true" if bool(value) else "false"


def is_zero_address_text(value: Any) -> str:
    address = lower(value)
    if not address:
        return ""
    return bool_text(address == ZERO_ADDRESS)


def address_match_text(left: Any, right: Any) -> str:
    left_address = lower(left)
    right_address = lower(right)
    if not left_address or not right_address:
        return ""
    return bool_text(left_address == right_address)


def integer(value: Any) -> int | None:
    if value is None or value == "":
        return None
    try:
        return int(value)
    except (TypeError, ValueError):
        return None


def num(value: Any) -> str:
    parsed = number(value)
    if parsed is None:
        return ""
    return f"{parsed:.10g}"


if __name__ == "__main__":
    main()
