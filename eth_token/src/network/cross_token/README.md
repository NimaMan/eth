# Cross-Token Operator Tracing

Detects repeat operators across multiple token networks.

## Problem

A single scammer may launch or manipulate multiple tokens. Our current
pipeline is strictly token-scoped, so the same operator appears as
unconnected graphs.

## Approach

1. Build an operator index keyed by address
2. For each address, store: tokens touched, first/last seen, behavior fingerprint
3. Detect repeat patterns: same funder, same intermediary, snipe-then-dump timing

## Modules

- `operator.rs` — Detect repeat operators across token graphs
- `overlap.rs` — Compute address overlap between token networks
