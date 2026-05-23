#!/usr/bin/env python3
"""Build pre-iteration token investigations from an OOT Risk Atlas run."""

from __future__ import annotations

import argparse
import json
import math
import re
import time
import urllib.error
import urllib.request
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import pandas as pd


MODELING_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_RUN_ID = "backdoor_horizon_oot_v1"
DEFAULT_ULIQ_TOKEN = "0x0ca72b24abf950be8bb145f5c68f7004485192b2"
DEFAULT_ULIQ_POOL = "0x57ca70b663ead85122589f345835d8f8209608dc"

FOUR_BLOCK_TARGET = "scam_within_4_chain_block_delta"

TIMELINE_COLUMNS = [
    "token_address",
    "pool_address",
    "block_number",
    "timestamp",
    "active_observation_index",
    "active_reasons",
    "tx_count",
    "token_transfer_count",
    "denom_transfer_count",
    "buy_volume_denom",
    "sell_volume_denom",
    "total_bribe_eth",
    "total_liquidity_denom",
    "denom_reserve",
    "token_reserve",
    "price_to_initial_ratio",
    "can_buy",
    "can_sell",
    "effective_can_buy",
    "effective_can_sell",
    "buy_tax",
    "sell_tax",
    "liquidity_removed_as_of",
    "liquidity_removal_in_block",
    "direct_lp_removal_as_of",
    "direct_lp_removal_in_block",
    "lp_approved_pct_as_of",
    "lp_approval_count_in_block",
    "lp_approval_seen_as_of",
    "lp_removable_pct_as_of",
    "creator_lp_balance_pct_as_of",
    "creator_lp_approved_pct_as_of",
    "creator_lp_removable_pct_as_of",
    "creator_lp_approved_gt_90_pct_as_of",
    "creator_lp_router_removable_gt_90_pct_as_of",
    "first_lp_approval_to_as_of_chain_block_delta",
    "last_lp_approval_to_as_of_chain_block_delta",
    "pool_creation_to_first_lp_approval_chain_block_delta",
    "trading_enabled_to_first_lp_approval_chain_block_delta",
    "control_transfer_from_after_renounce_seen_as_of",
    "control_transfer_from_after_renounce_in_block",
    "control_transfer_from_holder_to_burn_seen_as_of",
    "control_transfer_from_holder_to_burn_in_block",
    "control_transfer_from_pair_seen_as_of",
    "control_transfer_from_pair_in_block",
    "control_transfer_from_without_transfer_log_seen_as_of",
    "control_transfer_from_without_transfer_log_in_block",
    "pair_token_to_control_seen_as_of",
    "pair_token_to_control_in_block",
    "pair_token_to_control_to_pool_reserve_ratio",
    "pair_balance_backdoor_signal_seen_as_of",
    "pair_balance_backdoor_signal_in_block",
    "last_pair_balance_backdoor_signal_to_as_of_chain_block_delta",
    "token_transfer_to_total_supply_ratio",
    "token_transfer_to_pool_token_reserve_ratio",
    "scam_block",
    "time_to_scam_chain_block_delta",
    FOUR_BLOCK_TARGET,
    "scam_within_10_chain_block_delta",
]

SIGNAL_BOOL_COLUMNS = [
    "liquidity_removal_in_block",
    "direct_lp_removal_in_block",
    "lp_approval_seen_as_of",
    "control_transfer_from_after_renounce_seen_as_of",
    "control_transfer_from_after_renounce_in_block",
    "control_transfer_from_holder_to_burn_seen_as_of",
    "control_transfer_from_holder_to_burn_in_block",
    "control_transfer_from_pair_seen_as_of",
    "control_transfer_from_pair_in_block",
    "control_transfer_from_without_transfer_log_seen_as_of",
    "control_transfer_from_without_transfer_log_in_block",
    "pair_token_to_control_seen_as_of",
    "pair_token_to_control_in_block",
    "pair_balance_backdoor_signal_seen_as_of",
    "pair_balance_backdoor_signal_in_block",
]


