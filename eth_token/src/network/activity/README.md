# activity

Per-address activity tracking for one token network.

This folder is the Rust replacement for the useful parts of Python
`AddressTokenActivityTracker`. It should aggregate movement counters and
derived features for addresses observed around a tracked token.

## Files

- `address.rs`: address-level activity state and counters.
- `movement.rs`: token, ETH, WETH, stable, and known-denom movement records.
- `pnl.rs`: lightweight realized/unrealized PnL proxy calculations.

## Boundaries

- Consumes normalized updates from `ingest` or existing token trackers.
- Does not classify clusters.
- Does not fetch prices directly; price input must come from pools or snapshots.
- Does not create Asena layouts.

## First Implementation Target

Port the Python counters that matter for launch analysis: token in/out, denom
in/out, buy/sell counts, fees, bribes, first/last seen block, and current balance
proxies.

## Current Contract

The activity layer now exposes:

- `AddressMovement`: one token, denomination, LP, or other movement from the
  perspective of a tracked address;
- `AddressMovementTotals`: running totals and event counts for token in/out,
  denom in/out, LP in/out, and other movements;
- `AddressCostRecord`: transaction fee and bribe costs attached to an
  observation;
- `AddressActivity`: bounded per-address movement/cost history plus unbounded
  running totals;
- `AddressActivitySummary`: Python-style feature snapshot for downstream graph,
  cluster, and Asena views;
- `AddressPnlProxy`: lightweight realized/unrealized/total profit proxy.

The running totals are intentionally not bounded by `history_limit`; only the
stored examples are bounded. This lets live summaries remain correct while memory
use stays predictable.
