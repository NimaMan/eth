# src

Source tree for the Rust token-state implementation.

The module layout mirrors the current Python package at `pyeth/eth_token/eth_token`, but the Rust crate should use Rust ownership boundaries rather than copying Python file-by-file. Shared event decoding and transaction models should be imported from existing Rust crates instead of recreated here.

## Module Boundaries

- `erc20`: token metadata, token snapshots, and token-level chain data helpers.
- `pools`: AMM pool state machines and pool-specific calculations.
- `state`: token transfer state, control-address tracking, and pool-state bridges.
- `health`: scam/volume/trading-health scoring.
- `network`: token address activity and graph construction.
- `manager`: block-level orchestration over processed Rust transactions.
- `utils`: generic helpers with no domain ownership.

## Rule

If a module needs receipt logs, traces, or decoded transaction fields, it should consume `tx_processor` data structures. It should not fetch, trace, or decode raw Ethereum transactions itself.