@dataclass
class Case:
    slug: str
    title: str
    category: str
    selection_reason: str
    token_address: str
    pool_address: str
    selected_block: int
    selected_score: float | None
    selected_time_to_scam: float | None


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run-id", default=DEFAULT_RUN_ID)
    parser.add_argument(
        "--network-api-base",
        default="",
        help=(
            "Optional token analytics network API base, for example "
            "http://100.96.34.94:40019/eth/tokens/api/analytics/network"
        ),
    )
    parser.add_argument("--network-timeout-seconds", type=int, default=120)
    parser.add_argument("--skip-network", action="store_true")
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    artifact_root = MODELING_ROOT / "artifacts" / args.run_id
    report_root = artifact_root / "investigations" / "pre_iteration_02"
    report_root.mkdir(parents=True, exist_ok=True)

    dataset = pd.read_csv(artifact_root / "datasets" / "modeling_dataset.csv")
    test_dataset = dataset[dataset["split"] == "test"].copy()
    event_evidence = pd.read_csv(artifact_root / "datasets" / "test_event_evidence.csv")
    metrics = json.loads((artifact_root / "reports" / "baseline_metrics.json").read_text())
    predictions_by_target = _load_best_predictions(metrics)
    four_block_predictions = predictions_by_target[FOUR_BLOCK_TARGET]

    cases = _select_cases(four_block_predictions)
    case_summary, timelines, score_summary = _build_case_tables(
        cases=cases,
        test_dataset=test_dataset,
        event_evidence=event_evidence,
        predictions_by_target=predictions_by_target,
    )

    network_summaries: dict[str, dict[str, Any]] = {}
    if args.network_api_base and not args.skip_network:
        for case in cases:
            network_summaries[case.slug] = _fetch_network_summary(
                case=case,
                test_dataset=test_dataset,
                report_root=report_root,
                api_base=args.network_api_base.rstrip("/"),
                timeout_seconds=args.network_timeout_seconds,
            )

    case_summary.to_csv(report_root / "case_summary.csv", index=False)
    timelines.to_csv(report_root / "case_timelines.csv", index=False)
    score_summary.to_csv(report_root / "case_model_scores.csv", index=False)

    report = _render_report(
        cases=cases,
        case_summary=case_summary,
        timelines=timelines,
        score_summary=score_summary,
        event_evidence=event_evidence,
        network_summaries=network_summaries,
    )
    report_path = (
        MODELING_ROOT
        / "experiments"
        / args.run_id
        / "investigations"
        / "pre_iteration_02_signal_review.md"
    )
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(report, encoding="utf-8")
    print(f"wrote {report_path}")
    print(f"wrote {report_root}")


def _load_best_predictions(metrics: dict[str, Any]) -> dict[str, pd.DataFrame]:
    predictions: dict[str, pd.DataFrame] = {}
    for target, entry in metrics["best_classification"].items():
        path = Path(entry["test_prediction_path"])
        if path.is_file():
            predictions[target] = pd.read_csv(path)
    if FOUR_BLOCK_TARGET not in predictions:
        raise RuntimeError(f"missing best prediction file for {FOUR_BLOCK_TARGET}")
    return predictions


def _select_cases(predictions: pd.DataFrame) -> list[Case]:
    cases: list[Case] = []
    used_pools: set[str] = set()

    def add_from_row(
        slug: str,
        title: str,
        category: str,
        reason: str,
        row: pd.Series,
    ) -> None:
        pool = str(row["pool_address"]).lower()
        used_pools.add(pool)
        cases.append(
            Case(
                slug=slug,
                title=title,
                category=category,
                selection_reason=reason,
                token_address=str(row["token_address"]).lower(),
                pool_address=pool,
                selected_block=int(row["block_number"]),
                selected_score=_float_or_none(row.get("y_score")),
                selected_time_to_scam=_float_or_none(row.get("time_to_scam_chain_block_delta")),
            )
        )

    uliq_rows = predictions[
        (predictions["token_address"].str.lower() == DEFAULT_ULIQ_TOKEN)
        & (predictions["pool_address"].str.lower() == DEFAULT_ULIQ_POOL)
    ].copy()
    if not uliq_rows.empty:
        uliq_rows["_priority"] = uliq_rows["y_true"].fillna(0) * 10 - uliq_rows[
            "time_to_scam_chain_block_delta"
        ].fillna(10_000)
        add_from_row(
            "uliq_holdout_backdoor_miss",
            "ULIQ holdout backdoor miss",
            "holdout_false_negative",
            "Known fast ULIQ token-control backdoor; included to see what was visible before the scam.",
            uliq_rows.sort_values(["_priority", "block_number"], ascending=[False, False]).iloc[0],
        )

    selectors = [
        (
            "direct_lp_true_positive",
            "Direct LP true positive",
            "true_positive",
            "Highest 4-block true positive without a pair-balance backdoor flag.",
            (predictions["y_true"] == 1)
            & (~predictions["pair_balance_backdoor_signal_seen_as_of"].fillna(False))
            & (~predictions["pool_address"].str.lower().isin(used_pools)),
            False,
        ),
        (
            "backdoor_true_positive",
            "Backdoor true positive",
            "true_positive_backdoor",
            "Highest 4-block true positive with pair-balance backdoor signal already visible.",
            (predictions["y_true"] == 1)
            & (predictions["pair_balance_backdoor_signal_seen_as_of"].fillna(False))
            & (~predictions["pool_address"].str.lower().isin(used_pools)),
            False,
        ),
        (
            "backdoor_false_negative",
            "Backdoor false negative",
            "false_negative_backdoor",
            "Lowest-scored 4-block positive row where pair-balance backdoor signal was already visible.",
            (predictions["y_true"] == 1)
            & (predictions["pair_balance_backdoor_signal_seen_as_of"].fillna(False))
            & (~predictions["pool_address"].str.lower().isin(used_pools)),
            True,
        ),
        (
            "near_horizon_false_positive",
            "Near-horizon false positive",
            "near_horizon_false_positive",
            "Highest-scored 4-block false positive that did scam soon after the 4-block window.",
            (predictions["y_true"] == 0)
            & predictions["time_to_scam_chain_block_delta"].notna()
            & (predictions["time_to_scam_chain_block_delta"] > 4)
            & (predictions["time_to_scam_chain_block_delta"] <= 20)
            & (~predictions["pool_address"].str.lower().isin(used_pools)),
            False,
        ),
        (
            "unlabeled_backdoor_false_positive",
            "Unlabeled backdoor false positive",
            "unlabeled_false_positive_backdoor",
            "Highest-scored 4-block false positive with backdoor signal and no scam label in the test run.",
            (predictions["y_true"] == 0)
            & predictions["scam_block"].isna()
            & (predictions["pair_balance_backdoor_signal_seen_as_of"].fillna(False))
            & (~predictions["pool_address"].str.lower().isin(used_pools)),
            False,
        ),
    ]

    for slug, title, category, reason, mask, ascending in selectors:
        pool_mask = ~predictions["pool_address"].str.lower().isin(used_pools)
        rows = predictions[mask & pool_mask].sort_values("y_score", ascending=ascending)
        if not rows.empty:
            add_from_row(slug, title, category, reason, rows.iloc[0])

    return cases


