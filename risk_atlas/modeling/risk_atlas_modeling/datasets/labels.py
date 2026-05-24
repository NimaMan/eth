"""Label construction for scam horizon and time-to-event targets."""

from __future__ import annotations

import numpy as np
import pandas as pd

from .. import db
from ..run_config import RunConfig


POOL_KEYS = ["token_address", "pool_address"]


def build_labeled_dataset(
    observations: pd.DataFrame,
    scam_labels: pd.DataFrame,
    config: RunConfig,
) -> pd.DataFrame:
    run_bounds = db.fetch_run_bounds(config.run_id)
    obs = observations.copy()
    obs["token_address"] = obs["token_address"].str.lower()
    obs["pool_address"] = obs["pool_address"].str.lower()
    obs["block_number"] = pd.to_numeric(obs["block_number"], errors="coerce").astype("int64")
    obs = obs.sort_values([*POOL_KEYS, "block_number", "active_observation_index"]).reset_index(drop=True)
    obs["pool_observation_number"] = obs.groupby(POOL_KEYS).cumcount()

    labels = scam_labels.copy()
    if labels.empty:
        labels = pd.DataFrame(columns=[*POOL_KEYS, "scam_block"])
    labels["token_address"] = labels["token_address"].str.lower()
    labels["pool_address"] = labels["pool_address"].str.lower()
    labels["scam_block"] = pd.to_numeric(labels["scam_block"], errors="coerce")

    frame = obs.merge(labels[[*POOL_KEYS, "scam_block"]], how="left", on=POOL_KEYS)
    frame["has_scam_label"] = frame["scam_block"].notna()
    frame["pre_scam_observation"] = (~frame["has_scam_label"]) | (frame["block_number"] < frame["scam_block"])
    frame = frame[frame["pre_scam_observation"]].copy()
    frame["time_to_scam_chain_block_delta"] = frame["scam_block"] - frame["block_number"]
    frame.loc[~frame["has_scam_label"], "time_to_scam_chain_block_delta"] = np.nan
    frame["time_to_scam_observed"] = frame["time_to_scam_chain_block_delta"].notna()

    for horizon in config.chain_block_delta_horizons:
        target = f"scam_within_{horizon}_chain_block_delta"
        known = frame["time_to_scam_observed"] | (frame["block_number"] <= run_bounds["end_block"] - horizon)
        positive = frame["time_to_scam_chain_block_delta"].between(1, horizon, inclusive="both")
        frame[target] = np.where(known, positive.astype(int), np.nan)

    active_delta = _active_observation_delta_to_scam(obs, labels)
    frame = frame.merge(active_delta, how="left", on=[*POOL_KEYS, "block_number", "pool_observation_number"])
    for horizon in config.active_observation_delta_horizons:
        target = f"scam_within_{horizon}_active_observation_delta"
        enough_future = frame.groupby(POOL_KEYS)["pool_observation_number"].transform("max") >= (
            frame["pool_observation_number"] + horizon
        )
        known = frame["active_observation_delta_to_scam"].notna() | enough_future
        positive = frame["active_observation_delta_to_scam"].between(1, horizon, inclusive="both")
        frame[target] = np.where(known, positive.astype(int), np.nan)

    return frame


def _active_observation_delta_to_scam(observations: pd.DataFrame, labels: pd.DataFrame) -> pd.DataFrame:
    rows = []
    label_map = {
        (row.token_address, row.pool_address): row.scam_block
        for row in labels.itertuples(index=False)
        if pd.notna(row.scam_block)
    }
    for pool_key, group in observations.groupby(POOL_KEYS, sort=False):
        scam_block = label_map.get(pool_key)
        group = group.sort_values(["block_number", "pool_observation_number"])
        if scam_block is None:
            for row in group.itertuples(index=False):
                rows.append((*pool_key, row.block_number, row.pool_observation_number, np.nan))
            continue
        scam_positions = group.index[group["block_number"] >= scam_block].tolist()
        scam_position = int(group.loc[scam_positions[0], "pool_observation_number"]) if scam_positions else None
        for row in group.itertuples(index=False):
            delta = np.nan if scam_position is None else scam_position - int(row.pool_observation_number)
            rows.append((*pool_key, row.block_number, row.pool_observation_number, delta))
    return pd.DataFrame(
        rows,
        columns=[*POOL_KEYS, "block_number", "pool_observation_number", "active_observation_delta_to_scam"],
    )
