from __future__ import annotations

from dataclasses import dataclass
from typing import Optional


@dataclass
class EnvConfig:
    tip_gwei: int = 1
    slippage_bps: int = 50
    block_step: int = 1
    reth_datadir: Optional[str] = None


@dataclass
class TrainingConfig:
    steps: int = 10
    seed: Optional[int] = None