def _build_case_tables(
    *,
    cases: list[Case],
    test_dataset: pd.DataFrame,
    event_evidence: pd.DataFrame,
    predictions_by_target: dict[str, pd.DataFrame],
) -> tuple[pd.DataFrame, pd.DataFrame, pd.DataFrame]:
    summary_rows = []
    timeline_rows = []
    score_rows = []
    for case in cases:
        pool_rows = _pool_rows(test_dataset, case)
        events = _pool_events(event_evidence, case)
        mechanisms = sorted(
            {
                str(value)
                for value in events.loc[events["event_kind"] == "scam", "mechanism"].dropna()
                if str(value) and str(value) != "nan"
            }
        )
        mempool_seen = events["mempool_first_seen_ms"].notna().sum()

        first_observation_block = _min_value(pool_rows, "block_number")
        last_observation_block = _max_value(pool_rows, "block_number")
        scam_block = _min_value(pool_rows[pool_rows["scam_block"].notna()], "scam_block")
        first_lp_approval_block = _event_min(events, "lp_approval")
        first_token_control_block = _event_min(events, "token_control_signal")
        first_backdoor_block = _first_true_block(pool_rows, "pair_balance_backdoor_signal_seen_as_of")
        first_holder_burn_block = _first_true_block(
            pool_rows, "control_transfer_from_holder_to_burn_seen_as_of"
        )
        first_without_log_block = _first_true_block(
            pool_rows, "control_transfer_from_without_transfer_log_seen_as_of"
        )
        first_pair_to_control_block = _first_true_block(pool_rows, "pair_token_to_control_seen_as_of")
        first_lp_removable_block = _first_threshold_block(pool_rows, "lp_removable_pct_as_of", 90.0)
        first_creator_lp_90_block = _first_threshold_block(
            pool_rows, "creator_lp_balance_pct_as_of", 90.0
        )
        selected_row = _selected_or_last_row(pool_rows, case.selected_block)

        summary_rows.append(
            {
                "case": case.slug,
                "title": case.title,
                "category": case.category,
                "selection_reason": case.selection_reason,
                "token_address": case.token_address,
                "pool_address": case.pool_address,
                "selected_block": case.selected_block,
                "selected_4_block_score": case.selected_score,
                "selected_time_to_scam_chain_block_delta": case.selected_time_to_scam,
                "first_observation_block": first_observation_block,
                "last_observation_block": last_observation_block,
                "scam_block": scam_block,
                "scam_mechanisms": ", ".join(mechanisms),
                "first_lp_approval_block": first_lp_approval_block,
                "first_token_control_event_block": first_token_control_block,
                "first_lp_removable_90_block": first_lp_removable_block,
                "first_creator_lp_balance_90_block": first_creator_lp_90_block,
                "first_pair_balance_backdoor_signal_block": first_backdoor_block,
                "first_holder_to_burn_signal_block": first_holder_burn_block,
                "first_without_transfer_log_signal_block": first_without_log_block,
                "first_pair_token_to_control_signal_block": first_pair_to_control_block,
                "mempool_evidence_rows": int(mempool_seen),
                "event_count": int(len(events)),
                "observation_count": int(len(pool_rows)),
                "selected_lp_removable_pct": selected_row.get("lp_removable_pct_as_of"),
                "selected_creator_lp_balance_pct": selected_row.get("creator_lp_balance_pct_as_of"),
                "selected_pair_balance_backdoor_signal": selected_row.get(
                    "pair_balance_backdoor_signal_seen_as_of"
                ),
                "selected_holder_to_burn_signal": selected_row.get(
                    "control_transfer_from_holder_to_burn_seen_as_of"
                ),
                "selected_without_transfer_log_signal": selected_row.get(
                    "control_transfer_from_without_transfer_log_seen_as_of"
                ),
                "selected_pair_token_to_control_signal": selected_row.get(
                    "pair_token_to_control_seen_as_of"
                ),
            }
        )

        for _, row in _interesting_timeline_rows(pool_rows, case).iterrows():
            item = {"case": case.slug}
            for column in TIMELINE_COLUMNS:
                if column in row.index:
                    item[column] = row[column]
            timeline_rows.append(item)

        for target, predictions in predictions_by_target.items():
            rows = predictions[
                (predictions["token_address"].str.lower() == case.token_address)
                & (predictions["pool_address"].str.lower() == case.pool_address)
            ].copy()
            if rows.empty:
                continue
            score_rows.append(
                {
                    "case": case.slug,
                    "target": target,
                    "rows": int(len(rows)),
                    "positive_rows": int(rows["y_true"].fillna(0).sum()),
                    "max_score": float(rows["y_score"].max()),
                    "max_score_block": int(rows.loc[rows["y_score"].idxmax(), "block_number"]),
                    "max_positive_score": _max_positive_score(rows),
                    "first_positive_block": _first_positive_block(rows),
                    "selected_block_score": _score_at_block(rows, case.selected_block),
                }
            )

    return pd.DataFrame(summary_rows), pd.DataFrame(timeline_rows), pd.DataFrame(score_rows)


