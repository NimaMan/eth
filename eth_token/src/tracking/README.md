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
- `token_update_router/token_candidates.rs` decides which tracked tokens should
  be visited for a processed transaction. It must stay conservative enough not
  to miss token/pool updates, but it is the hot path for range rebuilds.
- `token_update_router/v2_pool_candidate_router.rs` owns the V2-specific
  candidate fallback for unknown pair events. It first tries known pool mappings
  and ERC-20 transfers touching the pair address, then falls back to cheap V2
  pool identity lookup only when needed.
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

## Candidate Routing Requirement

For each processed transaction, the router must identify every tracked token
whose token state or pool state can change. Missing a candidate can leave
reserves, LP state, tax status, or tradability stale, which directly corrupts
strategy eligibility and backtest PnL. Visiting too many candidates makes range
rebuilds slow.

The routing order is:

```text
known token or known pool address in processed tx
  -> direct tracked-token candidate
V2 PairCreated
  -> use decoded token0/token1/factory from the event
unknown V2 Swap/Sync/Mint/Burn pair address
  -> use ERC-20 transfers touching the pair if they resolve to a tracked token
  -> otherwise read cheap V2 pool identity
V3/V4 pool events
  -> use decoded pool-created/initialize currencies and pool keys
```

V2 is special because later `Swap`, `Sync`, `Mint`, and `Burn` events expose the
pair address but not `token0`/`token1`. V3/V4 creation or initialize events carry
the token/currency addresses in the decoded event, so they do not need the same
pair identity fallback.

Candidate routing must not fetch full V2 metadata. It only needs pool identity:
`token0`, `token1`, and validated known protocol. Decimals belong to pool
registration, not candidate discovery. Known V2 identity validation should use
local CREATE2 pair-address calculation rather than a simulated factory
`getPair` call.

The chain metadata cache answers what a pool is. Candidate-level caches should
answer whether that pool can route to the current tracked-token set. Those are
different lifetimes: a "not currently relevant" candidate cache must be
invalidated when new tokens are indexed or retention changes the tracked set.
