"""Out-of-time experiment configuration for Risk Atlas modeling."""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
import tomllib

from .run_config import MODELING_ROOT, RunConfig


@dataclass(frozen=True)
class BlockRange:
    start_block: int
    end_block: int
    block_count: int


@dataclass(frozen=True)
class HoldoutPool:
    token_address: str
    pool_address: str
    scam_block: int


@dataclass(frozen=True)
class OotExperimentConfig:
    experiment_id: str
    protocol: str
    modeling_focus: str
    artifact_root: Path
    experiment_root: Path
    train_run_id: str
    test_run_id: str
    train_range: BlockRange
    gap_range: BlockRange
    test_range: BlockRange
    holdout: HoldoutPool | None
    chain_block_delta_horizons: list[int]
    active_observation_delta_horizons: list[int]
    train_time_to_event: bool
    train_end_fraction: float
    validation_end_fraction: float
    test_overlap_policy: str
    baseline_models: list[str]
    planned_iterations: int
    current_iteration: int

    def run_config(self, run_id: str, artifact_root: Path | None = None) -> RunConfig:
        return RunConfig(
            run_id=run_id,
            protocol=self.protocol,
            artifact_root=artifact_root or self.artifact_root,
            chain_block_delta_horizons=self.chain_block_delta_horizons,
            active_observation_delta_horizons=self.active_observation_delta_horizons,
            train_time_to_event=self.train_time_to_event,
            train_end_fraction=self.train_end_fraction,
            validation_end_fraction=self.validation_end_fraction,
            baseline_models=self.baseline_models,
        )


def load_oot_config(path: str | Path) -> OotExperimentConfig:
    config_path = Path(path)
    if not config_path.is_absolute():
        config_path = MODELING_ROOT / config_path
    data = tomllib.loads(config_path.read_text(encoding="utf-8"))
    paths = data.get("paths", {})
    runs = data.get("atlas_runs", {})
    ranges = data.get("ranges", {})
    targets = data.get("targets", {})
    split = data.get("split", {})
    models = data.get("models", {})
    iterations = data.get("iterations", {})
    artifact_root = _rooted_path(paths.get("artifact_root", f"artifacts/{data['experiment_id']}"))
    experiment_root = _rooted_path(paths.get("experiment_root", f"experiments/{data['experiment_id']}"))
    test_range = _range(ranges["test"])
    holdout = None
    if ranges.get("test", {}).get("holdout_pool_address"):
        holdout = HoldoutPool(
            token_address=str(ranges["test"]["holdout_token_address"]).lower(),
            pool_address=str(ranges["test"]["holdout_pool_address"]).lower(),
            scam_block=int(ranges["test"]["holdout_scam_block"]),
        )
    return OotExperimentConfig(
        experiment_id=str(data["experiment_id"]),
        protocol=str(data.get("protocol", "UNISWAP-V2")),
        modeling_focus=str(data.get("modeling_focus", "out_of_time_scam_probability")),
        artifact_root=artifact_root,
        experiment_root=experiment_root,
        train_run_id=str(runs["train_run_id"]),
        test_run_id=str(runs["test_run_id"]),
        train_range=_range(ranges["train"]),
        gap_range=_range(ranges["gap"]),
        test_range=test_range,
        holdout=holdout,
        chain_block_delta_horizons=[
            int(value) for value in targets.get("chain_block_delta_horizons", [1, 2, 3, 4])
        ],
        active_observation_delta_horizons=[
            int(value) for value in targets.get("active_observation_delta_horizons", [])
        ],
        train_time_to_event=bool(targets.get("train_time_to_event", False)),
        train_end_fraction=float(split.get("train_end_fraction", 0.80)),
        validation_end_fraction=float(split.get("validation_end_fraction", 1.00)),
        test_overlap_policy=str(split.get("test_overlap_policy", "drop_test_pool_if_seen_in_train")),
        baseline_models=[
            str(value)
            for value in models.get(
                "baseline_models",
                ["logistic_regression", "decision_tree", "random_forest", "hist_gradient_boosting"],
            )
        ],
        planned_iterations=int(iterations.get("planned_count", 1)),
        current_iteration=int(iterations.get("current_iteration", 1)),
    )


def _rooted_path(path: str) -> Path:
    value = Path(path)
    return value if value.is_absolute() else MODELING_ROOT / value


def _range(data: dict) -> BlockRange:
    return BlockRange(
        start_block=int(data["start_block"]),
        end_block=int(data["end_block"]),
        block_count=int(data["block_count"]),
    )
