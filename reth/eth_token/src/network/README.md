# network

Token network and address activity graph construction.

This folder corresponds to Python modules under `erc20_token/network`.

## Responsibilities

- Track address activity involving a token.
- Build token transfer and interaction graphs.
- Support live token network updates from processed blocks.
- Provide graph summaries for downstream analytics.

## Boundaries

- This module consumes transfer/control/pool events from token state.
- It should not fetch historical blocks directly.
- Live updates should be driven by `manager` orchestration.

## First Port Targets

1. Address activity tracker.
2. Token network data model.
3. Historical and live network builder APIs.
