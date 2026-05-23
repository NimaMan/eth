"""Build a compact markdown report from modeling metrics."""

from __future__ import annotations

import json
from pathlib import Path


def build_markdown(metrics: dict) -> str:
    lines = [
        "# Risk Atlas Short-Horizon Scam Models",
        "",
        f"Run: `{metrics.get('run_id', '-')}`",
        f"Protocol: `{metrics.get('protocol', '-')}`",
        f"Focus: `{metrics.get('modeling_focus', 'baseline')}`",
        f"Rows: `{metrics.get('dataset_rows', '-')}`",
        "",
    ]
    manifest = metrics.get("oot_dataset_manifest") or {}
    if manifest:
        rows = manifest.get("rows", {})
        pools = manifest.get("pools", {})
        ranges = manifest.get("ranges", {})
        lines.extend(
            [
                "## Out-Of-Time Dataset",
                "",
                f"Train run: `{manifest.get('train_run_id', '-')}`",
                f"Test run: `{manifest.get('test_run_id', '-')}`",
                f"Train blocks: `{_range_label(ranges.get('train'))}`",
                f"Gap blocks: `{_range_label(ranges.get('gap'))}`",
                f"Test blocks: `{_range_label(ranges.get('test'))}`",
                f"Split rows: train `{rows.get('train', 0)}`, validation `{rows.get('validation', 0)}`, test `{rows.get('test', 0)}`",
                f"Dropped overlapping test pools: `{pools.get('dropped_test_overlap', 0)}`",
                "",
            ]
        )
    lines.extend(
        [
        "## Classification",
        "",
            "| Target | Best model | Test PR AUC | Test ROC AUC | Positive rate | Precision@1% | Recall@1% | Precision@5% | Recall@5% |",
            "| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
        ]
    )
    for target, row in metrics.get("best_classification", {}).items():
        test = row.get("test", {})
        lines.append(
            "| {target} | {model} | {pr_auc} | {roc_auc} | {positive_rate} | {p1} | {r1} | {p5} | {r5} |".format(
                target=target,
                model=row.get("model", "-"),
                pr_auc=_fmt(test.get("pr_auc")),
                roc_auc=_fmt(test.get("roc_auc")),
                positive_rate=_pct(test.get("positive_rate")),
                p1=_pct(test.get("precision_at_top_1pct")),
                r1=_pct(test.get("recall_at_top_1pct")),
                p5=_pct(test.get("precision_at_top_5pct")),
                r5=_pct(test.get("recall_at_top_5pct")),
            )
        )
    if metrics.get("time_to_event"):
        lines.extend(["", "## Time To Scam", ""])
        lines.extend([
            "| Model | Test MAE chain_block_delta | Test median AE chain_block_delta | Test R2 |",
            "| --- | ---: | ---: | ---: |",
        ])
        for row in metrics.get("time_to_event", []):
            test = row.get("test", {})
            lines.append(
                f"| {row.get('model', '-')} | {_fmt(test.get('mae_chain_block_delta'))} | "
                f"{_fmt(test.get('median_ae_chain_block_delta'))} | {_fmt(test.get('r2'))} |"
            )
    lines.append("")
    return "\n".join(lines)


def write_report(metrics_path: Path, report_path: Path) -> None:
    metrics = json.loads(metrics_path.read_text(encoding="utf-8"))
    report_path.write_text(build_markdown(metrics), encoding="utf-8")


def _range_label(value) -> str:
    if not isinstance(value, dict):
        return "-"
    return f"{value.get('start_block', '-')}-{value.get('end_block', '-')}"


def _fmt(value) -> str:
    return "-" if value is None else f"{float(value):.4f}"


def _pct(value) -> str:
    return "-" if value is None else f"{float(value) * 100:.2f}%"
