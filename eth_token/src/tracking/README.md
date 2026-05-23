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
- Live warmup/tail orchestration and publication stay in
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
It must also account for tokens that have been discovered in `TokenRegistry`
before the index refresh runs, so the V2 candidate cache keys irrelevant-pool
entries by both token-index generation and registry token count.

## Live Retention

Live retention is owned by `tracked_token_index.rs` and
`live_token_retention.rs`. Its job is to keep the live registry and token index
bounded without deleting newly discovered launch surfaces before the tracker has
enough pool state to classify them.

Retention only runs through the explicit retention pass:

```text
BlockTokenProcessor::apply_index_retention_policy(current_block)
  -> TrackedTokenIndex::apply_live_retention_policy(...)
  -> LiveTokenRetentionPolicy::apply_to_token(...)
```

`index_registry_token(...)` does not run retention. Index refresh only updates
membership, LRU order, and pool-to-token mappings. This is intentional:
discovery and index refresh must not silently remove a just-created token or
pool before the block-level retention report records the decision.

### Default Policy

The default live policy is:

```text
min WETH-denom reserve:   0.1 WETH
min stable-denom reserve: 1000 stable units
min other-denom reserve:  0
drop tokens without pools after blocks:          disabled
drop tokens without retained pools after blocks: disabled
retain liquidity-removal pools for blocks:       15000
```

WETH denoms currently include canonical WETH. Stable denoms include USDC, USDT,
and DAI. Other denoms default to a zero threshold, so they are retained unless a
separate token-drop policy is configured.

### Pool Decisions

Each pool is evaluated from its `BasePool`:

- If the pool has liquidity-removal evidence, it is retained until the
  liquidity-removal retention window expires.
- If the pool has no observed liquidity state yet, it is retained. A pool is
  considered observed once it has `latest_block_number` or a reserve snapshot.
- If the denom threshold is zero or lower, it is retained.
- If the observed denom reserve is at or above the denom threshold, it is
  retained.
- Otherwise it is dropped as `BelowDenomThreshold`.

The unobserved-pool rule is important for launch correctness. V2, V3, and V4
pool creation/initialize events can be decoded before the tracker has seen a
reserve update or trading simulation result. Retention must keep those pools so
later `Mint`, `Sync`, `Swap`, V3 position events, V4 liquidity events, or
simulation updates can attach state to the same pool.

### Liquidity-Removal Evidence

Before evaluating a token, retention scans every pool and marks liquidity
removal evidence when:

- the pool has a reserve snapshot,
- the current denom reserve is below the policy threshold,
- the max observed denom reserve was at or above the threshold,
- the drop from max observed reserve is at least 80%.

Marked liquidity-removal pools are retained for
`retain_liquidity_removal_pools_for_blocks`, currently `15000` blocks by
default. After that window expires, the pool is dropped with
`LiquidityRemovalRetentionExpired`. If that was the token's last retained pool,
the token can also be dropped for the same reason.

### Token Decisions

Token retention is derived from pool retention:

- If any pool is retained, the token is retained.
- If the token has no pools and `drop_tokens_without_pools_after_blocks` is
  configured, the token can be dropped after that many blocks from its latest
  token reference block.
- If the token has pools but none are retained and
  `drop_tokens_without_retained_pools_after_blocks` is configured, the token can
  be dropped after that many blocks from its latest pool reference block.
- If a liquidity-removal retention window expired and no pool remains retained,
  the token can be dropped with `LiquidityRemovalRetentionExpired`.

By default, the two "drop tokens without ..." windows are disabled. That means a
live token is not dropped merely because it has no pools or no retained pools
unless the configured policy explicitly enables that behavior.

### Applying Drops

When retention drops a pool, it removes that pool key from every pool map on the
token: V2, V3, V4, Curve, and Balancer. The field names in
`LiveTokenRetentionDecision` still use the legacy `retained_v2_pools` and
`dropped_v2_pools` names, but they contain all pool kinds.

When retention drops a token, it removes the token from both `TokenRegistry` and
`TrackedTokenIndex`. When a retained token loses pools, the index refreshes the
pool-to-token mapping so stale pool keys no longer route future transactions.

### Required Invariants

- Pool discovery must happen before pool retention can remove a pool.
- Index refresh must not invoke retention.
- A newly discovered pool with no observed state must be retained.
- Retention reports are the only place a retention drop should be counted.
- Candidate caches must be pruned when token-index membership changes.
