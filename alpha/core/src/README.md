# src

Source tree for `eth_alpha_core`.

This crate is the pure trading domain. It defines types and traits shared by live trading, backtesting, strategies, risk, persistence, and execution adapters.

## Module Boundaries

- `ids`: strongly typed identifiers and Ethereum primitive aliases.
- `time`: block/time markers.
- `amount`: on-chain amounts and decimal ratios.
- `market`: confirmed token/pool market events and snapshots.
- `risk`: speculative risk events and risk policy outputs.
- `order`: order intent and order lifecycle.
- `execution`: execution reports and execution adapter trait.
- `position`: position state machine and snapshots.
- `portfolio`: portfolio state, limits, and metrics.
- `strategy`: strategy context, decision, and trait.
- `store`: persistence trait only.
- `error`: core error types.

## Rule

No Redis, PostgreSQL, ZMQ, block fetching, transaction signing, or service orchestration belongs here.
