# Live Block Ingestion & Distribution

This document describes how the live-block pipeline is wired today: from
WebSocket notifications → block processing → Redis snapshots → RabbitMQ
broadcast → downstream consumers. It also clarifies where the optional
Reth index writer fits in.

## End-to-End Sequence

1. **WebSocket signal**  
   `LiveBlockProcessor` subscribes to `newHeads` over WebSocket. Every
   header notification is treated as a trigger to fetch and process the
   corresponding block.

2. **Block fetch & processing**  
   The processor uses the HTTP provider (`BlockProcessor` + `TxBatchProcessor`)
   to retrieve the full block body, execute transaction decoding, and build a
   `ProcessedBlockResult`. Any per-block analytics (stablecoins, taxes, etc.)
   happen at this step.

3. **Redis snapshot**  
   Once a block is processed, `_publish_live_block_snapshot` serialises the
   result via the **live data registry** and stores it under
   `live:block:<number>` plus `live:block:latest`. The snapshot includes:
   - canonical header (all roots, timestamps, base fee…)
   - transaction count
   - full processed transactions (JSON-normalised: sets → lists, big ints → str,
     bytes → hex)
   The registry keeps only the latest N blocks (default 3) to bound memory.

4. **RabbitMQ notification**  
   After the snapshot is written, the processor drops a minimal message
   (`{"block_number": N}`) onto the `blocks_exchange` FANOUT exchange.
   Each consumer maintains its own exclusive, auto-delete queue configured with
   `x-max-length = 1` so they always see the newest block number without
   accumulating history.

5. **Consumers hydrate from Redis**  
   `eth_token.token_manager.BlockSubscriber` (and any other consumer) receives
   the RabbitMQ message, reads the block number, and calls the live data
   registry’s `LiveDataReader.get_block_snapshot(number)` to fetch the full
   payload. That snapshot is then fed to `BlockTokenProcessor`, ensuring every
   consumer sees identical processed data regardless of when they started.

6. **Reth index writer (independent path)**  
   If `index_address_txs=True`, `LiveBlockProcessor` enqueues each processed
   block into `_index_worker`, which writes address/transaction mappings via
   `TransactionAddresstoTxIndexer`. This worker runs independently of the
   Redis/Rabbit flow so indexing delays cannot block snapshot publication.

## Why Redis *and* Rabbit?

| Component     | Responsibility                                               |
| ------------- | ------------------------------------------------------------ |
| **Redis**     | Authoritative store for the latest block snapshots. Any process can fetch or rehydrate block state on demand, even if it started late. |
| **RabbitMQ**  | Low-latency notification that “block N is ready”. Consumers react immediately and pull the actual payload from Redis. |

This split keeps the notification path tiny (block number only) while giving
every consumer a consistent, JSON-safe snapshot to read at their own pace.

## Queue Behaviour Recap

- Consumers create queues with unique names (e.g. `token_tracking_blocks_<uuid>`), `exclusive=True`, `auto_delete=True`.
- Queue arguments:  
  ```json
  {
    "x-max-length": 1,
    "x-overflow": "drop-head"
  }
  ```
  That guarantees only the latest block number is retained.
- Because the exchange is FANOUT, every consumer gets every notification.

## Block Subscriber Flow

1. RabbitMQ message arrives (`{"block_number": 23790000}`).
2. Subscriber calls `LiveDataReader.get_block_snapshot(23790000)`.
3. Snapshot is validated (must contain header + transactions).
4. Callback (or `BlockTokenProcessor`) receives the fully hydrated block.

No consumer needs direct access to the processing pipeline—Redis provides the
canonical payload and keeps the schema consistent across Python/Rust components.

## Future Extensions

The same registry infrastructure can be applied to other shared state such as
token snapshots or strategy positions. The contract stays the same:
publish once to Redis → broadcast a lightweight notification → let consumers
hydrate from the cache when needed.
