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
  `alpha/live/feed` and `eth_chain_server`.

## Layout

- `block_processor/` applies ordered processed blocks into the token registry.
  Its README documents the current intra-block simulation path and the target
  post-block simulation design for historical range indexing.
- `token_update_router/` applies one processed transaction into tracked token and
  pool state.
- `token_update_router/pool_discovery/` discovers known V2-router-compatible,
  Uniswap V3, and Uniswap V4 pools from processed events.
- `token_update_router/pool_discovery/known_v2.rs` is protocol-aware through
  `KnownV2Protocol`; it classifies Uniswap V2, SushiSwap V2, PancakeSwap V2,
  ShibaSwap V2, and Fraxswap V2 instead of treating every V2-shaped pool as
  Uniswap.
- `token_update_router/pool_discovery/uniswap_v3.rs` and
  `token_update_router/pool_discovery/uniswap_v4.rs` stay Uniswap-specific.
- `token_update_router/trading_status_update/` decides when to refresh pool
  trading-status checks and calls the existing simulator/pool APIs.
- `tracked_token_index.rs` and `live_token_retention.rs` own live tracked-token
  indexing and retention.
- `builders/` rebuilds single-token state from processed history.
- `replay_context/` owns token-control trigger policy for deciding when to
  refresh all known pools.
