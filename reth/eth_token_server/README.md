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
- `ETH_TOKEN_SERVER_MAX_BLOCKS=10000`
- `ETH_TOKEN_SERVER_DEFAULT_BLOCKS=7000`
- `ETH_TOKEN_SERVER_PROCESSED_BLOCK_CACHE_DIR` unset, which disables the disk cache
- `ETH_TOKEN_SERVER_PROCESSED_BLOCK_CACHE_BLOCKS=100000`

To enable the traced `ProcessedBlock` disk cache:

```bash
export ETH_TOKEN_SERVER_PROCESSED_BLOCK_CACHE_DIR=/home/nima/storage/samsung8tb/ethereum/processed_block_cache
export ETH_TOKEN_SERVER_PROCESSED_BLOCK_CACHE_BLOCKS=100000
```

The cache stores a sparse token-analysis subset of traced `ProcessedBlock`s as
`bincode` compressed with `zstd`: block header, only non-empty token-relevant
processed transaction fields, and per-transaction processing errors. Full
traces, receipts, raw metadata, calldata, state maps, empty event families, and
unrelated decoded event families are not retained. Runs read one block at a
time, so cached blocks are not retained in memory after they are applied to the
token tracker.

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

Example:

```bash
curl -s http://127.0.0.1:8765/runs \
  -H 'content-type: application/json' \
  -d '{"block_count":7000}'
```