def _fetch_network_summary(
    *,
    case: Case,
    test_dataset: pd.DataFrame,
    report_root: Path,
    api_base: str,
    timeout_seconds: int,
) -> dict[str, Any]:
    pool_rows = _pool_rows(test_dataset, case)
    anchor_block = int(_min_value(pool_rows[pool_rows["scam_block"].notna()], "scam_block") or case.selected_block)
    start_block = max(0, min(case.selected_block, anchor_block) - 80)
    end_block = anchor_block + 20
    payload = {
        "token": case.token_address,
        "start_block": start_block,
        "end_block": end_block,
        "history_limit": 1000,
        "max_token_blocks": 128,
        "max_seeds": 12,
        "max_blocks_per_address": 32,
        "lookback_blocks": 40,
        "lookahead_blocks": 30,
        "include_timeline": True,
    }
    try:
        created = _http_json(api_base, method="POST", payload=payload)
        job_id = created["id"]
        deadline = time.monotonic() + timeout_seconds
        job = created
        while time.monotonic() < deadline:
            job = _http_json(f"{api_base}/{job_id}")
            status = (job.get("progress") or {}).get("status")
            if status in {"complete", "failed", "canceled"}:
                break
            time.sleep(1)
        (report_root / f"{case.slug}_network.json").write_text(
            json.dumps(job, indent=2, sort_keys=True),
            encoding="utf-8",
        )
        return _summarize_network_job(job)
    except (urllib.error.URLError, TimeoutError, KeyError, ValueError) as error:
        return {"error": str(error)}


def _http_json(url: str, *, method: str = "GET", payload: dict[str, Any] | None = None) -> dict[str, Any]:
    data = None
    headers = {}
    if payload is not None:
        data = json.dumps(payload).encode("utf-8")
        headers["content-type"] = "application/json"
    request = urllib.request.Request(url, data=data, headers=headers, method=method)
    with urllib.request.urlopen(request, timeout=30) as response:
        return json.loads(response.read().decode("utf-8"))


