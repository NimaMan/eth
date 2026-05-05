## Live Data Registry

Centralized helpers for reading Rust-published live block data and publishing
Python-owned token/position snapshots so every process shares the same Redis
key contract.

### Layout

- `redis_client.py` — cached sync/async Redis clients driven by `LIVE_BLOCKCHAIN_DATA_REDIS_URL`.
- `keys.py` — canonical key builders (`eth/live/block/<n>/header`, `eth/live/block/<n>/txs`, `eth/live/token/snapshot/<addr>`, …).
- `snapshot_serialization.py` — utilities for normalizing headers and serializing Python-owned snapshots.
- `publisher.py` — async writer used by token/position publishers. Rust owns block publication.
- `reader.py` — synchronous reader for consumers (mempool processor, notebooks, etc.).

Keep new shared live-state capabilities here so schema/behaviour stays consistent across services.
