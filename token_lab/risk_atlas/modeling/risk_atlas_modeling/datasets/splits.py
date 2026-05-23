"""Train, validation, and test split helpers."""

from __future__ import annotations

import pandas as pd

from .. import db
from ..run_config import RunConfig


def split_labels(frame: pd.DataFrame, config: RunConfig) -> pd.Series:
    bounds = db.fetch_run_bounds(config.run_id)
    start = bounds["start_block"]
    end = bounds["end_block"]
    train_end = start + int((end - start) * config.train_end_fraction)
    validation_end = start + int((end - start) * config.validation_end_fraction)
    block = pd.to_numeric(frame["block_number"], errors="coerce")
    split = pd.Series("test", index=frame.index)
    split.loc[block <= train_end] = "train"
    split.loc[(block > train_end) & (block <= validation_end)] = "validation"
    return split