def _summarize_network_job(job: dict[str, Any]) -> dict[str, Any]:
    progress = job.get("progress") or {}
    result = job.get("result") or {}
    token = result.get("token") or {}
    flow = result.get("flow_context") or {}
    nodes = flow.get("nodes") or []
    edges = flow.get("edges") or []
    labelled_nodes = [
        {
            "address": node.get("address"),
            "role": node.get("role"),
            "labels": node.get("labels") or [],
            "seed": node.get("seed"),
        }
        for node in nodes
        if node.get("labels")
    ]
    labelled_nodes.sort(
        key=lambda node: (
            "creator" not in node["labels"],
            "owner" not in node["labels"],
            "wallet" not in node["labels"],
            str(node.get("address") or ""),
        )
    )
    labelled_nodes = labelled_nodes[:10]
    top_edges = [
        {
            "source": edge.get("source"),
            "target": edge.get("target"),
            "kind": edge.get("kind"),
            "weight": edge.get("weight"),
            "amount": edge.get("total_scaled_amount"),
        }
        for edge in edges[:8]
    ]
    graph = result.get("graph") or {}
    return {
        "status": progress.get("status"),
        "stage": progress.get("stage"),
        "token_name": token.get("name"),
        "token_symbol": token.get("symbol"),
        "creator_address": token.get("creator_address"),
        "graph_node_count": graph.get("node_count"),
        "graph_edge_count": graph.get("edge_count"),
        "address_count": graph.get("address_count"),
        "seed_count": (flow.get("seed_count") if isinstance(flow, dict) else None),
        "labelled_nodes": labelled_nodes,
        "top_edges": top_edges,
        "error": job.get("error"),
    }


def _render_report(
    *,
    cases: list[Case],
    case_summary: pd.DataFrame,
    timelines: pd.DataFrame,
    score_summary: pd.DataFrame,
    event_evidence: pd.DataFrame,
    network_summaries: dict[str, dict[str, Any]],
) -> str:
    lines = [
        "# Pre-Iteration 02 Signal Review",
        "",
        "Status: investigation complete; do not train iteration 02 until these findings are folded into the feature plan.",
        "",
        "Objective: find signals that could warn about an upcoming scam before the scam/removal event, using only evidence visible at or before each observation block.",
        "",
        "## Case Set",
        "",
        _markdown_table(
            [
                [
                    "Case",
                    "Category",
                    "Selected block",
                    "4-block score",
                    "Scam block",
                    "Mechanism",
                    "Mempool rows",
                ]
            ]
            + [
                [
                    row["case"],
                    row["category"],
                    _fmt_int(row["selected_block"]),
                    _fmt_pct(row["selected_4_block_score"]),
                    _fmt_int(row["scam_block"]),
                    row["scam_mechanisms"] or "-",
                    _fmt_int(row["mempool_evidence_rows"]),
                ]
                for _, row in case_summary.iterrows()
            ]
        ),
        "",
        "## Cross-Case Findings",
        "",
    ]
    lines.extend(_cross_case_findings(case_summary, score_summary, network_summaries))
    lines.extend(["", "## Cases", ""])
    for case in cases:
        summary = case_summary[case_summary["case"] == case.slug].iloc[0]
        case_scores = score_summary[score_summary["case"] == case.slug].sort_values("target")
        case_timeline = timelines[timelines["case"] == case.slug].sort_values("block_number")
        case_events = _pool_events(event_evidence, case)
        network = network_summaries.get(case.slug, {})

        lines.extend(
            [
                f"### {case.title}",
                "",
                f"- Case: `{case.slug}`",
                f"- Token: `{case.token_address}`",
                f"- Pool: `{case.pool_address}`",
                f"- Why selected: {case.selection_reason}",
                f"- Scam mechanism: `{summary['scam_mechanisms'] or 'none in test run'}`",
                f"- Selected row: block `{_fmt_int(summary['selected_block'])}`, score `{_fmt_pct(summary['selected_4_block_score'])}`, time-to-scam `{_fmt_number(summary['selected_time_to_scam_chain_block_delta'])}` chain blocks.",
                "",
                "Model score summary:",
                "",
                _markdown_table(
                    [["Target", "Rows", "Positive rows", "Max score", "Max score block", "First positive block"]]
                    + [
                        [
                            row["target"],
                            _fmt_int(row["rows"]),
                            _fmt_int(row["positive_rows"]),
                            _fmt_pct(row["max_score"]),
                            _fmt_int(row["max_score_block"]),
                            _fmt_int(row["first_positive_block"]),
                        ]
                        for _, row in case_scores.iterrows()
                    ]
                ),
                "",
                "Event sequence:",
                "",
                _event_table(case_events),
                "",
                "Signal blocks:",
                "",
                _markdown_table(
                    [
                        ["Signal", "First block"],
                        ["LP approval event", _fmt_int(summary["first_lp_approval_block"])],
                        ["LP removable >= 90%", _fmt_int(summary["first_lp_removable_90_block"])],
                        [
                            "Creator LP balance >= 90%",
                            _fmt_int(summary["first_creator_lp_balance_90_block"]),
                        ],
                        [
                            "Token-control event",
                            _fmt_int(summary["first_token_control_event_block"]),
                        ],
                        [
                            "Pair-balance backdoor flag",
                            _fmt_int(summary["first_pair_balance_backdoor_signal_block"]),
                        ],
                        [
                            "Holder-to-burn transferFrom flag",
                            _fmt_int(summary["first_holder_to_burn_signal_block"]),
                        ],
                        [
                            "transferFrom without Transfer log flag",
                            _fmt_int(summary["first_without_transfer_log_signal_block"]),
                        ],
                        [
                            "Pair-token-to-control flag",
                            _fmt_int(summary["first_pair_token_to_control_signal_block"]),
                        ],
                    ]
                ),
                "",
                "Observation timeline:",
                "",
                _timeline_table(case_timeline),
                "",
                "Network snapshot:",
                "",
            ]
        )
        lines.extend(_network_lines(network))
        lines.extend(["", "Feature lesson:", "", *_feature_lesson(summary), ""])

    return "\n".join(lines).rstrip() + "\n"


