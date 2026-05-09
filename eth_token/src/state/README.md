# state

Token state aggregation and event trackers.

This folder corresponds to Python modules under `erc20_token/token_state`.

## Responsibilities

- Track token transfers and holder-level state.
- Track control-address activity and suspicious ownership/control behavior.
- Bridge pool state into token-level state.
- Maintain rolling state needed by live token updates.

## Boundaries

- This module consumes processed transaction events.
- It does not decode logs directly.
- Pool math belongs in `pools`.
- Health scoring belongs in `health`.

## Python Sources

1. `token_transfer_tracker.py`.
2. `control_address_tracker.py`.
3. `pool_state_bridge.py`.
4. `token_state_monitor.py` is represented by Rust `status.rs` and `TokenStatusManager`.
