# Observations

Observation structs describe raw facts before they are folded into track state.
They are intentionally narrow: event-backed AMM activity, simulation outcomes,
custody findings, PnL counterparty roles, liquidity snapshots, and behavior-risk
scanner/simulation findings.

These models do not replace current ingestion code. They provide a typed target
for future adapters and fixtures while `BasePool` remains the live projection
input. The behavior-risk observation models are placeholders for verified-source
scanners, bytecode/function-signature classifiers, state-read adapters, traces,
and trade simulation summaries.