def _cross_case_findings(
    case_summary: pd.DataFrame,
    score_summary: pd.DataFrame,
    network_summaries: dict[str, dict[str, Any]],
) -> list[str]:
    lines = []
    mempool_total = int(case_summary["mempool_evidence_rows"].fillna(0).sum())
    backdoor_cases = case_summary[
        case_summary["first_pair_balance_backdoor_signal_block"].notna()
        | case_summary["first_holder_to_burn_signal_block"].notna()
    ]
    lp_cases = case_summary[case_summary["first_lp_removable_90_block"].notna()]
    lines.append(
        f"- Mempool coverage is not useful in this sample yet: `{mempool_total}` selected evidence rows had `mempool_first_seen_ms`."
    )
    lines.append(
        f"- `{len(lp_cases)}` of `{len(case_summary)}` selected cases reached `lp_removable_pct_as_of >= 90`; this is the common direct-removal signal the current model already understands."
    )
    lines.append(
        f"- `{len(backdoor_cases)}` of `{len(case_summary)}` selected cases showed token-control/backdoor flags before or at the selected block; several were still scored low, so this needs a separate branch instead of relying on the generic model."
    )
    creator_owner_cases = []
    for case_slug, network in network_summaries.items():
        for node in network.get("labelled_nodes") or []:
            labels = set(node.get("labels") or [])
            if {"creator", "owner"}.issubset(labels):
                creator_owner_cases.append(case_slug)
                break
    if creator_owner_cases:
        joined = "`, `".join(sorted(creator_owner_cases))
        lines.append(
            f"- Network snapshots surfaced creator+owner labelled control wallets for `{joined}`; those should become investigation features once we can make them as-of safe."
        )
    four_scores = score_summary[score_summary["target"] == FOUR_BLOCK_TARGET]
    if not four_scores.empty:
        low_positive = four_scores[
            four_scores["positive_rows"].fillna(0) > 0
        ].sort_values("max_positive_score").head(1)
        if not low_positive.empty:
            row = low_positive.iloc[0]
            lines.append(
                f"- The weakest positive case in this review is `{row['case']}` with max positive 4-block score `{_fmt_pct(row['max_positive_score'])}`; that is the main miss to explain before iteration 02."
            )
    lines.append(
        "- Next iteration should model two warning families separately: direct LP-removal readiness and token-control/pair-balance backdoor behavior."
    )
    return lines


def _feature_lesson(summary: pd.Series) -> list[str]:
    lessons = []
    scam_block = _float_or_none(summary.get("scam_block"))
    selected_block = _float_or_none(summary.get("selected_block"))
    backdoor_block = _float_or_none(summary.get("first_pair_balance_backdoor_signal_block"))
    lp_block = _float_or_none(summary.get("first_lp_removable_90_block"))
    if backdoor_block is not None and scam_block is not None:
        lessons.append(
            f"- Backdoor timing should be explicit: first backdoor flag was `{_fmt_int(backdoor_block)}`, `{_fmt_int(scam_block - backdoor_block)}` chain blocks before scam."
        )
    if lp_block is not None and scam_block is not None:
        lessons.append(
            f"- LP removability timing should be explicit: first >=90% removable block was `{_fmt_int(lp_block)}`, `{_fmt_int(scam_block - lp_block)}` chain blocks before scam."
        )
    if backdoor_block is not None and selected_block is not None:
        lessons.append(
            f"- Add `blocks_since_first_backdoor_signal`: it was `{_fmt_int(selected_block - backdoor_block)}` at the selected row."
        )
    if summary.get("first_holder_to_burn_signal_block") == summary.get(
        "first_pair_balance_backdoor_signal_block"
    ):
        lessons.append(
            "- The holder-to-burn `transferFrom` and pair-balance backdoor flags co-arrived; add an interaction feature instead of leaving them as independent booleans."
        )
    if not lessons:
        lessons.append("- No obvious pre-scam feature is represented yet; inspect raw transaction/event mechanics before treating this as label noise.")
    return lessons


