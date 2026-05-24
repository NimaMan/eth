"""Classification metrics for scam-within-horizon targets."""

from __future__ import annotations

import numpy as np
from sklearn.metrics import average_precision_score, brier_score_loss, precision_recall_fscore_support, roc_auc_score


def classification_metrics(y_true, y_score) -> dict:
    y_true = np.asarray(y_true).astype(int)
    y_score = np.asarray(y_score).astype(float)
    y_pred = (y_score >= 0.5).astype(int)
    precision, recall, f1, _ = precision_recall_fscore_support(
        y_true,
        y_pred,
        average="binary",
        zero_division=0,
    )
    positives = int(y_true.sum())
    metrics = {
        "rows": int(len(y_true)),
        "positives": positives,
        "positive_rate": float(positives / len(y_true)) if len(y_true) else 0.0,
        "roc_auc": _safe_metric(roc_auc_score, y_true, y_score),
        "pr_auc": _safe_metric(average_precision_score, y_true, y_score),
        "brier": _safe_metric(brier_score_loss, y_true, y_score),
        "precision_at_0_5": float(precision),
        "recall_at_0_5": float(recall),
        "f1_at_0_5": float(f1),
    }
    for label, fraction in (("0_5", 0.005), ("1", 0.01), ("2", 0.02), ("5", 0.05), ("10", 0.10)):
        metrics[f"precision_at_top_{label}pct"] = precision_at_fraction(y_true, y_score, fraction)
        metrics[f"recall_at_top_{label}pct"] = recall_at_fraction(y_true, y_score, fraction)
    return metrics


def precision_at_fraction(y_true, y_score, fraction: float) -> float:
    if len(y_true) == 0:
        return 0.0
    count = max(1, int(round(len(y_true) * fraction)))
    order = np.argsort(-np.asarray(y_score))[:count]
    return float(np.asarray(y_true)[order].sum() / count)


def recall_at_fraction(y_true, y_score, fraction: float) -> float:
    positives = np.asarray(y_true).sum()
    if positives <= 0:
        return 0.0
    count = max(1, int(round(len(y_true) * fraction)))
    order = np.argsort(-np.asarray(y_score))[:count]
    return float(np.asarray(y_true)[order].sum() / positives)


def _safe_metric(fn, y_true, y_score) -> float | None:
    try:
        return float(fn(y_true, y_score))
    except Exception:
        return None
