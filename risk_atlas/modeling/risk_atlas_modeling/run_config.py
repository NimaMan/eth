"""Run configuration loading for Risk Atlas modeling jobs."""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
import tomllib


MODELING_ROOT = Path(__file__).resolve().parents[1]


@dataclass(frozen=True)
class RunConfig:
    run_id: str
    protocol: str
    artifact_root: Path
    chain_block_delta_horizons: list[int]
    active_observation_delta_horizons: list[int]
    train_time_to_event: bool
    train_end_fraction: float
    validation_end_fraction: float
    baseline_models: list[str]


def load_config(path: str | Path) -> RunConfig:
    config_path = Path(path)
    if not config_path.is_absolute():
        config_path = MODELING_ROOT / config_path
    data = tomllib.loads(config_path.read_text(encoding="utf-8"))
    paths = data.get("paths", {})
    targets = data.get("targets", {})
    split = data.get("split", {})
    models = data.get("models", {})
    artifact_root = Path(paths.get("artifact_root", f"artifacts/{data['run_id']}"))
    if not artifact_root.is_absolute():
        artifact_root = MODELING_ROOT / artifact_root
    return RunConfig(
        run_id=str(data["run_id"]),
        protocol=str(data.get("protocol", "UNISWAP-V2")),
        artifact_root=artifact_root,
        chain_block_delta_horizons=[int(value) for value in targets.get("chain_block_delta_horizons", [1, 3, 5, 10])],
        active_observation_delta_horizons=[
            int(value) for value in targets.get("active_observation_delta_horizons", [1, 3, 5, 10])
        ],
        train_time_to_event=bool(targets.get("train_time_to_event", True)),
        train_end_fraction=float(split.get("train_end_fraction", 0.70)),
        validation_end_fraction=float(split.get("validation_end_fraction", 0.85)),
        baseline_models=[str(value) for value in models.get("baseline_models", ["logistic_regression"])],
    )
