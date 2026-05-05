# health

Token health, volume analysis, and scam-risk signals.

This folder corresponds to Python modules under `erc20_token/token_health`.

## Responsibilities

- Define scam and health thresholds.
- Analyze token volume and liquidity behavior.
- Produce health/risk predictions from token and pool state.
- Keep scoring deterministic and testable from fixtures.

## Boundaries

- This module should not own state mutation from transactions.
- It should read snapshots or state summaries produced by `erc20`, `pools`, and `state`.
- Machine-learning or model-specific workflows should be explicit dependencies, not hidden inside block processing.

## First Port Targets

1. Threshold constants and config structs.
2. Volume analysis functions.
3. Health predictor output model.
