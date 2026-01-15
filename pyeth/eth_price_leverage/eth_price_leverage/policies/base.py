from __future__ import annotations

from abc import ABC, abstractmethod
from typing import Any


class Policy(ABC):
    """Base interface for policies that select actions given a state."""

    @abstractmethod
    def select_action(self, state: Any) -> Any:
        raise NotImplementedError

