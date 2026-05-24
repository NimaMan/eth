"""Time-to-event evaluation utilities."""

from __future__ import annotations

import numpy as np
from sklearn.metrics import mean_absolute_error, median_absolute_error, r2_score


def regression_metrics(y_true, y_pred) -> dict:
    y_true = np.asarray(y_true, dtype=float)
    y_pred = np.asarray(y_pred, dtype=float)
    return {
        "rows": int(len(y_true)),
        "mae_chain_block_delta": float(mean_absolute_error(y_true, y_pred)) if len(y_true) else None,
        "median_ae_chain_block_delta": float(median_absolute_error(y_true, y_pred)) if len(y_true) else None,
        "r2": _safe_r2(y_true, y_pred),
    }


def _safe_r2(y_true, y_pred) -> float | None:
    try:
        return float(r2_score(y_true, y_pred))
    except Exception:
        return None
