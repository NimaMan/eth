# Token Manager – Code Map

This README documents the modules inside `eth_token/token_manager`.  
The centerpiece is `block_token_processor.py`, which mutates ERC‑20 state whenever a new block
arrives. Surrounding components feed it live data, expose queues to downstream consumers, and keep
recent token objects cached.

## Module Roles

| File | Purpose |
| ---- | ------- |
| `block_token_processor.py` | Deterministic per-block mutation engine for ERC‑20 tokens. Walks transactions sequentially, handles contract creation instantly, mutates existing tokens in order, and refreshes the shared `LiveTokensCache`. |
| `live_block_token_processor.py` | Async wrapper that warms up the cache, subscribes to the live block feed, monitors updated tokens, and exposes `unprocessed_token_updates` / `new_updates_event` to other services (e.g. `LiveTokenTracker`). |
| `block_subscriber.py` | `LiveBlockSnapshotSubscriber`: listens to the `block_published_notifier` exchange, hydrates block snapshots via `RedisSnapshotReader`, and forwards them to the processor callback. |
| `live_tokens_cache.py` | LRU cache of active `ERC20Token` objects with pool-to-token mapping, optional PnL persistence, and eviction policies. Serves both the processor and any consumers needing quick lookups. |

## Data Flow

```
┌────────────────────────────┐       snapshot write          ┌─────────────────────────────┐
│ eth_data LiveBlockProcessor│ ─────────────────────────────► │ Redis (LiveDataPublisher)   │
│  • newHeads                │                               └──────────────┬──────────────┘
│  • ProcessedBlockResult    │ block number msg                               │
└────────────┬───────────────┘                                               ▼
             │                                                  ┌──────────────────────────────┐
             ▼                                                  │ block_published_notifier (fanout)│
┌────────────────────────────┐                                  └──────────────┬──────────────┘
│ block_subscriber.BlockSub… │ ◄───────────────────────────────────────────────┘
│  • queue per consumer      │
│  • resolve block via Redis │
└────────────┬───────────────┘
             ▼
┌────────────────────────────┐
│ live_block_token_processor │
│  • process_block_tokens    │
│  • enqueue updated_tokens  │
└────────────┬───────────────┘
             ▼
┌────────────────────────────┐
│ LiveTokenTracker (outside) │
│  • consumes queue          │
│  • publishes pools/strategies
└────────────────────────────┘
```

## BlockTokenProcessor Event Sequence

1. **Block ingestion**
   - `process_block_tokens` receives the Redis snapshot payload.
   - Every transaction is normalized via `_ensure_tx_dict` and annotated with the current and
     previous headers so downstream logic has context.
2. **Sequential processing**
   - The processor iterates `block_tx_list` in order and calls `_process_transaction` for each entry.
   - The `updated_tokens` dict is cleared before the block and only contains addresses touched in
     the current block when processing finishes.
3. **Token creation (`_handle_token_creation`)**
   - If a transaction deploys an ERC‑20 contract, metadata is pulled through `TokenChainDataFetcher`
     and an `ERC20Token` object is instantiated immediately.
   - The creation transaction is replayed on the new token, the token is inserted into
     `LiveTokensCache`, and it is marked in `updated_tokens`. At this point `token.has_pool` is still
     false, but any pool creation events later in the block can now see the token.
4. **Existing token updates (`_handle_token_update_from_transaction`)**
   - Uses the `erc20_contracts` set embedded in each processed transaction to know which token (or
     pool) was touched. Because `LiveTokensCache.__getitem__` understands pool addresses, pool
     transactions are immediately routed to the owning token.
   - `_update_token` applies the transaction to the `ERC20Token`, keeping pool state, control and tax
     monitors, transfer trackers, etc. up to date.
   - After each mutation the token is remembered in `updated_tokens` and pool mappings are refreshed
     via `LiveTokensCache.update_pool_mapping(token)`. Updating on every transaction ensures that if
     a pool is created midway through a block, later transactions that reference the pool can be
     resolved correctly.
5. **Block completion**
   - Once the final transaction is processed, `process_block_tokens` records `processed_blocks` and
     returns. `updated_tokens` now holds the token objects to be published downstream.

## LiveBlockTokenProcessor Responsibilities

1. **Warmup** (`HistoricalBlockTokenProcessor.process_range_until_live`)
   - Replays a configurable number of historical blocks to hydrate `LiveTokensCache` before live
     tracking starts. While this runs, `process_block_tokens` behaves exactly as described above.
2. **Live subscription** (`LiveBlockSnapshotSubscriber.start`)
   - Starts the RabbitMQ consumer described earlier and routes each live block snapshot to
     `process_block_live`, which simply calls `process_block_tokens` and fires `block_processed_event`.
3. **Update monitoring** (`_monitor_token_updates`)
   - Waits for `block_processed_event`, copies the `updated_tokens` dict for that block, enqueues
     `(block_number, updated_tokens_snapshot)` into `unprocessed_token_updates`, and sets
     `new_updates_event`.
   - Consumers (e.g. `LiveTokenTracker._process_token_updates`) await that event and drain the queue.
4. **PnL scheduling (optional)**
   - If `add_pnl_to_db` is enabled, it dispatches async jobs that call
     `LiveTokensCache._write_token_pnl(token_address)` per updated token.

## LiveTokensCache Highlights

- Wraps an `OrderedDict` for LRU eviction plus a pool-to-token map so pool addresses can be used as
  lookup keys.
- Writes PnL to the DB (via `TokenPnLWriter`) before evicting entries when `add_pnl_to_db=True`.
- Provides helpers that the publisher layer uses to format full snapshots or incremental diffs.

## End-to-End Summary

1. `LiveBlockProcessor` publishes the `ProcessedBlockResult` snapshot to Redis and the block number
   to RabbitMQ.
2. `LiveBlockSnapshotSubscriber` consumes those numbers, fetches the full snapshot, and hands it to
   `LiveBlockTokenProcessor.process_block_live`.
3. `BlockTokenProcessor` walks the transactions sequentially, creating new tokens as needed and
   updating existing ones. `updated_tokens` ends up containing the token objects mutated during the
   block.
4. `_monitor_token_updates` snapshots those mutated tokens and puts them on an asyncio queue so
   higher-level components (strategy engines, publishers, writers) can react without touching Redis
   or RabbitMQ directly.
