# Ethereum Token Server

`eth_token_server` is the runtime/API layer around `eth_token`.

It runs token tracking over requested block ranges, keeps the resulting
`TokenRegistry` and `TrackedTokenIndex` in process memory, and exposes read-only
HTTP/SSE views for an inspector frontend.

It intentionally does not persist snapshots. A tracking run disappears when the
server stops or when the run is removed in a later lifecycle endpoint.

## Run

```bash
cd /home/nima/code/crypto/blockchains/eth/reth
RUST_LOG=info cargo run -p eth_token_server
```

Default configuration:

- `RETH_DATADIR=/home/nima/storage/samsung8tb/ethereum/reth`
- `ETH_TOKEN_SERVER_BIND=127.0.0.1:8765`
- `ETH_TOKEN_SERVER_HISTORY_LIMIT=1000`
- `ETH_TOKEN_SERVER_DEFAULT_BLOCKS=7000`
- `ETH_TOKEN_SERVER_LIVE_WARMUP_BLOCKS=7000`
- `ETH_TOKEN_SERVER_LIVE_BLOCK_APPLY_TIMEOUT_MS=30000`
- `ETH_TOKEN_SERVER_LIVE_PROCESSED_BLOCK_DISK_CACHE_RETRY_ATTEMPTS=20`
- `ETH_TOKEN_SERVER_LIVE_PROCESSED_BLOCK_DISK_CACHE_RETRY_DELAY_MS=100`
- `ETH_TOKEN_SERVER_PROCESSED_BLOCK_DISK_CACHE_DIR=$ETH_NODE_ROOT/processed_block_disk_cache`
- `ETH_TOKEN_SERVER_PROCESSED_BLOCK_DISK_CACHE_BLOCKS=100000`

To override the traced `ProcessedBlock` disk cache location:

```bash
export ETH_TOKEN_SERVER_PROCESSED_BLOCK_DISK_CACHE_DIR=/home/nima/storage/samsung8tb/ethereum/processed_block_disk_cache
export ETH_TOKEN_SERVER_PROCESSED_BLOCK_DISK_CACHE_BLOCKS=100000
```

The cache stores a sparse token-analysis subset of traced `ProcessedBlock`s as
`bincode` compressed with `zstd`: block header, only non-empty token-relevant
processed transaction fields, original calldata/gas replay fields, and
per-transaction processing errors. Full traces, receipts, raw metadata, state
maps, empty event families, and unrelated decoded event families are not
retained. Runs acquire cached blocks in 250-block chunks, so only one cache
read batch is retained before those blocks are applied to the token tracker in
block order.

For compatibility, the server still accepts the old
`ETH_TOKEN_SERVER_PROCESSED_BLOCK_CACHE_DIR` and
`ETH_TOKEN_SERVER_PROCESSED_BLOCK_CACHE_BLOCKS` environment variables, and will
use an existing `$ETH_NODE_ROOT/processed_block_cache` directory if the newer
`processed_block_disk_cache` directory is not present.
It also accepts the old `ETH_TOKEN_SERVER_LIVE_CACHE_RETRY_ATTEMPTS` and
`ETH_TOKEN_SERVER_LIVE_CACHE_RETRY_DELAY_MS` names as aliases for the live
processed-block disk-cache retry settings.

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
  - `pools`: frontend-facing `PoolView` rows for the token's V2 pools.
- `GET /runs/:id/pools` returns all tracked V2 pools as the same `PoolView` rows.
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

- No V3/V4 pool rows yet.
- No token snapshot DTO matching Python `build_token_snapshot` yet.
- Token tax/max-buy event fields are not exposed because Rust token state does not track them yet.
- Denom symbols/names and cross-pool liquidity matrix/best-price views are not exposed yet.
