## Live Data Registry

Centralized helpers for publishing and reading live chain-derived data
(blocks, token snapshots, portfolio positions) so every process
shares the same view without duplicating Redis plumbing.

### Layout

- `redis_client.py` — cached sync/async Redis clients driven by `LIVE_DATA_REDIS_URL`.
- `keys.py` — canonical key builders (`block:processed_block_snapshot:<n>`, `token:snapshot:<addr>`, …).
- `snapshot_serialization.py` — utilities for shaping/serialising block snapshots (token snapshots will live here too).
- `publisher.py` — async writer used by block/token processors.
- `reader.py` — synchronous reader for consumers (mempool processor, notebooks, etc.).

Keep new shared live-state capabilities here so schema/behaviour stays consistent across services.
