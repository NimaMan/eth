# PnL DB Assessment Criteria

The PnL store is still pre-v1. We should keep iterating until these questions
have clear answers from the data and code.

## Hard Correctness

These checks answer whether the ledger is internally coherent:

- Conservation: for every pool, token in equals token out and denom in equals
  denom out at raw-integer precision.
- Finite values: scaled convenience columns must not contain NaN or infinity.
- No movement persistence in aggregate runs: historical aggregate runs should
  write zero `pool_pnl_movements` rows.
- Closed-position realized PnL: addresses with near-zero token balance should
  have realized PnL computable from denom cashflow even when mark price is
  unavailable.
- Native cost handling: WETH-denom PnL should subtract native fees and native
  priority fees only where that cost is economically attributed to the address.

## Role And Entity Semantics

These checks answer whether rows represent users or raw chain addresses:

- How many rows are pool addresses, routers, zero/burn addresses, token
  contracts, known proxies, or other non-user infrastructure?
- What share of absolute PnL is explained by non-user rows?
- Which repeated intermediary contracts dominate top winners/losers?
- Do we need entity grouping for flows where one address pays gas, another pays
  denom, and a third receives/sells tokens?

## Valuation Quality

These checks answer whether unrealized PnL can be trusted:

- Which pools have valid latest token-state rows with priced valuation status?
- Which PnL rows are open token balances without a usable mark price?
- Which large unrealized PnL rows come from dust, drained, or
  liquidity-removed pools?
- What threshold should turn unrealized PnL into null/unstable instead of a
  misleading number?

## Retention And Historical Scope

These checks answer whether historical exports preserve the right terminal
state:

- Did retention flushes write state before tokens or pools were dropped?
- Which rows are final-run exports versus retention-drop exports?
- Did a run long enough to exceed the retention window actually drop tokens?
- Are historical scopes isolated from live latest state?

## DB Shape Questions

The answers to these questions determine the first proper PnL DB shape:

- Should user-facing PnL default to `is_user_candidate = true` while raw address
  rows remain queryable?
- Should realized and unrealized PnL be stored as separate columns?
- Should valuation status come from `token_state.pool_latest` instead of PnL?
- Do we need address-role rows per pool, or should role be materialized directly
  into `pool_address_pnl`?
- Do retention-drop snapshots need separate PnL segment tables, or is latest
  aggregate per historical scope enough for the first live integration?
