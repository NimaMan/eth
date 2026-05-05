# tests

Integration and parity tests for the Rust token-state crate.

## Test Strategy

- Use fixture `ProcessedTransaction` and `ProcessedBlock` values from `tx_processor`.
- Compare Rust token/pool snapshots against known Python outputs during migration.
- Prefer deterministic unit tests for state transitions and health scoring.
- Keep live Reth database tests behind explicit integration flags.

## First Test Targets

1. Pool state transition fixtures for Uniswap V2.
2. Token transfer tracker fixtures.
3. Block token processor fixture with one processed block.
4. Snapshot serialization compatibility tests.