def _event_table(events: pd.DataFrame) -> str:
    if events.empty:
        return "_No event evidence rows._"
    rows = [["Block", "Kind", "Mechanism", "Tx", "Mempool first seen"]]
    for _, row in events.sort_values(["block_number", "event_kind"]).iterrows():
        rows.append(
            [
                _fmt_int(row.get("block_number")),
                _text_or_dash(row.get("event_kind")),
                _text_or_dash(row.get("mechanism")),
                _short_hash(row.get("tx_hash")),
                _fmt_int(row.get("mempool_first_seen_ms")),
            ]
        )
    return _markdown_table(rows)


def _timeline_table(rows: pd.DataFrame) -> str:
    if rows.empty:
        return "_No timeline rows._"
    table = [[
        "Block",
        "dt scam",
        "tx",
        "LP rem %",
        "creator LP %",
        "direct rm",
        "backdoor",
        "holder burn",
        "no log",
        "pair->control",
        "price/init",
    ]]
    for _, row in rows.iterrows():
        table.append(
            [
                _fmt_int(row.get("block_number")),
                _fmt_number(row.get("time_to_scam_chain_block_delta")),
                _fmt_int(row.get("tx_count")),
                _fmt_pct(row.get("lp_removable_pct_as_of")),
                _fmt_pct(row.get("creator_lp_balance_pct_as_of")),
                _fmt_bool(row.get("direct_lp_removal_in_block")),
                _fmt_bool(row.get("pair_balance_backdoor_signal_seen_as_of")),
                _fmt_bool(row.get("control_transfer_from_holder_to_burn_seen_as_of")),
                _fmt_bool(row.get("control_transfer_from_without_transfer_log_seen_as_of")),
                _fmt_bool(row.get("pair_token_to_control_seen_as_of")),
                _fmt_number(row.get("price_to_initial_ratio")),
            ]
        )
    return _markdown_table(table)


def _network_lines(network: dict[str, Any]) -> list[str]:
    if not network:
        return ["- Network API was not requested for this run."]
    if network.get("error"):
        return [f"- Network API error: `{network['error']}`"]
    lines = [
        f"- Status: `{network.get('status')}`",
        f"- Token: `{network.get('token_symbol') or '-'}` / `{network.get('token_name') or '-'}`",
        f"- Creator address from metadata: `{network.get('creator_address') or '-'}`",
        f"- Graph: `{network.get('graph_node_count')}` nodes, `{network.get('graph_edge_count')}` edges, `{network.get('address_count')}` addresses.",
    ]
    labelled = network.get("labelled_nodes") or []
    if labelled:
        label_text = "; ".join(
            f"`{node.get('address')}` ({','.join(node.get('labels') or [])})"
            for node in labelled[:5]
        )
        lines.append(f"- Labelled network nodes: {label_text}.")
    top_edges = network.get("top_edges") or []
    if top_edges:
        edge_text = "; ".join(
            f"`{_short_hash(edge.get('source'))}->{_short_hash(edge.get('target'))}` {edge.get('kind')} w={edge.get('weight')}"
            for edge in top_edges[:5]
        )
        lines.append(f"- Top context edges: {edge_text}.")
    return lines


def _interesting_timeline_rows(pool_rows: pd.DataFrame, case: Case) -> pd.DataFrame:
    if pool_rows.empty:
        return pool_rows
    masks = [
        pool_rows["block_number"] == pool_rows["block_number"].min(),
        pool_rows["block_number"] == case.selected_block,
        pool_rows["block_number"] == pool_rows["block_number"].max(),
    ]
    scam_blocks = pool_rows["scam_block"].dropna()
    if not scam_blocks.empty:
        scam_block = int(scam_blocks.min())
        masks.append(pool_rows["block_number"].between(scam_block - 10, scam_block))
    for column in SIGNAL_BOOL_COLUMNS:
        if column in pool_rows.columns:
            masks.append(pool_rows[column].fillna(False).astype(bool))
    if "lp_approval_count_in_block" in pool_rows.columns:
        masks.append(pool_rows["lp_approval_count_in_block"].fillna(0) > 0)
    if "lp_removable_pct_as_of" in pool_rows.columns:
        masks.append(pool_rows["lp_removable_pct_as_of"].fillna(0) >= 90)
    mask = masks[0]
    for next_mask in masks[1:]:
        mask = mask | next_mask
    selected = pool_rows[mask].sort_values("block_number")
    if len(selected) > 18:
        keep_blocks = set(selected.head(3)["block_number"]) | set(selected.tail(12)["block_number"])
        keep_blocks.add(case.selected_block)
        selected = selected[selected["block_number"].isin(keep_blocks)]
    return selected.drop_duplicates(subset=["block_number"])


def _pool_rows(dataset: pd.DataFrame, case: Case) -> pd.DataFrame:
    return dataset[
        (dataset["token_address"].str.lower() == case.token_address)
        & (dataset["pool_address"].str.lower() == case.pool_address)
    ].sort_values("block_number")


