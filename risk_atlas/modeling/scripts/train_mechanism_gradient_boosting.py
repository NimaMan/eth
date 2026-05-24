#!/usr/bin/env python3
"""Train mechanism-specific short-horizon gradient-boosting heads."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

import joblib
import numpy as np
import pandas as pd
from sklearn.ensemble import HistGradientBoostingClassifier
from sklearn.impute import SimpleImputer
from sklearn.pipeline import Pipeline
from sklearn.utils.class_weight import compute_sample_weight

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from risk_atlas_modeling.datasets.features import feature_matrix
from risk_atlas_modeling.evaluation.classification import classification_metrics


DEFAULT_RUN_ID = "backdoor_horizon_oot_v1"
DEFAULT_MECHANISM = "pair_balance_backdoor_drain"
DEFAULT_HORIZONS = (1, 2, 3)
ULIQ_TOKEN = "0x0ca72b24abf950be8bb145f5c68f7004485192b2"
ULIQ_POOL = "0x57ca70b663ead85122589f345835d8f8209608dc"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run-id", default=DEFAULT_RUN_ID)
    parser.add_argument("--mechanism", default=DEFAULT_MECHANISM)
    parser.add_argument("--horizons", nargs="+", type=int, default=list(DEFAULT_HORIZONS))
    parser.add_argument("--max-iter", type=int, default=160)
    parser.add_argument("--learning-rate", type=float, default=0.05)
    parser.add_argument("--max-leaf-nodes", type=int, default=15)
    parser.add_argument("--min-samples-leaf", type=int, default=20)
    parser.add_argument(
        "--sample-weighting",
        choices=["balanced", "none"],
        default="balanced",
    )
    parser.add_argument(
        "--output-label",
        default="",
        help="Optional suffix for the mechanism model output directory.",
    )
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    artifact_root = ROOT / "artifacts" / args.run_id
    dataset_path = artifact_root / "datasets" / "modeling_dataset.csv"
    frame = pd.read_csv(dataset_path)
    frame = add_mechanism_labels(frame, artifact_root, args.mechanism, args.horizons)

    output_name = args.mechanism if not args.output_label else f"{args.mechanism}_{args.output_label}"
    output_root = artifact_root / "mechanism_models" / output_name
    model_dir = output_root / "models"
    report_dir = output_root / "reports"
    prediction_dir = report_dir / "predictions"
    for directory in (model_dir, report_dir, prediction_dir):
        directory.mkdir(parents=True, exist_ok=True)

    metrics = {
        "run_id": args.run_id,
        "mechanism": args.mechanism,
        "model_family": "hist_gradient_boosting",
        "model_params": {
            "max_iter": args.max_iter,
            "learning_rate": args.learning_rate,
            "max_leaf_nodes": args.max_leaf_nodes,
            "min_samples_leaf": args.min_samples_leaf,
            "sample_weighting": args.sample_weighting,
        },
        "dataset_rows": int(len(frame)),
        "split_rows": {
            key: int(value) for key, value in frame["split"].value_counts().sort_index().items()
        },
        "feature_columns": list(feature_matrix(frame).columns),
        "classification": [],
        "uliq_holdout": {},
    }

    for horizon in args.horizons:
        target = mechanism_target_name(args.mechanism, horizon)
        row = train_target(
            frame=frame,
            target=target,
            model_dir=model_dir,
            prediction_dir=prediction_dir,
            max_iter=args.max_iter,
            learning_rate=args.learning_rate,
            max_leaf_nodes=args.max_leaf_nodes,
            min_samples_leaf=args.min_samples_leaf,
            sample_weighting=args.sample_weighting,
        )
        metrics["classification"].append(row)
        metrics["uliq_holdout"][target] = uliq_rows(
            frame=frame,
            prediction_path=Path(row["test_prediction_path"]),
            target=target,
        )

    metrics_path = report_dir / "metrics.json"
    metrics_path.write_text(json.dumps(metrics, indent=2, sort_keys=True), encoding="utf-8")
    write_summary(metrics, report_dir / "summary.md")
    print(f"wrote {metrics_path}")
    print(f"wrote {report_dir / 'summary.md'}")


def add_mechanism_labels(
    frame: pd.DataFrame,
    artifact_root: Path,
    mechanism: str,
    horizons: list[int],
) -> pd.DataFrame:
    result = frame.copy()
    result["token_address"] = result["token_address"].astype(str).str.lower()
    result["pool_address"] = result["pool_address"].astype(str).str.lower()
    result["source_run_id"] = result["source_run_id"].astype(str)

    events = mechanism_events(artifact_root, mechanism)
    labels = {
        (row.run_id, row.token_address, row.pool_address): int(row.mechanism_scam_block)
        for row in events.itertuples(index=False)
    }
    result["mechanism_scam_block"] = [
        labels.get((row.source_run_id, row.token_address, row.pool_address), np.nan)
        for row in result.itertuples(index=False)
    ]
    result["time_to_mechanism_scam_chain_block_delta"] = (
        result["mechanism_scam_block"] - result["block_number"]
    )
    result.loc[
        result["mechanism_scam_block"].isna(),
        "time_to_mechanism_scam_chain_block_delta",
    ] = np.nan

    source_end_block = source_run_end_blocks(artifact_root)
    result["source_end_block"] = result["source_run_id"].map(source_end_block)
    for horizon in horizons:
        target = mechanism_target_name(mechanism, horizon)
        positive = result["time_to_mechanism_scam_chain_block_delta"].between(
            1,
            horizon,
            inclusive="both",
        )
        known = result["mechanism_scam_block"].notna() | (
            result["block_number"] <= result["source_end_block"] - horizon
        )
        result[target] = np.where(known, positive.astype(int), np.nan)
    return result


def mechanism_events(artifact_root: Path, mechanism: str) -> pd.DataFrame:
    frames = []
    for name in ("train_event_evidence.csv", "test_event_evidence.csv"):
        path = artifact_root / "datasets" / name
        events = pd.read_csv(path)
        events["token_address"] = events["token_address"].astype(str).str.lower()
        events["pool_address"] = events["pool_address"].astype(str).str.lower()
        selected = events[
            (events["event_kind"] == "scam")
            & (events["mechanism"].astype(str) == mechanism)
        ].copy()
        frames.append(selected)
    events = pd.concat(frames, ignore_index=True)
    if events.empty:
        return pd.DataFrame(columns=["run_id", "token_address", "pool_address", "mechanism_scam_block"])
    return (
        events.groupby(["run_id", "token_address", "pool_address"], as_index=False)["block_number"]
        .min()
        .rename(columns={"block_number": "mechanism_scam_block"})
    )


def source_run_end_blocks(artifact_root: Path) -> dict[str, int]:
    manifest_path = artifact_root / "reports" / "oot_dataset_manifest.json"
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    ranges = manifest["ranges"]
    return {
        manifest["train_run_id"]: int(ranges["train"]["end_block"]),
        manifest["test_run_id"]: int(ranges["test"]["end_block"]),
    }


def train_target(
    *,
    frame: pd.DataFrame,
    target: str,
    model_dir: Path,
    prediction_dir: Path,
    max_iter: int,
    learning_rate: float,
    max_leaf_nodes: int,
    min_samples_leaf: int,
    sample_weighting: str,
) -> dict:
    rows = frame[frame[target].notna()].copy()
    if rows[target].nunique() < 2:
        raise RuntimeError(f"target has fewer than two classes: {target}")

    x = feature_matrix(rows)
    y = rows[target].astype(int)
    train = rows["split"] == "train"
    validation = rows["split"] == "validation"
    test = rows["split"] == "test"
    if y[train].nunique() < 2:
        raise RuntimeError(f"train split has fewer than two classes: {target}")

    model = Pipeline([
        ("impute", SimpleImputer(strategy="median")),
        (
            "model",
            HistGradientBoostingClassifier(
                max_iter=max_iter,
                learning_rate=learning_rate,
                max_leaf_nodes=max_leaf_nodes,
                min_samples_leaf=min_samples_leaf,
                random_state=7,
            ),
        ),
    ])
    if sample_weighting == "balanced":
        sample_weight = compute_sample_weight(class_weight="balanced", y=y[train])
        model.fit(x[train], y[train], model__sample_weight=sample_weight)
    else:
        model.fit(x[train], y[train])

    validation_score = model.predict_proba(x[validation])[:, 1]
    test_score = model.predict_proba(x[test])[:, 1]

    model_path = model_dir / f"{target}_hist_gradient_boosting.joblib"
    prediction_path = prediction_dir / f"{target}_hist_gradient_boosting_test_predictions.csv"
    joblib.dump(model, model_path)
    write_test_predictions(rows.loc[test], target, test_score, prediction_path)

    return {
        "target": target,
        "model": "hist_gradient_boosting",
        "model_path": str(model_path),
        "test_prediction_path": str(prediction_path),
        "train_rows": int(train.sum()),
        "validation": classification_metrics(y[validation], validation_score),
        "test": classification_metrics(y[test], test_score),
    }


def write_test_predictions(rows: pd.DataFrame, target: str, score: np.ndarray, path: Path) -> None:
    columns = [
        "source_run_id",
        "token_address",
        "pool_address",
        "protocol",
        "block_number",
        "active_observation_index",
        "mechanism_scam_block",
        "time_to_mechanism_scam_chain_block_delta",
        "time_to_scam_chain_block_delta",
        "scam_block",
        "pair_balance_backdoor_signal_seen_as_of",
        "pair_balance_backdoor_signal_in_block",
        "last_pair_balance_backdoor_signal_to_as_of_chain_block_delta",
        "control_transfer_from_after_renounce_seen_as_of",
        "control_transfer_from_after_renounce_in_block",
        "control_transfer_from_holder_to_burn_seen_as_of",
        "control_transfer_from_holder_to_burn_in_block",
        "control_transfer_from_without_transfer_log_seen_as_of",
        "control_transfer_from_without_transfer_log_in_block",
        "pair_token_to_control_seen_as_of",
        "pair_token_to_control_in_block",
        "pair_token_to_control_to_pool_reserve_ratio",
        "lp_removable_pct_as_of",
        "creator_lp_balance_pct_as_of",
        "token_transfer_to_pool_token_reserve_ratio",
    ]
    available = [column for column in columns if column in rows.columns]
    predictions = rows[available].copy()
    predictions["target"] = target
    predictions["y_true"] = rows[target].astype(int).to_numpy()
    predictions["y_score"] = score
    predictions = predictions.sort_values("y_score", ascending=False)
    predictions.to_csv(path, index=False)


def uliq_rows(frame: pd.DataFrame, prediction_path: Path, target: str) -> list[dict]:
    predictions = pd.read_csv(prediction_path)
    rows = predictions[
        (predictions["token_address"].astype(str).str.lower() == ULIQ_TOKEN)
        & (predictions["pool_address"].astype(str).str.lower() == ULIQ_POOL)
    ].sort_values("block_number")
    if rows.empty:
        return []
    return rows.replace({np.nan: None}).to_dict("records")


def mechanism_target_name(mechanism: str, horizon: int) -> str:
    return f"{mechanism}_within_{horizon}_chain_block_delta"


def write_summary(metrics: dict, path: Path) -> None:
    lines = [
        "# Mechanism-Specific Gradient Boosting",
        "",
        f"Run: `{metrics['run_id']}`",
        f"Mechanism: `{metrics['mechanism']}`",
        f"Model: `hist_gradient_boosting` with `{metrics['model_params']['sample_weighting']}` sample weighting",
        "",
        "## Performance",
        "",
        "| Target | Test positives | Positive rate | PR AUC | ROC AUC | Precision@1% | Recall@1% | Precision@5% | Recall@5% |",
        "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
    ]
    for row in metrics["classification"]:
        test = row["test"]
        lines.append(
            "| {target} | {positives} | {positive_rate} | {pr_auc} | {roc_auc} | {p1} | {r1} | {p5} | {r5} |".format(
                target=row["target"],
                positives=test["positives"],
                positive_rate=_pct(test["positive_rate"]),
                pr_auc=_fmt(test["pr_auc"]),
                roc_auc=_fmt(test["roc_auc"]),
                p1=_pct(test["precision_at_top_1pct"]),
                r1=_pct(test["recall_at_top_1pct"]),
                p5=_pct(test["precision_at_top_5pct"]),
                r5=_pct(test["recall_at_top_5pct"]),
            )
        )
    lines.extend(["", "## ULIQ Holdout", ""])
    for target, rows in metrics["uliq_holdout"].items():
        lines.extend([
            f"### `{target}`",
            "",
            "| Block | Blocks to mechanism scam | y | Score | Backdoor seen | Holder burn | No transfer log |",
            "| ---: | ---: | ---: | ---: | --- | --- | --- |",
        ])
        for row in rows:
            lines.append(
                "| {block} | {delta} | {y} | {score} | {backdoor} | {holder} | {no_log} |".format(
                    block=_int(row.get("block_number")),
                    delta=_int(row.get("time_to_mechanism_scam_chain_block_delta")),
                    y=_int(row.get("y_true")),
                    score=_pct(row.get("y_score")),
                    backdoor=_bool(row.get("pair_balance_backdoor_signal_seen_as_of")),
                    holder=_bool(row.get("control_transfer_from_holder_to_burn_seen_as_of")),
                    no_log=_bool(row.get("control_transfer_from_without_transfer_log_seen_as_of")),
                )
            )
        lines.append("")
    path.write_text("\n".join(lines).rstrip() + "\n", encoding="utf-8")


def _fmt(value) -> str:
    return "-" if value is None else f"{float(value):.4f}"


def _pct(value) -> str:
    return "-" if value is None else f"{float(value) * 100:.2f}%"


def _int(value) -> str:
    if value is None:
        return "-"
    try:
        if pd.isna(value):
            return "-"
        return str(int(float(value)))
    except (TypeError, ValueError):
        return "-"


def _bool(value) -> str:
    if value is None:
        return "-"
    if isinstance(value, str):
        return "Y" if value.lower() == "true" else "-"
    try:
        if pd.isna(value):
            return "-"
    except TypeError:
        pass
    return "Y" if bool(value) else "-"


if __name__ == "__main__":
    main()
