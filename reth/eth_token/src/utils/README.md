# utils

Generic helpers shared by token-state modules.

This folder corresponds to small Python utilities such as bounded history and numeric parsing.

## Responsibilities

- Bounded history helpers.
- Numeric parsing/conversion helpers.
- Small formatting or normalization helpers that have no domain ownership.

## Boundaries

- No block, transaction, pool, or token orchestration logic here.
- If a helper depends on domain state, it belongs in that domain module instead.

## First Port Targets

1. `bounded_history.py`.
2. `pools/numeric.py` numeric parsing helpers.
