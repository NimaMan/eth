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
