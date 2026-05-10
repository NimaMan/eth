# Temporal Analysis

Analyzes the ordering of events to detect causal relationships.

## Problem

The graph stores *that* two addresses are connected, but not *when* or
*in what order*. A funder who sends ETH and then the recipient buys the
token in the next transaction is a very different signal than random
co-occurrence.

## Approach

1. Use `block_number` and `log_index` from edge evidence for ordering
2. Use `AddressBlockParticipationIndex` to load blocks where both addresses appear
3. Build temporal event sequences and match against known patterns

## Modules

- `ordering.rs` — Tx ordering within blocks (funded-then-bought, etc.)
- `windows.rs` — Sliding window coactivity detection
