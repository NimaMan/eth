# eth_token_server

Agent operating map for the runtime/API layer around `eth_token`.

## Purpose

- Host range and live token tracking in process memory.
- Warm from the processed-block disk cache and tail live processed blocks.
- Expose read-only HTTP/SSE views for tokens, pools, live status, mempool
  signals, and alpha-facing market inputs.

## Owns

- Process lifetime, config loading, logs, warmup/live-tail orchestration, and
  in-memory `TokenRegistry` / `TrackedTokenIndex` hosting.
- HTTP view DTOs for token lists/details, pool rows, live status, retention,
  and mempool signal reads.
- Processed-block cache read/fill path used by server runs.
- Live tracker status and terminal failure reporting.

## Does Not Own

- Core token/pool state rules; use `eth_token`.
- Raw transaction/block processing; use `tx_processor`.
- Mempool signal creation; use `mempool_processor`.
- Strategy decisions; use `alpha`.
- Snapshot persistence; a tracking run disappears when the process or run ends.

## Data Flow

```text
processed-block disk cache + Redis eth/live/blocks
  -> eth_live_feed warmup/live tail
  -> eth_token block application
  -> in-memory token/pool views
  -> HTTP/SSE clients, mempool context, alpha polling
```

## Where To Look First

| Need | Start here |
| --- | --- |
| Public server crate exports | `src/lib.rs` |
| Runtime config | `src/config.rs`, `../config.env` |
| Live token tracker host | `src/live.rs` |
| HTTP routes/server | `src/server/`, `src/main.rs` |
| Token/pool DTOs | `src/views/token.rs`, `src/views/pool.rs`, `src/views/live.rs` |
| Token active-block lookup | `src/views/activity.rs` |
| Mempool signal endpoint | `src/mempool_signals.rs` |
| Processed-block cache sizing | `examples/processed_block_disk_cache_size.rs` |

## Tests And Commands

```bash
RUST_LOG=info cargo run -p eth_token_server
curl -s http://127.0.0.1:8765/health
curl -s http://127.0.0.1:8765/live/status
cargo run -p eth_token_server --example processed_block_disk_cache_size -- --fill-missing-then-read
```

Key config defaults come from `config.env`:

```bash
RETH_DATADIR=/home/nima/storage/samsung8tb/ethereum/reth
RETH_INDEX_DIR=/home/nima/storage/samsung8tb/ethereum/reth/reth_index
TOKEN_SERVER_BIND=127.0.0.1:8765
TOKEN_SERVER_AUTO_START_LIVE=true
PROCESSED_BLOCK_DISK_CACHE_DIR=/home/nima/storage/samsung8tb/ethereum/processed-block-cache
LIVE_BLOCKCHAIN_DATA_REDIS_URL=redis://localhost:6379/0
ETH_PROCESSED_BLOCK_STREAM=eth/live/blocks
MEMPOOL_DATABASE_URL=postgresql://postgres:postgres@localhost:5432/eth_db
```

## Current Hazards

- The server is a read gateway, not the owner of token logic. Put token state
  rules in `eth_token`, then expose them here.
- The processed-block cache is shared replay input, not a token-server snapshot.
- `GET /runs/:id/tokens/:address` may include raw cloned token state for
  debugging; production UI should prefer explicit summary/pool DTOs.
- V4 pools use a composite `pool_address` of `pool_manager#pool_id` because V4
  pools are not ERC-20-style pool contracts.
- `GET /tokens/:address/activity-blocks` uses the custom RethIndex
  address-participation table. It expects an ERC-20 token address; a Uniswap v4
  pool id from Dexscreener is a 32-byte identifier and is not queryable as an
  address.
- Set `TOKEN_SERVER_AUTO_START_LIVE=false` for range/detail API work when live
  tailing is not needed.
