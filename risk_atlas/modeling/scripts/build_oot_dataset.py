#!/usr/bin/env python
"""Build an out-of-time Risk Atlas modeling dataset from separate train/test runs."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

import pandas as pd

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from risk_atlas_modeling import db
from risk_atlas_modeling.datasets.labels import POOL_KEYS, build_labeled_dataset
from risk_atlas_modeling.datasets.splits import split_labels
from risk_atlas_modeling.oot_config import OotExperimentConfig, load_oot_config
from risk_atlas_modeling.queries import EVENT_EVIDENCE_SQL, OBSERVATIONS_SQL, SCAM_LABELS_SQL


def main() -> None:
    args = parse_args()
    config = load_oot_config(args.config)
    dataset_dir = config.artifact_root / "datasets"
    report_dir = config.artifact_root / "reports"
    dataset_dir.mkdir(parents=True, exist_ok=True)
    report_dir.mkdir(parents=True, exist_ok=True)

    train_frames = extract_frames(config.train_run_id, config.protocol)
    test_frames = extract_frames(config.test_run_id, config.protocol)
    write_prefixed_frames(dataset_dir, "train", train_frames)
    write_prefixed_frames(dataset_dir, "test", test_frames)

    train_labeled = build_labeled_dataset(
        train_frames["observations"],
        train_frames["scam_labels"],
        config.run_config(config.train_run_id),
    )
    train_labeled["source_run_id"] = config.train_run_id
    train_labeled["oot_source_split"] = "train_range"
    train_labeled["split"] = split_labels(train_labeled, config.run_config(config.train_run_id))
    train_labeled.loc[train_labeled["split"] == "test", "split"] = "validation"

    test_labeled = build_labeled_dataset(
        test_frames["observations"],
        test_frames["scam_labels"],
        config.run_config(config.test_run_id),
    )
    test_labeled["source_run_id"] = config.test_run_id
    test_labeled["oot_source_split"] = "test_range"
    test_labeled["split"] = "test"

    train_pools = pool_key_set(train_labeled)
    before_test_rows = len(test_labeled)
    before_test_pools = len(pool_key_set(test_labeled))
    overlap = sorted(pool_key_set(test_labeled) & train_pools)
    if config.test_overlap_policy == "drop_test_pool_if_seen_in_train" and overlap:
        overlap_index = pd.MultiIndex.from_tuples(overlap, names=POOL_KEYS)
        test_index = pd.MultiIndex.from_frame(test_labeled[POOL_KEYS])
        test_labeled = test_labeled[~test_index.isin(overlap_index)].copy()

    frame = pd.concat([train_labeled, test_labeled], ignore_index=True)
    frame.to_csv(dataset_dir / "modeling_dataset.csv", index=False)
    if overlap:
        pd.DataFrame(overlap, columns=POOL_KEYS).to_csv(dataset_dir / "dropped_test_overlap_pools.csv", index=False)
    else:
        pd.DataFrame(columns=POOL_KEYS).to_csv(dataset_dir / "dropped_test_overlap_pools.csv", index=False)

    manifest = dataset_manifest(
        config,
        frame,
        train_labeled,
        test_labeled,
        before_test_rows,
        before_test_pools,
        overlap,
    )
    (report_dir / "oot_dataset_manifest.json").write_text(
        json.dumps(manifest, indent=2, sort_keys=True),
        encoding="utf-8",
    )
    print(
        "wrote {path} rows={rows} train={train} validation={validation} test={test} "
        "dropped_test_overlap_pools={overlap}".format(
            path=dataset_dir / "modeling_dataset.csv",
            rows=len(frame),
            train=int((frame["split"] == "train").sum()),
            validation=int((frame["split"] == "validation").sum()),
            test=int((frame["split"] == "test").sum()),
            overlap=len(overlap),
        )
    )


def extract_frames(run_id: str, protocol: str) -> dict[str, pd.DataFrame]:
    params = {"run_id": run_id, "protocol": protocol}
    return {
        "observations": db.read_sql(OBSERVATIONS_SQL, params),
        "scam_labels": db.read_sql(SCAM_LABELS_SQL, params),
        "event_evidence": db.read_sql(EVENT_EVIDENCE_SQL, params),
    }


def write_prefixed_frames(dataset_dir: Path, prefix: str, frames: dict[str, pd.DataFrame]) -> None:
    for name, frame in frames.items():
        frame.to_csv(dataset_dir / f"{prefix}_{name}.csv", index=False)


def pool_key_set(frame: pd.DataFrame) -> set[tuple[str, str]]:
    if frame.empty:
        return set()
    values = frame[POOL_KEYS].copy()
    values["token_address"] = values["token_address"].astype(str).str.lower()
    values["pool_address"] = values["pool_address"].astype(str).str.lower()
    return set(map(tuple, values.drop_duplicates().to_numpy()))


def dataset_manifest(
    config: OotExperimentConfig,
    frame: pd.DataFrame,
    train_labeled: pd.DataFrame,
    test_labeled: pd.DataFrame,
    before_test_rows: int,
    before_test_pools: int,
    overlap: list[tuple[str, str]],
) -> dict:
    holdout_rows = []
    if config.holdout is not None and not test_labeled.empty:
        mask = (
            test_labeled["token_address"].astype(str).str.lower().eq(config.holdout.token_address)
            & test_labeled["pool_address"].astype(str).str.lower().eq(config.holdout.pool_address)
        )
        holdout = test_labeled[mask].copy()
        holdout_rows = holdout[
            [
                "block_number",
                "time_to_scam_chain_block_delta",
                "pair_balance_backdoor_signal_seen_as_of",
                "control_transfer_from_holder_to_burn_seen_as_of",
                "control_transfer_from_after_renounce_seen_as_of",
            ]
        ].to_dict("records") if not holdout.empty else []
    return {
        "experiment_id": config.experiment_id,
        "protocol": config.protocol,
        "modeling_focus": config.modeling_focus,
        "train_run_id": config.train_run_id,
        "test_run_id": config.test_run_id,
        "ranges": {
            "train": config.train_range.__dict__,
            "gap": config.gap_range.__dict__,
            "test": config.test_range.__dict__,
        },
        "rows": {
            "total": int(len(frame)),
            "train": int((frame["split"] == "train").sum()),
            "validation": int((frame["split"] == "validation").sum()),
            "test": int((frame["split"] == "test").sum()),
            "test_before_overlap_drop": int(before_test_rows),
        },
        "pools": {
            "train": len(pool_key_set(train_labeled)),
            "test_before_overlap_drop": int(before_test_pools),
            "test_after_overlap_drop": len(pool_key_set(test_labeled)),
            "dropped_test_overlap": len(overlap),
        },
        "overlap_policy": config.test_overlap_policy,
        "dropped_test_overlap_pools": [
            {"token_address": token, "pool_address": pool} for token, pool in overlap[:100]
        ],
        "targets": {
            "chain_block_delta_horizons": config.chain_block_delta_horizons,
            "active_observation_delta_horizons": config.active_observation_delta_horizons,
        },
        "holdout": {
            "token_address": config.holdout.token_address,
            "pool_address": config.holdout.pool_address,
            "scam_block": config.holdout.scam_block,
            "rows": holdout_rows,
        } if config.holdout else None,
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--config", required=True)
    return parser.parse_args()


if __name__ == "__main__":
    main()
