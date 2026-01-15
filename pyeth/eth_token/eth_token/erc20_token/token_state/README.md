# Token Runtime State Modules

This package is the landing zone for the in-progress refactor of
`ERC20TokenData`.  The goal is to split the monolithic class into thin,
focused helpers while keeping **all behaviour identical**.  Nothing in
this directory introduces new logic yet—we are only preparing the
structure so that future mechanical moves are easy to reason about and
review.

## Module overview

| Module | Responsibility | Notes |
| --- | --- | --- |
| `token_data.py` | Final home of the slim `ERC20TokenData` orchestrator. It will instantiate the helper classes below and route each transaction to them in the same order used today. | Remains the single entry point referenced by `ERC20Token`. |
| `control_address_tracker.py` | Tracks who currently controls the token/pools (creator, current owner, AccessControl roles, proxy admins, Ownable2Step pending owners). | Will surface a "new addresses" list so both the token and the pool manager stay in sync. |
| `token_transfer_tracker.py` | Records all token-level activity: ERC20 transfers, internal ETH transfers, approvals, and the per-address counters that feed analytics. | Keeps the interface identical for `token_network`/`token_health` consumers. |
| `pool_state_bridge.py` | Thin wrapper around `eth_token.erc20_token.pools.*`. Routes transactions to `PoolManager`, refreshes cached price/liquidity data, and exposes helper getters. | Naming matches the existing `pools/` directory to avoid confusion with the Uniswap pool implementations. |
| `token_state_monitor.py` | Tracks contract-level toggles (trading enabled, taxes, buy-limits) and hidden-mint detection. Delegates advanced scoring to the pre-existing `token_health` package. | Prevents governance logic from leaking into the control/transfer modules. |

## Migration principles

- **Zero behavioural drift.** Code will be copy/pasted into the new
  modules verbatim before any clean-up is attempted.
- **Explicit dependencies.** Each helper should state which parts of the
  original `ERC20TokenData` surface it expects (e.g., references to
  `PoolManager`, `token_health`, or analytics structures).
- **Testable units.** Once the move is complete, each module can be
  exercised independently, making future changes safer.

Keep this README in sync as modules gain real implementations so
newcomers understand the layout at a glance.
