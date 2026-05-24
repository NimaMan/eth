#!/usr/bin/env python
"""Evaluate trained baseline models for a Risk Atlas run."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from risk_atlas_modeling.reports.build_report import write_report
from risk_atlas_modeling.run_config import load_config


def main() -> None:
    args = parse_args()
    config = load_config(args.config)
    metrics_path = config.artifact_root / "reports" / "baseline_metrics.json"
    if not metrics_path.is_file():
        raise SystemExit(f"missing metrics file: {metrics_path}")
    metrics = json.loads(metrics_path.read_text(encoding="utf-8"))
    write_report(metrics_path, config.artifact_root / "reports" / "summary.md")
    print_best(metrics)


def print_best(metrics: dict) -> None:
    for target, row in metrics.get("best_classification", {}).items():
        test = row.get("test", {})
        print(
            f"{target}: {row.get('model')} "
            f"test_pr_auc={test.get('pr_auc')} test_roc_auc={test.get('roc_auc')}"
        )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--config", required=True)
    return parser.parse_args()


if __name__ == "__main__":
    main()
