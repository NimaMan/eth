"""Feature importance extraction for baseline models."""

from __future__ import annotations

import numpy as np
import pandas as pd


def feature_importance(model, feature_names: list[str]) -> pd.DataFrame:
    estimator = model.steps[-1][1] if hasattr(model, "steps") else model
    values = None
    if hasattr(estimator, "feature_importances_"):
        values = estimator.feature_importances_
    elif hasattr(estimator, "coef_"):
        coef = np.asarray(estimator.coef_)
        values = np.abs(coef[0] if coef.ndim > 1 else coef)
    if values is None:
        return pd.DataFrame(columns=["feature", "importance"])
    frame = pd.DataFrame({"feature": feature_names, "importance": np.asarray(values, dtype=float)})
    return frame.sort_values("importance", ascending=False).reset_index(drop=True)
