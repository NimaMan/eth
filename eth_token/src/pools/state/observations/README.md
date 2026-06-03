# Observations

Observation structs describe raw facts before they are folded into track state.
They are intentionally narrow: event-backed AMM activity, simulation outcomes,
custody findings, PnL counterparty roles, and liquidity snapshots.

These models do not replace current ingestion code. They provide a typed target
for future adapters and fixtures while `BasePool` remains the live projection
input.
