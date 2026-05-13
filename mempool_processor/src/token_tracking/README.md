# Token Tracking

`token_tracking` gives the mempool processor a local, read-optimized view of
confirmed token, pool, and creator context. The mempool processor does not own
canonical token state; it consumes token-server/live-tracker state and uses it
to route pending transactions and interpret simulation results.

## Live Sources

Primary live source:

```text
eth_token live tracker
  -> eth_token_server live HTTP APIs
  -> GET /eth/tokens/api/live/tokens
  -> GET /eth/tokens/api/live/pools
  -> TokenTrackingCache
```

Update notification:

```text
eth_token_server applies confirmed block
  -> broadcasts LiveTokenEvent::BlockApplied in process
  -> /eth/tokens/api/live/updates long-poll returns
  -> mempool refreshes /live/tokens + /live/pools
```

The update endpoint is only a wakeup path. Token and pool context always comes
from `/live/tokens` and `/live/pools`. Redis token snapshots and token-update
ZMQ are intentionally not part of this module anymore.

Refresh sequence:

1. `mempool_signal_detector` fetches `/live/tokens` and `/live/pools`.
2. It records the accepted token-context block in `TokenTrackingCache`.
3. It opens `/live/updates?after_block=<accepted_block>&timeout_ms=30000`.
4. token-server returns immediately when a newer block is already applied or
   when the next `BlockApplied` runtime event fires.
5. mempool reloads `/live/tokens` and `/live/pools` and repeats the wait.

## Cache Context Rules

Live runs must never move token context backwards. `TokenTrackingCache` tracks
the last accepted context block, status, and source.

Accepted live-token-server snapshots must be:

- `status=warming` before any live context has been accepted, or `status=live`
- non-empty for startup hydrate success
- block number greater than or equal to the current accepted context block

Rejected snapshots are counted and logged with block/status/source:

- warming snapshots after a live context has already been accepted
- non-live/non-warming snapshots
- stale snapshots below the accepted block
- empty hydrate results during startup

These rules make token-server restarts visible and prevent a warming response
from overwriting usable live context.

## Hot-Path Lookups

The router and simulator use the cache for:

- `is_creator(address)`: classify known creator/owner/tax-setter calls.
- `get_token_for_creator(address)`: find the token affected by a creator call.
- `get_pool(pool_address_or_display_key)`: resolve known pool context.
- `get_token_for_pool(pool_address_or_display_key)`: map liquidity events back
  to token context.
- `get_pools_for_token(token)`: run per-pool buy/sell viability checks.

Pool identity can be either an EVM address or a V4 composite string:
`pool_manager#pool_id`. V4 composite identifiers must stay strings; callers
must not parse them as `Address`.

## Interaction With Unresolved Intents

The cache can legitimately miss when a pending tx arrives before the confirming
block has been processed by token-server. For critical txs, a miss should become
an unresolved intent instead of a public signal or simulation error.

Critical cache-wait examples:

- LP approval where the LP/pool token is not mapped yet.
- Liquidity removal where the token/pool cannot be resolved yet.
- Creator-control tx where the creator/target mapping is not present yet.
- V4 modify-liquidity tx whose composite pool id is not mapped yet.

Once the cache accepts a newer token-server snapshot, the unresolved-intent lane
retries those txs and routes them through the normal direct-publish or
simulation paths.

## Scam And Retention Semantics

The cache excludes scam pools from active routing while keeping the token entry
available for inspection and future context. Pool-level scam state should not
delete token-level history immediately.

Token-server remains authoritative for confirmed scam/pruning policy. The
mempool processor uses the resulting active/scam indexes only to decide whether
a pending tx is actionable, cache-waiting, or ignored.

## Logging And Metrics

Interval logs should include:

- token, pool, and creator counts
- accepted token-context block, status, and source
- stale/non-live snapshot rejection counters

Cache-wait misses should be represented through unresolved-intent metrics and
`unresolved_intents.log`, not as repeated simulation errors.

## Boundaries

This module should not:

- reconstruct canonical token/pool state from scratch
- duplicate tx decoding or tax math from `tx_processor`
- publish trading signals directly
- classify V4 composite ids as EVM addresses

It should provide stable, explicit context to the router, simulator, and signal
manager.
