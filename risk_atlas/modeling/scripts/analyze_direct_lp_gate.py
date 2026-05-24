#!/usr/bin/env python3
"""Analyze the direct LP-removal danger-state gate."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

import numpy as np
import pandas as pd
import joblib
from sklearn.inspection import permutation_importance
from sklearn.metrics import average_precision_score, roc_auc_score

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from risk_atlas_modeling.datasets.features import feature_matrix


DEFAULT_RUN_ID = "backdoor_horizon_oot_v1"
MECHANISM = "direct_lp_liquidity_removal"
POOL_KEYS = ["source_run_id", "token_address", "pool_address"]
HORIZONS = (1, 2, 3, 4, 10, 50, 100)
GATE_COLUMNS = [
    "lp_removable_pct_as_of",
    "creator_lp_removable_pct_as_of",
    "creator_lp_router_removable_pct_as_of",
]
GATE_MIN_PCT = 30


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run-id", default=DEFAULT_RUN_ID)
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    artifact_root = ROOT / "artifacts" / args.run_id
    frame = pd.read_csv(artifact_root / "datasets" / "modeling_dataset.csv")
    frame = normalize_frame(frame)
    frame = add_direct_lp_labels(frame, artifact_root)
    frame = add_gate_state(frame)

    metrics = {
        "run_id": args.run_id,
        "mechanism": MECHANISM,
        "gate_definition": " OR ".join(f"{column} > {GATE_MIN_PCT}" for column in GATE_COLUMNS),
        "gate_min_pct": GATE_MIN_PCT,
        "split_summary": split_summary(frame),
        "rule_horizon_metrics": rule_horizon_metrics(frame, artifact_root),
        "within_gate_model_metrics": within_gate_model_metrics(frame, artifact_root),
        "within_gate_feature_signal": within_gate_feature_signal(frame, artifact_root),
    }

    output_root = artifact_root / "gate" / "direct_lp_removal"
    output_root.mkdir(parents=True, exist_ok=True)
    metrics_path = output_root / "metrics.json"
    summary_path = output_root / "summary.md"
    metrics_path.write_text(json.dumps(metrics, indent=2, sort_keys=True), encoding="utf-8")
    summary_path.write_text(summary_markdown(metrics), encoding="utf-8")
    print(f"wrote {metrics_path}")
    print(f"wrote {summary_path}")


def normalize_frame(frame: pd.DataFrame) -> pd.DataFrame:
    result = frame.copy()
    result["token_address"] = result["token_address"].astype(str).str.lower()
    result["pool_address"] = result["pool_address"].astype(str).str.lower()
    result["source_run_id"] = result["source_run_id"].astype(str)
    result["block_number"] = pd.to_numeric(result["block_number"], errors="coerce")
    return result


def add_direct_lp_labels(frame: pd.DataFrame, artifact_root: Path) -> pd.DataFrame:
    events = direct_lp_events(artifact_root)
    labels = {
        (row.run_id, row.token_address, row.pool_address): int(row.direct_lp_scam_block)
        for row in events.itertuples(index=False)
    }
    result = frame.copy()
    result["direct_lp_scam_block"] = [
        labels.get((row.source_run_id, row.token_address, row.pool_address), np.nan)
        for row in result.itertuples(index=False)
    ]
    result["has_direct_lp_scam"] = result["direct_lp_scam_block"].notna()
    result["time_to_direct_lp_scam_chain_block_delta"] = (
        result["direct_lp_scam_block"] - result["block_number"]
    )
    result.loc[
        ~result["has_direct_lp_scam"],
        "time_to_direct_lp_scam_chain_block_delta",
    ] = np.nan
    return result


def direct_lp_events(artifact_root: Path) -> pd.DataFrame:
    frames = []
    for name in ("train_event_evidence.csv", "test_event_evidence.csv"):
        events = pd.read_csv(artifact_root / "datasets" / name)
        events["token_address"] = events["token_address"].astype(str).str.lower()
        events["pool_address"] = events["pool_address"].astype(str).str.lower()
        frames.append(
            events[
                (events["event_kind"] == "scam")
                & (events["mechanism"].astype(str) == MECHANISM)
            ].copy()
        )
    events = pd.concat(frames, ignore_index=True)
    if events.empty:
        return pd.DataFrame(columns=["run_id", "token_address", "pool_address", "direct_lp_scam_block"])
    return (
        events.groupby(["run_id", "token_address", "pool_address"], as_index=False)["block_number"]
        .min()
        .rename(columns={"block_number": "direct_lp_scam_block"})
    )


def add_gate_state(frame: pd.DataFrame) -> pd.DataFrame:
    result = frame.copy()
    gate_values = pd.DataFrame(index=result.index)
    for column in GATE_COLUMNS:
        if column in result.columns:
            gate_values[column] = pd.to_numeric(result[column], errors="coerce").fillna(0)
        else:
            gate_values[column] = 0
    result["direct_lp_danger_state"] = gate_values.gt(GATE_MIN_PCT).any(axis=1)
    return result


def split_summary(frame: pd.DataFrame) -> dict[str, dict]:
    summaries = {}
    for split, rows in frame.groupby("split", sort=True):
        direct = rows[rows["has_direct_lp_scam"]]
        direct_pool_count = pool_count(direct)
        danger = rows[rows["direct_lp_danger_state"]]
        direct_danger = direct[direct["direct_lp_danger_state"]]
        first_leads = first_warning_leads(direct)
        no_warning = max(0, direct_pool_count - int(first_leads["pool_count"]))
        summaries[str(split)] = {
            "rows": int(len(rows)),
            "pools": pool_count(rows),
            "direct_lp_scam_pools": direct_pool_count,
            "danger_rows": int(len(danger)),
            "danger_pools": pool_count(danger),
            "direct_lp_scam_pools_with_warning": int(first_leads["pool_count"]),
            "direct_lp_scam_pools_without_warning": no_warning,
            "direct_lp_warning_pool_coverage": ratio(first_leads["pool_count"], direct_pool_count),
            "direct_lp_warning_row_count": int(len(direct_danger)),
            "non_direct_danger_pools": pool_count(rows[(~rows["has_direct_lp_scam"]) & rows["direct_lp_danger_state"]]),
            "first_warning_lead_chain_block_delta": first_leads["stats"],
        }
    return summaries


def first_warning_leads(rows: pd.DataFrame) -> dict:
    if rows.empty:
        return {"pool_count": 0, "stats": stats([])}
    signal_rows = rows[rows["direct_lp_danger_state"]].copy()
    if signal_rows.empty:
        return {"pool_count": 0, "stats": stats([])}
    first = (
        signal_rows.groupby(POOL_KEYS, as_index=False)
        .agg(
            first_warning_block=("block_number", "min"),
            direct_lp_scam_block=("direct_lp_scam_block", "first"),
        )
    )
    leads = first["direct_lp_scam_block"] - first["first_warning_block"]
    leads = leads[leads.notna() & (leads >= 1)]
    return {"pool_count": int(len(leads)), "stats": stats(leads)}


def rule_horizon_metrics(frame: pd.DataFrame, artifact_root: Path) -> dict[str, dict]:
    end_blocks = source_run_end_blocks(artifact_root)
    result = {}
    for horizon in HORIZONS:
        known = frame["has_direct_lp_scam"] | (
            frame["block_number"] <= frame["source_run_id"].map(end_blocks) - horizon
        )
        positive = frame["time_to_direct_lp_scam_chain_block_delta"].between(
            1,
            horizon,
            inclusive="both",
        )
        target = positive.astype(int)
        rows = frame[known].copy()
        rows["_target"] = target[known].astype(int)
        horizon_key = f"within_{horizon}_chain_block_delta"
        result[horizon_key] = {}
        for split, split_rows in rows.groupby("split", sort=True):
            y_true = split_rows["_target"].to_numpy(dtype=int)
            y_pred = split_rows["direct_lp_danger_state"].to_numpy(dtype=bool)
            result[horizon_key][str(split)] = binary_rule_metrics(y_true, y_pred)
    return result


def within_gate_model_metrics(frame: pd.DataFrame, artifact_root: Path) -> dict[str, dict]:
    result = {}
    for horizon in HORIZONS:
        target = mechanism_target_name(horizon)
        prediction_path = (
            artifact_root
            / "mechanism_models"
            / "direct_lp_liquidity_removal_gate"
            / "reports"
            / "predictions"
            / f"{target}_hist_gradient_boosting_test_predictions.csv"
        )
        if not prediction_path.is_file():
            continue
        predictions = pd.read_csv(prediction_path)
        predictions = add_gate_state(predictions)
        gated = predictions[predictions["direct_lp_danger_state"]].copy()
        if gated.empty:
            continue
        y_true = gated["y_true"].astype(int).to_numpy()
        y_score = pd.to_numeric(gated["y_score"], errors="coerce").fillna(0).to_numpy()
        key = f"within_{horizon}_chain_block_delta"
        result[key] = {
            "rows": int(len(gated)),
            "positives": int(y_true.sum()),
            "base_rate": ratio(y_true.sum(), len(y_true)),
            "pr_auc": safe_average_precision(y_true, y_score),
            "roc_auc": safe_roc_auc(y_true, y_score),
            "precision_at_top_0_5pct": precision_at_fraction(y_true, y_score, 0.005),
            "recall_at_top_0_5pct": recall_at_fraction(y_true, y_score, 0.005),
            "precision_at_top_1pct": precision_at_fraction(y_true, y_score, 0.01),
            "recall_at_top_1pct": recall_at_fraction(y_true, y_score, 0.01),
            "precision_at_top_5pct": precision_at_fraction(y_true, y_score, 0.05),
            "recall_at_top_5pct": recall_at_fraction(y_true, y_score, 0.05),
        }
    return result


def within_gate_feature_signal(frame: pd.DataFrame, artifact_root: Path) -> dict[str, list[dict]]:
    result = {}
    rows = frame[(frame["split"] == "test") & frame["direct_lp_danger_state"]].copy()
    if rows.empty:
        return result
    x = feature_matrix(rows)
    for horizon in (1, 2, 4):
        target = mechanism_target_name(horizon)
        model_path = (
            artifact_root
            / "mechanism_models"
            / "direct_lp_liquidity_removal_gate"
            / "models"
            / f"{target}_hist_gradient_boosting.joblib"
        )
        if not model_path.is_file():
            continue
        y = rows["time_to_direct_lp_scam_chain_block_delta"].between(
            1,
            horizon,
            inclusive="both",
        ).astype(int)
        if y.nunique() < 2:
            continue
        model = joblib.load(model_path)
        importance = permutation_importance(
            model,
            x,
            y,
            scoring="average_precision",
            n_repeats=3,
            random_state=11,
            n_jobs=1,
        )
        rows_out = []
        for column, mean, std in sorted(
            zip(x.columns, importance.importances_mean, importance.importances_std),
            key=lambda item: item[1],
            reverse=True,
        )[:12]:
            rows_out.append({
                "feature": column,
                "average_precision_drop": float(mean),
                "std": float(std),
            })
        result[f"within_{horizon}_chain_block_delta"] = rows_out
    return result


def source_run_end_blocks(artifact_root: Path) -> dict[str, int]:
    manifest = json.loads((artifact_root / "reports" / "oot_dataset_manifest.json").read_text(encoding="utf-8"))
    ranges = manifest["ranges"]
    return {
        manifest["train_run_id"]: int(ranges["train"]["end_block"]),
        manifest["test_run_id"]: int(ranges["test"]["end_block"]),
    }


def binary_rule_metrics(y_true: np.ndarray, y_pred: np.ndarray) -> dict:
    positives = int(y_true.sum())
    predicted = int(y_pred.sum())
    tp = int((y_true & y_pred).sum())
    fp = int(((1 - y_true) & y_pred).sum())
    fn = int((y_true & (~y_pred)).sum())
    return {
        "rows": int(len(y_true)),
        "positives": positives,
        "predicted_positive_rows": predicted,
        "true_positives": tp,
        "false_positives": fp,
        "false_negatives": fn,
        "positive_rate": ratio(positives, len(y_true)),
        "precision": ratio(tp, predicted),
        "recall": ratio(tp, positives),
    }


def mechanism_target_name(horizon: int) -> str:
    return f"{MECHANISM}_within_{horizon}_chain_block_delta"


def safe_average_precision(y_true: np.ndarray, y_score: np.ndarray) -> float | None:
    if y_true.sum() <= 0 or y_true.sum() >= len(y_true):
        return None
    return float(average_precision_score(y_true, y_score))


def safe_roc_auc(y_true: np.ndarray, y_score: np.ndarray) -> float | None:
    if y_true.sum() <= 0 or y_true.sum() >= len(y_true):
        return None
    return float(roc_auc_score(y_true, y_score))


def precision_at_fraction(y_true: np.ndarray, y_score: np.ndarray, fraction: float) -> float:
    if len(y_true) == 0:
        return 0.0
    count = max(1, int(round(len(y_true) * fraction)))
    order = np.argsort(-np.asarray(y_score))[:count]
    return ratio(np.asarray(y_true)[order].sum(), count)


def recall_at_fraction(y_true: np.ndarray, y_score: np.ndarray, fraction: float) -> float:
    positives = np.asarray(y_true).sum()
    if positives <= 0:
        return 0.0
    count = max(1, int(round(len(y_true) * fraction)))
    order = np.argsort(-np.asarray(y_score))[:count]
    return ratio(np.asarray(y_true)[order].sum(), positives)


def pool_count(rows: pd.DataFrame) -> int:
    if rows.empty:
        return 0
    return int(rows[POOL_KEYS].drop_duplicates().shape[0])


def stats(values) -> dict:
    series = pd.Series(values, dtype="float64").dropna()
    if series.empty:
        return {"count": 0, "min": None, "p25": None, "median": None, "p75": None, "max": None}
    return {
        "count": int(series.count()),
        "min": float(series.min()),
        "p25": float(series.quantile(0.25)),
        "median": float(series.median()),
        "p75": float(series.quantile(0.75)),
        "max": float(series.max()),
    }


def ratio(numerator, denominator) -> float:
    denominator = float(denominator or 0)
    return float(numerator) / denominator if denominator else 0.0


def summary_markdown(metrics: dict) -> str:
    lines = [
        "# Direct LP-Removal Gate Metrics",
        "",
        f"Run: `{metrics['run_id']}`",
        f"Gate: `{metrics['gate_definition']}`",
        "",
        "## Split Summary",
        "",
        "| Split | Rows | Pools | Direct-LP scam pools | Warning coverage | Danger rows | Danger pools | Non-direct danger pools | Median first-warning lead |",
        "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
    ]
    for split, row in metrics["split_summary"].items():
        lead = row["first_warning_lead_chain_block_delta"]
        lines.append(
            "| {split} | {rows} | {pools} | {direct} | {coverage} | {danger_rows} | {danger_pools} | {non_direct} | {lead} |".format(
                split=split,
                rows=row["rows"],
                pools=row["pools"],
                direct=row["direct_lp_scam_pools"],
                coverage=pct(row["direct_lp_warning_pool_coverage"]),
                danger_rows=row["danger_rows"],
                danger_pools=row["danger_pools"],
                non_direct=row["non_direct_danger_pools"],
                lead=block_delta(lead["median"]),
            )
        )
    lines.extend([
        "",
        "## Test Rule Metrics",
        "",
        "| Horizon | Positives | Predicted rows | Precision | Recall |",
        "| --- | ---: | ---: | ---: | ---: |",
    ])
    for horizon, split_rows in metrics["rule_horizon_metrics"].items():
        row = split_rows.get("test", {})
        lines.append(
            "| `{horizon}` | {positives} | {predicted} | {precision} | {recall} |".format(
                horizon=horizon,
                positives=row.get("positives", 0),
                predicted=row.get("predicted_positive_rows", 0),
                precision=pct(row.get("precision", 0)),
                recall=pct(row.get("recall", 0)),
            )
        )
    lines.extend([
        "",
        "## Within-Gate Model Metrics",
        "",
        "These metrics only evaluate rows where the direct-LP danger state is already active.",
        "",
        "| Horizon | Gated rows | Positives | Base rate | PR AUC | ROC AUC | Precision@1% | Recall@1% |",
        "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
    ])
    for horizon, row in metrics.get("within_gate_model_metrics", {}).items():
        lines.append(
            "| `{horizon}` | {rows} | {positives} | {base} | {pr_auc} | {roc_auc} | {p1} | {r1} |".format(
                horizon=horizon,
                rows=row.get("rows", 0),
                positives=row.get("positives", 0),
                base=pct(row.get("base_rate", 0)),
                pr_auc=fmt(row.get("pr_auc")),
                roc_auc=fmt(row.get("roc_auc")),
                p1=pct(row.get("precision_at_top_1pct", 0)),
                r1=pct(row.get("recall_at_top_1pct", 0)),
            )
        )
    lines.extend([
        "",
        "## Within-Gate Feature Signal",
        "",
        "Top features by average-precision drop after permutation on gated test rows.",
        "",
    ])
    for horizon, rows in metrics.get("within_gate_feature_signal", {}).items():
        lines.extend([
            f"### `{horizon}`",
            "",
            "| Feature | AP drop |",
            "| --- | ---: |",
        ])
        for row in rows[:8]:
            lines.append(
                f"| `{row['feature']}` | {fmt(row['average_precision_drop'])} |"
            )
        lines.append("")
    lines.append("")
    return "\n".join(lines)


def pct(value) -> str:
    return f"{float(value) * 100:.2f}%"


def fmt(value) -> str:
    return "-" if value is None else f"{float(value):.4f}"


def block_delta(value) -> str:
    return "-" if value is None else f"{float(value):.2f}"


if __name__ == "__main__":
    main()
