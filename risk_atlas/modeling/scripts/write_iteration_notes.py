#!/usr/bin/env python
"""Write iteration learning notes from a completed OOT modeling artifact."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from risk_atlas_modeling.oot_config import load_oot_config


def main() -> None:
    args = parse_args()
    config = load_oot_config(args.config)
    metrics_path = config.artifact_root / "reports" / "baseline_metrics.json"
    if not metrics_path.is_file():
        raise SystemExit(f"missing metrics file: {metrics_path}")
    metrics = json.loads(metrics_path.read_text(encoding="utf-8"))
    iteration_path = config.experiment_root / "iterations" / f"iteration_{config.current_iteration:02d}.md"
    iteration_path.parent.mkdir(parents=True, exist_ok=True)
    iteration_path.write_text(build_notes(config.experiment_id, metrics), encoding="utf-8")
    print(f"wrote {iteration_path}")


def build_notes(experiment_id: str, metrics: dict) -> str:
    manifest = metrics.get("oot_dataset_manifest") or {}
    rows = manifest.get("rows", {})
    pools = manifest.get("pools", {})
    lines = [
        "# Iteration 01",
        "",
        "Status: completed baseline OOT training.",
        "",
        "## Dataset",
        "",
        f"- Experiment: `{experiment_id}`",
        f"- Train rows: `{rows.get('train', 0)}`",
        f"- Validation rows: `{rows.get('validation', 0)}`",
        f"- Test rows: `{rows.get('test', 0)}`",
        f"- Dropped overlapping test pools: `{pools.get('dropped_test_overlap', 0)}`",
        "",
        "## Best Models",
        "",
        "| Target | Model | Test PR AUC | Test ROC AUC | Precision@1% | Recall@1% |",
        "| --- | --- | ---: | ---: | ---: | ---: |",
    ]
    for target, row in (metrics.get("best_classification") or {}).items():
        test = row.get("test", {})
        lines.append(
            f"| `{target}` | `{row.get('model', '-')}` | {_fmt(test.get('pr_auc'))} | "
            f"{_fmt(test.get('roc_auc'))} | {_pct(test.get('precision_at_top_1pct'))} | "
            f"{_pct(test.get('recall_at_top_1pct'))} |"
        )
    lines.extend(["", "## ULIQ Holdout", ""])
    for target, sample in (metrics.get("review_samples") or {}).items():
        holdout = sample.get("uliq_holdout") or []
        if not holdout:
            lines.append(f"- `{target}`: no ULIQ rows after overlap filtering.")
            continue
        best = max(holdout, key=lambda row: float(row.get("y_score") or 0))
        lines.append(
            f"- `{target}`: max p={_fmt(best.get('y_score'))} at block "
            f"`{best.get('block_number')}`; y={best.get('y_true')}."
        )
    lines.extend(
        [
            "",
            "## Review Notes",
            "",
            "- True positives: pending manual pool review.",
            "- False positives: pending manual pool review.",
            "- False negatives: pending manual pool review.",
            "- Next feature change: pending after reviewing the prediction samples.",
            "",
        ]
    )
    return "\n".join(lines)


def _fmt(value) -> str:
    return "-" if value is None else f"{float(value):.4f}"


def _pct(value) -> str:
    return "-" if value is None else f"{float(value) * 100:.2f}%"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--config", required=True)
    return parser.parse_args()


if __name__ == "__main__":
    main()