def _pool_events(event_evidence: pd.DataFrame, case: Case) -> pd.DataFrame:
    return event_evidence[
        (event_evidence["token_address"].str.lower() == case.token_address)
        & (event_evidence["pool_address"].str.lower() == case.pool_address)
    ].sort_values(["block_number", "event_kind"])


def _selected_or_last_row(rows: pd.DataFrame, selected_block: int) -> pd.Series:
    if rows.empty:
        return pd.Series(dtype=object)
    exact = rows[rows["block_number"] == selected_block]
    if not exact.empty:
        return exact.iloc[-1]
    return rows.iloc[-1]


def _event_min(events: pd.DataFrame, event_kind: str) -> int | None:
    rows = events[events["event_kind"] == event_kind]
    return _min_value(rows, "block_number")


def _first_true_block(rows: pd.DataFrame, column: str) -> int | None:
    if column not in rows.columns:
        return None
    selected = rows[rows[column].fillna(False).astype(bool)]
    return _min_value(selected, "block_number")


def _first_threshold_block(rows: pd.DataFrame, column: str, threshold: float) -> int | None:
    if column not in rows.columns:
        return None
    selected = rows[rows[column].fillna(0) >= threshold]
    return _min_value(selected, "block_number")


def _min_value(rows: pd.DataFrame, column: str) -> int | None:
    if rows.empty or column not in rows.columns:
        return None
    values = rows[column].dropna()
    if values.empty:
        return None
    return int(values.min())


def _max_value(rows: pd.DataFrame, column: str) -> int | None:
    if rows.empty or column not in rows.columns:
        return None
    values = rows[column].dropna()
    if values.empty:
        return None
    return int(values.max())


def _max_positive_score(rows: pd.DataFrame) -> float | None:
    positives = rows[rows["y_true"] == 1]
    if positives.empty:
        return None
    return float(positives["y_score"].max())


def _first_positive_block(rows: pd.DataFrame) -> int | None:
    positives = rows[rows["y_true"] == 1]
    if positives.empty:
        return None
    return int(positives["block_number"].min())


def _score_at_block(rows: pd.DataFrame, block_number: int) -> float | None:
    exact = rows[rows["block_number"] == block_number]
    if exact.empty:
        return None
    return float(exact.iloc[-1]["y_score"])


def _markdown_table(rows: list[list[Any]]) -> str:
    if not rows:
        return ""
    escaped = [[_escape_md(value) for value in row] for row in rows]
    widths = [max(len(row[index]) for row in escaped) for index in range(len(escaped[0]))]
    output = []
    for row_index, row in enumerate(escaped):
        output.append("| " + " | ".join(value.ljust(widths[index]) for index, value in enumerate(row)) + " |")
        if row_index == 0:
            output.append("| " + " | ".join("-" * widths[index] for index in range(len(row))) + " |")
    return "\n".join(output)


def _escape_md(value: Any) -> str:
    if value is None:
        return "-"
    if isinstance(value, float) and math.isnan(value):
        return "-"
    text = str(value)
    return text.replace("|", "\\|").replace("\n", " ")


def _fmt_int(value: Any) -> str:
    number = _float_or_none(value)
    if number is None:
        return "-"
    return str(int(number))


def _fmt_number(value: Any) -> str:
    number = _float_or_none(value)
    if number is None:
        return "-"
    if abs(number - round(number)) < 1e-9:
        return str(int(round(number)))
    return f"{number:.4g}"


def _fmt_pct(value: Any) -> str:
    number = _float_or_none(value)
    if number is None:
        return "-"
    return f"{number:.2%}" if number <= 1.0 else f"{number:.2f}%"


def _fmt_bool(value: Any) -> str:
    if value is None:
        return "-"
    if isinstance(value, float) and math.isnan(value):
        return "-"
    if isinstance(value, str):
        return "Y" if value.lower() == "true" else "-"
    return "Y" if bool(value) else "-"


def _text_or_dash(value: Any) -> str:
    if value is None:
        return "-"
    if isinstance(value, float) and math.isnan(value):
        return "-"
    text = str(value)
    if not text or text == "nan":
        return "-"
    return text


def _float_or_none(value: Any) -> float | None:
    if value is None:
        return None
    try:
        number = float(value)
    except (TypeError, ValueError):
        return None
    if math.isnan(number):
        return None
    return number


def _short_hash(value: Any) -> str:
    if value is None:
        return "-"
    if isinstance(value, float) and math.isnan(value):
        return "-"
    text = str(value)
    if not text or text == "nan":
        return "-"
    if re.fullmatch(r"0x[a-fA-F0-9]{16,}", text):
        return f"{text[:10]}...{text[-6:]}"
    return text


if __name__ == "__main__":
    main()
