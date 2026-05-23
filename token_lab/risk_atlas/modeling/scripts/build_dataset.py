#!/usr/bin/env python
"""Build a run-scoped modeling dataset."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from risk_atlas_modeling.datasets.extract import extract_raw_frames, write_raw_frames
from risk_atlas_modeling.datasets.labels import build_labeled_dataset
from risk_atlas_modeling.datasets.splits import split_labels
from risk_atlas_modeling.run_config import load_config


def main() -> None:
    args = parse_args()
    config = load_config(args.config)
    frames = extract_raw_frames(config)
    labeled = build_labeled_dataset(frames["observations"], frames["scam_labels"], config)
    labeled["split"] = split_labels(labeled, config)
    config.artifact_root.mkdir(parents=True, exist_ok=True)
    write_raw_frames(config, frames)
    dataset_path = config.artifact_root / "datasets" / "modeling_dataset.csv"
    labeled.to_csv(dataset_path, index=False)
    print(f"wrote {dataset_path} rows={len(labeled)}")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--config", required=True)
    return parser.parse_args()


if __name__ == "__main__":
    main()
