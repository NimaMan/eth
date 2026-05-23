"""Extract raw Risk Atlas rows for a configured run."""

from __future__ import annotations

from pathlib import Path

import pandas as pd

from .. import db
from ..run_config import MODELING_ROOT, RunConfig


SQL_ROOT = MODELING_ROOT / "sql"


def extract_raw_frames(config: RunConfig) -> dict[str, pd.DataFrame]:
    params = {"run_id": config.run_id, "protocol": config.protocol}
    return {
        "observations": db.read_sql_file(SQL_ROOT / "observations.sql", params),
        "scam_labels": db.read_sql_file(SQL_ROOT / "scam_labels.sql", params),
        "event_evidence": db.read_sql_file(SQL_ROOT / "event_evidence.sql", params),
    }


def write_raw_frames(config: RunConfig, frames: dict[str, pd.DataFrame]) -> None:
    dataset_dir = config.artifact_root / "datasets"
    dataset_dir.mkdir(parents=True, exist_ok=True)
    for name, frame in frames.items():
        _write_frame(frame, dataset_dir / f"{name}.csv")


def read_raw_frames(config: RunConfig) -> dict[str, pd.DataFrame]:
    dataset_dir = config.artifact_root / "datasets"
    return {
        "observations": pd.read_csv(dataset_dir / "observations.csv"),
        "scam_labels": pd.read_csv(dataset_dir / "scam_labels.csv"),
        "event_evidence": pd.read_csv(dataset_dir / "event_evidence.csv"),
    }


def _write_frame(frame: pd.DataFrame, path: Path) -> None:
    frame.to_csv(path, index=False)
