#!/usr/bin/env python
"""Train baseline Risk Atlas scam models."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

import joblib
import numpy as np
import pandas as pd

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from risk_atlas_modeling.datasets.features import feature_matrix
from risk_atlas_modeling.evaluation.classification import classification_metrics
from risk_atlas_modeling.evaluation.feature_importance import feature_importance
from risk_atlas_modeling.evaluation.time_to_event import regression_metrics
from risk_atlas_modeling.models.baselines import classification_models, regression_models
from risk_atlas_modeling.reports.build_report import write_report
from risk_atlas_modeling.run_config import load_config


def main() -> None:
    args = parse_args()
    config = load_config(args.config)
    dataset_path = config.artifact_root / "datasets" / "modeling_dataset.csv"
    frame = pd.read_csv(dataset_path)
    model_dir = config.artifact_root / "models"
    report_dir = config.artifact_root / "reports"
    prediction_dir = report_dir / "predictions"
    model_dir.mkdir(parents=True, exist_ok=True)
    report_dir.mkdir(parents=True, exist_ok=True)
    prediction_dir.mkdir(parents=True, exist_ok=True)

    targets = [
        *(f"scam_within_{h}_chain_block_delta" for h in config.chain_block_delta_horizons),
        *(f"scam_within_{h}_active_observation_delta" for h in config.active_observation_delta_horizons),
    ]
    metrics = {
        "run_id": config.run_id,
        "protocol": config.protocol,
        "modeling_focus": _modeling_focus(config.artifact_root),
        "dataset_rows": int(len(frame)),
        "split_rows": {key: int(value) for key, value in frame["split"].value_counts().sort_index().items()},
        "feature_columns": list(feature_matrix(frame).columns),
        "oot_dataset_manifest": _load_json(config.artifact_root / "reports" / "oot_dataset_manifest.json"),
        "classification": [],
        "best_classification": {},
        "review_samples": {},
        "time_to_event": [],
    }
    for target in targets:
        metrics["classification"].extend(
            train_classification_target(
                frame,
                target,
                model_dir,
                report_dir,
                prediction_dir,
                config.baseline_models,
            )
        )
    for row in metrics["classification"]:
        target = row["target"]
        current = metrics["best_classification"].get(target)
        score = _score(row.get("validation", {}), row.get("test", {}), "pr_auc")
        current_score = _score((current or {}).get("validation", {}), (current or {}).get("test", {}), "pr_auc")
        if current is None or score > current_score:
            metrics["best_classification"][target] = row
    metrics["review_samples"] = build_review_samples(metrics["best_classification"], prediction_dir)
    if config.train_time_to_event:
        metrics["time_to_event"] = train_time_to_event(frame, model_dir)

    metrics_path = report_dir / "baseline_metrics.json"
    metrics_path.write_text(json.dumps(metrics, indent=2, sort_keys=True), encoding="utf-8")
    write_report(metrics_path, report_dir / "summary.md")
    print(f"wrote {metrics_path}")


def train_classification_target(
    frame: pd.DataFrame,
    target: str,
    model_dir: Path,
    report_dir: Path,
    prediction_dir: Path,
    selected_models: list[str],
) -> list[dict]:
    rows = frame[frame[target].notna()].copy()
    if rows[target].nunique() < 2:
        return []
    x = feature_matrix(rows)
    feature_names = list(x.columns)
    y = rows[target].astype(int)
    results = []
    models = classification_models()
    names = selected_models or list(models)
    for model_name in names:
        model = models.get(model_name)
        if model is None:
            continue
        train = rows["split"] == "train"
        validation = rows["split"] == "validation"
        test = rows["split"] == "test"
        if y[train].nunique() < 2:
            continue
        model.fit(x[train], y[train])
        validation_score = _predict_score(model, x[validation])
        test_score = _predict_score(model, x[test])
        prediction_path = prediction_dir / f"{target}_{model_name}_test_predictions.csv"
        write_test_predictions(rows.loc[test], target, test_score, prediction_path)
        result = {
            "target": target,
            "model": model_name,
            "train_rows": int(train.sum()),
            "validation": classification_metrics(y[validation], validation_score),
            "test": classification_metrics(y[test], test_score),
            "test_prediction_path": str(prediction_path),
        }
        results.append(result)
        joblib.dump(model, model_dir / f"{target}_{model_name}.joblib")
        importance = feature_importance(model, feature_names)
        if not importance.empty:
            importance.to_csv(report_dir / f"{target}_{model_name}_feature_importance.csv", index=False)
    return results


def write_test_predictions(rows: pd.DataFrame, target: str, score: np.ndarray, path: Path) -> None:
    columns = [
        "source_run_id",
        "token_address",
        "pool_address",
        "protocol",
        "block_number",
        "active_observation_index",
        "time_to_scam_chain_block_delta",
        "scam_block",
        "pair_balance_backdoor_signal_seen_as_of",
        "control_transfer_from_after_renounce_seen_as_of",
        "control_transfer_from_holder_to_burn_seen_as_of",
        "control_transfer_from_without_transfer_log_seen_as_of",
        "control_transfer_from_pair_seen_as_of",
        "pair_token_to_control_seen_as_of",
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


def build_review_samples(best_rows: dict, prediction_dir: Path) -> dict:
    samples = {}
    for target, row in best_rows.items():
        model = row.get("model")
        if not model:
            continue
        path = prediction_dir / f"{target}_{model}_test_predictions.csv"
        if not path.is_file():
            continue
        predictions = pd.read_csv(path)
        if predictions.empty:
            continue
        top_cutoff = max(1, int(round(len(predictions) * 0.01)))
        top = predictions.head(top_cutoff).copy()
        positives = predictions[predictions["y_true"] == 1].copy()
        samples[target] = {
            "model": model,
            "prediction_path": str(path),
            "top_1pct_rows": int(top_cutoff),
            "true_positives_top_1pct": _records(top[top["y_true"] == 1].head(20)),
            "false_positives_top_1pct": _records(top[top["y_true"] == 0].head(20)),
            "false_negatives_lowest_score": _records(positives.sort_values("y_score", ascending=True).head(20)),
            "uliq_holdout": _records(
                predictions[
                    predictions["token_address"].astype(str).str.lower()
                    == "0x0ca72b24abf950be8bb145f5c68f7004485192b2"
                ].head(20)
            ),
        }
    return samples


def _records(frame: pd.DataFrame) -> list[dict]:
    if frame.empty:
        return []
    return frame.replace({np.nan: None}).to_dict("records")


def train_time_to_event(frame: pd.DataFrame, model_dir: Path) -> list[dict]:
    rows = frame[frame["time_to_scam_chain_block_delta"].notna()].copy()
    rows = rows[rows["time_to_scam_chain_block_delta"] > 0]
    if len(rows) < 20:
        return []
    x = feature_matrix(rows)
    y = np.log1p(rows["time_to_scam_chain_block_delta"].astype(float))
    results = []
    for model_name, model in regression_models().items():
        train = rows["split"] == "train"
        validation = rows["split"] == "validation"
        test = rows["split"] == "test"
        if train.sum() < 20 or validation.sum() == 0 or test.sum() == 0:
            continue
        model.fit(x[train], y[train])
        validation_pred = np.expm1(model.predict(x[validation])).clip(min=0)
        test_pred = np.expm1(model.predict(x[test])).clip(min=0)
        result = {
            "target": "time_to_scam_chain_block_delta",
            "model": model_name,
            "train_rows": int(train.sum()),
            "validation": regression_metrics(rows.loc[validation, "time_to_scam_chain_block_delta"], validation_pred),
            "test": regression_metrics(rows.loc[test, "time_to_scam_chain_block_delta"], test_pred),
        }
        results.append(result)
        joblib.dump(model, model_dir / f"time_to_scam_chain_block_delta_{model_name}.joblib")
    return results


def _predict_score(model, x):
    if len(x) == 0:
        return np.asarray([])
    if hasattr(model, "predict_proba"):
        return model.predict_proba(x)[:, 1]
    if hasattr(model, "decision_function"):
        raw = model.decision_function(x)
        return 1 / (1 + np.exp(-raw))
    return model.predict(x)


def _score(validation: dict, test: dict, key: str) -> float:
    value = validation.get(key)
    if value is None:
        value = test.get(key)
    return float(value) if value is not None else -1.0


def _load_json(path: Path) -> dict | None:
    if not path.is_file():
        return None
    return json.loads(path.read_text(encoding="utf-8"))


def _modeling_focus(artifact_root: Path) -> str:
    manifest = _load_json(artifact_root / "reports" / "oot_dataset_manifest.json")
    if manifest:
        return str(manifest.get("modeling_focus") or "out_of_time_scam_probability")
    return "short_horizon_chain_block_scam_probability"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--config", required=True)
    return parser.parse_args()


if __name__ == "__main__":
    main()
