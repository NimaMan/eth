# Mempool Management

This analysis asks whether a pool is managed through public mempool
transactions before it becomes unsafe. The trading use case is direct: if a
pool's creator or liquidity controller consistently uses public pending
transactions for management actions, a live strategy may have an actionable
exit window when a future LP approval or liquidity-removal transaction appears
in the mempool.

## Question

```text
For pools with liquidity-removal signals, how many had prior creator or LP
management activity visible through the public mempool?
```

This is deliberately different from asking whether the liquidity-removal signal
itself came from the mempool. The current live signal store emits
`liquidity_removal` only after seeing a pending transaction, so that weaker
definition would make every row true and would not tell us whether the pool is
usually managed in public.

## Flag Semantics

The lab row exposes these flags:

| Field | Meaning |
| --- | --- |
| `removal_tx_public_mempool` | The liquidity-removal tx was observed as a live mempool signal. |
| `creator_main_tx_public_mempool` | The pool had a prior `trading_enabled` signal before the first liquidity removal. |
| `lp_control_tx_public_mempool` | The pool had a prior `lp_position_approval` signal before the first liquidity removal. |
| `pool_mempool_managed_pre_removal` | The creator or LP controller used public mempool management before the first liquidity removal. |
| `pool_mempool_exit_signal_available` | A mempool liquidity-removal signal existed for this pool. |

The production flag should use the strict pre-removal definition:

```text
pool_mempool_managed = creator_main_tx_public_mempool
  OR lp_control_tx_public_mempool
```

The exit engine can still react to `pool_mempool_exit_signal_available` for any
pool, but that is a live event, not a persistent pool-behavior label.

## Current Data Source

The current implementation reads PostgreSQL `live_trading.signal_events` and
the typed detail tables:

- `trading_enabled`
- `lp_position_approval`
- `liquidity_removal`

The confirmed `eth_db.pools` and `eth_db.tokens` rows do not currently carry
creation/trading/scam tx hashes for these live pools, so this first analysis is
based on the canonical live mempool signal store. If creation, liquidity-add,
and ownership-control tx hashes are later persisted, this analysis should join
them to the Reth mempool-arrival index and extend the flag with confirmed
first-seen evidence.

## Run

From `blockchains/eth`:

```bash
risk_atlas/scam_analytics/mempool_management/analyze_mempool_management.py \
  --database-url postgresql://postgres:postgres@localhost:5432/eth_db \
  --output-dir risk_atlas/scam_analytics/artifacts/mempool_management
```

Outputs:

- `pool_mempool_management.csv`
- `latest_summary.md`

## Promotion Path

Once the lab numbers are stable, promote the strict flag into a backend-owned
feature, not a frontend calculation:

```text
live_trading.signal_events
  -> eth_token::token_analytics feature/read model
  -> eth_chain_server API surface
  -> alpha strategy risk/exit policy
```

The strategy should use:

- `pool_mempool_managed=true` as a pool behavior feature;
- live `liquidity_removal` or `lp_position_approval` mempool events as
  immediate exit triggers;
- the measured `seconds_lp_approval_to_liquidity_removal` distribution to set
  urgency and gas/priority policy.
