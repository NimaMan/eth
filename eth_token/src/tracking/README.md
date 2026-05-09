# tracking

Token state tracking over processed Rust blocks and transactions.

This is the Rust replacement for the old Python `token_manager` and
`token_builder` responsibilities. It owns token registry state, tracked-token
indexes, block application, transaction application, token/pool discovery
decisions, and live retention.

## Boundaries

- Input is `tx_processor::ProcessedBlock` / `ProcessedTransaction`; raw block and
  raw transaction processing stays in `tx_processor`.
- Chain reads and metadata hydration stay in `eth_token::chain_metadata`.
- Pool state machines and trading-status outcome application stay in
  `eth_token::pools`.
- Actual buy/sell simulation execution stays in `tx_processor::simulator`.
- Live warmup/tail orchestration, Redis, and publication stay in
  `alpha/live/feed` and `eth_token_server`.

## Layout

- `block_processor/` applies ordered processed blocks into the token registry.
- `transaction_applier/` applies one processed transaction into tracked token and
  pool state.
- `transaction_applier/discovery/` discovers V2/V3/V4 pools from processed
  events.
- `transaction_applier/trading_status.rs` decides when to refresh pool
  trading-status checks and calls the existing simulator/pool APIs.
- `index/` and `retention.rs` own live tracked-token indexing and retention.
- `builders/` rebuilds single-token state from processed history.
- `replay_context/` tracks same-block prior transactions needed by simulation
  replay.

`eth_token::manager` is now only a compatibility facade. New code should import
from `eth_token::tracking`.
