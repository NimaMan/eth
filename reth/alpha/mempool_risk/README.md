# Mempool Risk

Planned crate: `eth_mempool_risk`

This crate analyzes pending transactions and emits speculative risk signals. It should stay separate from the confirmed live feed.

## Responsibilities

- Poll or subscribe to mempool transactions.
- Classify transactions relevant to tracked tokens, pools, creators, owners, and tax setters.
- Use `tx_simulator` / `LiveTxSimulator` against the latest tracked live state.
- Detect pending liquidity removals, tax changes, honeypot behavior, trading-status changes, and other threats.
- Emit `RiskEvent`s or `MempoolSignal`s.

## Non-Responsibilities

- No canonical token state mutation.
- No confirmed block projection.
- No portfolio ownership.
- No order submission.
- No signing.

## State Sources

Reads:

```text
eth_live_state chain-state snapshots
eth_live_state token/pool snapshots
reth/MDBX fallback through tx_simulator
mempool transaction feed
```

Writes:

```text
eth/live/risk/...
eth/live/mempool/signals/...
database/log sinks for risk analytics
```

It must not write:

```text
eth/live/token/snapshot/...
eth/live/block/...
eth/live/latest/...
```

## Why Separate From Live Feed

Confirmed blocks are ordered and canonical. Mempool transactions are speculative, bursty, and CPU-heavy to simulate. Keeping this crate separate lets risk processing lag, scale, or restart without corrupting confirmed token state.

## Lessons From Current Code

The existing mempool processor already hydrates token state from Redis and uses live chain overlays for ahead-of-MDBX simulation. Keep that dependency direction:

```text
live_feed writes live state
mempool_risk reads live state
mempool_risk emits risk
alpha_engine decides what to do with risk
```
