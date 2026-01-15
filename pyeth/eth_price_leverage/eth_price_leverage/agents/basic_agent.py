from __future__ import annotations

from dataclasses import dataclass, field
from typing import List, Any

import pyreth as pr

from ..envs.stablecoin_env import StablecoinEnv, StepOutput


@dataclass
class Transition:
    state: Any
    action: pr.PyStablecoinAction
    reward: float
    next_state: Any
    info: dict


@dataclass
class Agent:
    env: StablecoinEnv
    policy: Any
    history: List[Transition] = field(default_factory=list)

    def step(self) -> Transition:
        state = self.env.state()[1]  # only need portfolio for simple policies
        action = self.policy.select_action(state)
        out: StepOutput = self.env.step(action)
        next_state = out.portfolio
        tr = Transition(state=state, action=action, reward=out.reward, next_state=next_state, info=out.info)
        self.history.append(tr)
        return tr

    def run(self, steps: int = 10) -> List[Transition]:
        return [self.step() for _ in range(steps)]

