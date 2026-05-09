# Ethereum Token Server

`eth_token_server` is the runtime/API layer around `eth_token`.

It runs token tracking over requested block ranges, keeps the resulting
`TokenRegistry` and `TrackedTokenIndex` in process memory, and exposes read-only
HTTP/SSE views for an inspector frontend.

It intentionally does not persist snapshots. A tracking run disappears when the
server stops or when the run is removed in a later lifecycle endpoint.

## Run

```bash
cd /home/nima/code/crypto/blockchains/eth
RUST_LOG=info cargo run -p eth_token_server
```

Runtime configuration is read from the shared workspace config file:
`/home/nima/code/crypto/blockchains/eth/config.env`. Set `ETH_CONFIG_PATH` only
when running against a different config file.

Token server keys:

- `RETH_DATADIR=/home/nima/storage/samsung8tb/ethereum/reth`
- `TOKEN_SERVER_BIND=127.0.0.1:8765`
- `TOKEN_SERVER_LOG_DIR=/home/nima/code/crypto/blockchains/eth/logs/eth_token_server`
- `TOKEN_SERVER_HISTORY_LIMIT=1000`
- `TOKEN_SERVER_DEFAULT_BLOCKS=7000`
- `SIMULATOR_LOG_DIR=/home/nima/code/crypto/blockchains/eth/logs/simulators`
- `LIVE_TOKEN_TRACKER_WARMUP_BLOCKS=7000`
- `LIVE_TOKEN_TRACKER_BLOCK_APPLY_TIMEOUT_MS=3000`
- `LIVE_TOKEN_TRACKER_PROCESSED_BLOCK_DISK_CACHE_RETRY_ATTEMPTS=20`
- `LIVE_TOKEN_TRACKER_PROCESSED_BLOCK_DISK_CACHE_RETRY_DELAY_MS=100`
- `LIVE_TOKEN_TRACKER_STREAM_BLOCK_MS=5000`
- `LIVE_TOKEN_TRACKER_STREAM_COUNT=100`
- `LIVE_BLOCKCHAIN_DATA_REDIS_URL=redis://localhost:6379/0`
- `ETH_PROCESSED_BLOCK_STREAM=eth/live/blocks`
- `PROCESSED_BLOCK_DISK_CACHE_DIR=/home/nima/storage/samsung8tb/ethereum/processed_block_disk_cache`
- `PROCESSED_BLOCK_DISK_CACHE_BLOCKS=1000000`
- `MEMPOOL_DATABASE_URL=postgresql://postgres:postgres@localhost:5432/eth_db`
- `MEMPOOL_SIGNAL_LIMIT=200`

`TOKEN_SERVER_LOG_DIR` contains two token-server log streams:

- `eth_token_server.log.<date>`: the full token server runtime log.
- `live_token_tracker.log.<date>`: focused JSON records for live token tracker warmup, live-tail block apply, metadata/simulation timeouts, and terminal failures.

To override the traced `ProcessedBlock` disk cache location, set
`PROCESSED_BLOCK_DISK_CACHE_DIR` in `config.env`.

The cache stores a sparse token-analysis subset of traced `ProcessedBlock`s as
JSON compressed with `zstd`: block header, only non-empty token-relevant
processed transaction fields, original calldata/gas replay fields, and
per-transaction processing errors. Full traces, receipts, raw metadata, state
maps, empty event families, and unrelated decoded event families are not
retained. Runs acquire cached blocks in 250-block chunks, so only one cache
read batch is retained before those blocks are applied to the token tracker in
block order.

To measure the same fill-missing-then-read path used by server runs:

```bash
cargo run -p eth_token_server --example processed_block_disk_cache_size -- \
  --cache-dir /home/nima/storage/samsung8tb/ethereum/processed_block_disk_cache \
  --start 25036824 \
  --end 25036825 \
  --fill-missing-then-read
```

## API

- `GET /health`
- `GET /runs`
- `POST /runs`
- `GET /runs/:id/progress`
- `GET /runs/:id/tokens`
- `GET /runs/:id/tokens/:address`
- `GET /runs/:id/pools`
- `GET /runs/:id/errors`
- `GET /runs/:id/stream`
- `POST /runs/:id/stop`
- `GET /live/status`
- `POST /live/start`
- `POST /live/stop`
- `GET /live/tokens`
- `GET /live/tokens/:address`
- `GET /live/pools`
- `GET /live/retention`

Example:

```bash
curl -s http://127.0.0.1:8765/runs \
  -H 'content-type: application/json' \
  -d '{"block_count":7000}'
```

## Frontend Data Contract

The inspector should use the view DTOs exposed by this server:

- `GET /runs/:id/tokens` returns `TokenListResponse` with compact `TokenView` rows for the token table.
- `GET /runs/:id/tokens/:address` returns `TokenDetailResponse` with:
  - `token`: full cloned `ERC20Token` state for debugging and exploratory inspection.
  - `summary`: compact token summary.
  - `pools`: frontend-facing `PoolView` rows for the token's tracked pools.
- `GET /runs/:id/pools` returns all tracked pools as the same `PoolView` rows.
- `GET /live/tokens` and `GET /live/pools` return the same token and pool row DTOs for the live tracker.

Prefer the explicit `pools` array for UI rendering. The raw `token.v2_pools`
shape is internal Rust state and may change faster than the view layer.

Current `PoolView` pool-page fields include reserves, price, liquidity,
buy/sell/tax status, scam state, creation/can-buy blocks, runtime state, and
V2 LP state. The LP fields are intended for the token detail page:

- `lp_total_supply`
- `lp_holder_count`
- `lp_holders`
- `lp_total_approved_to_routers`
- `lp_approved_percentage`
- `lp_last_approval_block`
- `lp_last_approval`
- `lp_holders_with_approvals`
- `lp_transfer_count`
- `lp_approval_count`

Known server exposure gaps:

- V3/V4 rows are exposed, but V4 pools use a composite `pool_address` of `pool_manager#pool_id` because a V4 pool is not an ERC-20-style pool contract address.
- No token snapshot DTO matching Python `build_token_snapshot` yet.
- Token tax/max-buy event fields are not exposed because Rust token state does not track them yet.
- Denom symbols/names and cross-pool liquidity matrix/best-price views are not exposed yet.
