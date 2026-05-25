"""Extract raw Risk Atlas rows for a configured run."""

from __future__ import annotations

import pandas as pd

from .. import db
from ..queries import EVENT_EVIDENCE_SQL, OBSERVATIONS_SQL, SCAM_LABELS_SQL
from ..run_config import RunConfig


def extract_raw_frames(config: RunConfig) -> dict[str, pd.DataFrame]:
    params = {"run_id": config.run_id, "protocol": config.protocol}
    return {
        "observations": db.read_sql(OBSERVATIONS_SQL, params),
        "scam_labels": db.read_sql(SCAM_LABELS_SQL, params),
        "event_evidence": db.read_sql(EVENT_EVIDENCE_SQL, params),
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
