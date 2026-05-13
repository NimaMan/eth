# eth_chain_server

Agent operating map for the runtime/API layer around `eth_token`.

## Purpose

- Host range and live token tracking in process memory.
- Warm from the processed-block disk cache, then process and apply live blocks
  through the in-process `LiveChainRuntime`.
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
processed-block disk cache + live execution RPC/WS
  -> LiveChainRuntime block loop
  -> eth_live_feed warmup/direct LiveBlockUpdate apply
  -> eth_token block application
  -> LiveTokenEvent::BlockApplied broadcast
  -> RecentLiveBlocks ring
  -> in-memory token/pool views
  -> /live/updates, /live/processed-blocks, /live/tokens, /live/pools
  -> HTTP/SSE clients, mempool context, alpha polling
```

## Live Pipeline Boundary

Keep `eth_chain_server` as the confirmed-chain read model host. It should not own
pending transaction ingestion or mempool signal creation.

Live runtime contracts:

- `LiveChainRuntime` subscribes to new heads, processes confirmed blocks, writes
  the processed-block replay cache when available, and hands each
  `LiveBlockUpdate` directly to `LiveTokenRuntime`.
- `eth_chain_server` applies `eth_token`, keeps the live token/pool registry and
  recent processed block ring in memory, and exposes chain/token context over
  HTTP.
- `mempool_signal_detector` consumes `/live/tokens` and `/live/pools` for
  token context. `/live/updates` is a notification-only wakeup so it can refresh
  that context immediately after chain-server applies a block.
- `eth_alpha_trader` consumes chain-server APIs and persisted mempool signals.
- ASENA reads chain-server/trade APIs only.

The mempool signal endpoints read `live_trading.signal_events` plus typed detail
tables. The JSON response keeps the old `pool_address` field name for clients,
but it is backed by protocol-aware `pool_identifier` values such as an EVM pool
address, `pool_manager#pool_id`, or `vault#pool_id`. ZMQ and signal logs are
diagnostics; they are not the chain-server or ASENA source of truth.

Resolved live-tail bottleneck target: V2 pool metadata lookups became expensive
when they fell back through Redis live-state snapshots. See
[`docs/live-v2-metadata-bottleneck.md`](docs/live-v2-metadata-bottleneck.md)
for evidence and the migration plan that led to the direct handoff.

## Range Run Performance Path

Historical range builds are split into three distinct costs:

```text
processed-block disk cache read
  -> token block apply
  -> in-memory view materialization for HTTP clients
```

When the processed-block disk cache is hot, cache reads are usually only a few
milliseconds per block. Slow range builds should therefore be profiled around
`src/ranges/pipeline/apply.rs` and `src/ranges/pipeline/state.rs`, not
only around cache loading.

The token block apply path already uses block-scoped simulator sessions through
`eth_token`: token and pool facts are applied for the whole mined block first,
simulation requests are coalesced by `(token, pool_kind, pool_id)`, and one
historical post-block `BlockStateSession` is opened lazily per block that needs
pool simulation. If a block with `simulations_attempted=0` is slow, the
bottleneck is not simulation state loading.

The current range state keeps `BlockTokenProcessor` inside `RangeIndexState`
behind one `RwLock`. This has two performance consequences:

- `take_processor_for_apply` clones the full processor before each block apply.
  As a run accumulates tokens, pools, indexes, and network graphs, that clone
  can become a hidden per-block cost.
- View endpoints such as `GET /runs/:id/tokens`, `GET /runs/:id/pools`, and
  `GET /runs/:id/strategy/launch-stats` read the same state and materialize
  large DTOs. Polling those endpoints during an active build competes with the
  writer that restores the processor and updates progress.

Preferred fixes are:

- Move the mutable `BlockTokenProcessor` out of the progress/view lock, for
  example into a dedicated processor owner or mutex, and expose lightweight
  progress snapshots separately.
- Cache token/pool/strategy view DTO snapshots or refresh them on a slower
  cadence while the build is running.
- Keep `/runs/active` cheap and poll it frequently; fetch the heavy token/pool
  lists only periodically, on manual refresh, or when the run reaches a terminal
  status.

Useful profiling checks:

```bash
cargo run --manifest-path blockchains/eth/Cargo.toml -p tx_simulator --release \
  --example profile_block_tx_session -- \
  --datadir /home/nima/storage/samsung8tb/ethereum/reth \
  --block 25057078 --iterations 2 --warmup-iterations 1

cargo run --manifest-path blockchains/eth/Cargo.toml -p tx_simulator --release \
  --example profile_replay_engines -- \
  --datadir /home/nima/storage/samsung8tb/ethereum/reth \
  --blocks 25057078 --iterations 1 --mode feasibility
```

The server writes one log directory per process under `CHAIN_SERVER_LOG_DIR`
(default: `/home/nima/code/crypto/blockchains/eth/logs/eth_chain_server`):

```bash
/home/nima/code/crypto/blockchains/eth/logs/eth_chain_server/run-<YYYYMMDD-HHMMSSZ>-pid-<pid>/
```

Set `CHAIN_SERVER_LOG_RUN_ID=<name>` to force a predictable run folder name for
repeatable profiling. Each run folder contains a `run_manifest.json` with the
run id, pid, root path, run path, and expected file list:

```text
run_manifest.json                 run metadata and expected log files
server.log                         server lifecycle plus warnings/errors
events.jsonl                       structured warnings/errors and panic details
live_token_tracker.jsonl           live warmup/tail progress and failures
token_pipeline_profile.jsonl       token pipeline profile rows
pool_buy_sell_sim_failures.jsonl   pool buy/sell simulator warnings/errors only
simulation_failures.jsonl          other simulator warnings/errors
pipeline_issues.jsonl              structured operational issues for Bogaz/ops
pipeline_health.jsonl              structured health heartbeat and ops reads
pipeline_bottlenecks.jsonl         structured slow-path samples
```

The structured ops API is available under:

```text
GET /eth/tokens/api/ops/health
GET /eth/tokens/api/ops/issues
GET /eth/tokens/api/ops/bottlenecks
```

`/ops/issues` groups by `dedupe_key`, so repeated pool-local failures such as a
Uniswap V3 factory/configuration mismatch render as one issue group with an
occurrence count instead of many raw transaction rows.

`token_pipeline_profile.jsonl` contains these targets:

- `token_range_apply_profile`: range-runner wall time around processor take,
  token apply, state update, and processed-block disk cache read.
- `token_block_processor_profile`: block-token-processor phase totals and
  token-applier aggregate totals for the block, including candidate-token
  counts, candidate simulation-pool counts, actual simulated-pool counts, and
  the range `run_id` when the block was processed by a range run.
- `token_sim_session_profile`: optional per-pool simulation branch rows. Set
  `TOKEN_SIM_SESSION_PROFILE=1` when investigating simulator session reuse;
  normal runs omit these rows to keep profile logs focused on block-level cost.
- `live_token_apply_profile`: live warmup/tail wall time around state-lock wait,
  in-place token block processing, retention, progress/event update, and
  processed-block cache read/write timing.

Timing fields are emitted in microseconds as `*_us`; matching `*_ms` fields are
kept for quick inspection and older tooling.

Summarize a captured profile log:

```bash
python3 eth_chain_server/scripts/token_pipeline_profile_summary.py \
  /home/nima/code/crypto/blockchains/eth/logs/eth_chain_server/<run-id>/token_pipeline_profile.jsonl \
  --run-id run-1 \
  --csv /tmp/token_pipeline_profile.csv
```

## Where To Look First

| Need | Start here |
| --- | --- |
| Public server crate exports | `src/lib.rs` |
| Runtime config and process logging | `src/app/config.rs`, `src/app/logging.rs`, `../config.env` |
| Shared runtime state | `src/app/state.rs` |
| HTTP routes/server | `src/http/`, `src/main.rs` |
| Range indexing orchestration | `src/ranges/` |
| Token/pool DTOs | `src/read_models/token/`, `src/read_models/pool/`, `src/read_models/live.rs` |
| Token active-block lookup | `src/read_models/activity.rs` |
| Mempool and alpha stores | `src/stores/` |
| Live token tracker host | `src/live/` |
| Processed-block cache sizing | `examples/processed_block_disk_cache_size.rs` |

The active layout keeps runtime concerns separate:

```text
src/app/          config, logging, shared server state
src/http/         warp server, route registry, SSE helpers
src/ranges/       historical range job manager and block-apply pipeline
src/read_models/  DTO/read-model builders for tokens, pools, live status, runs
src/stores/       external read stores for mempool signals and alpha tables
src/live/         live warmup/tail tracker host
```

Compatibility re-exports for the old `config`, `server`, `range_indexer`,
`views`, `alpha_trading`, and `mempool_signals` module names remain in
`src/lib.rs` for examples and downstream callers, but new code should use the
folders above.

## Tests And Commands

```bash
cargo run -p eth_chain_server
curl -s http://127.0.0.1:8765/health
curl -s http://127.0.0.1:8765/live/status
cargo run -p eth_chain_server --example processed_block_disk_cache_size -- --fill-missing-then-read
```

Key config defaults come from `config.env`:

```bash
RETH_DATADIR=/home/nima/storage/samsung8tb/ethereum/reth
RETH_INDEX_DIR=/home/nima/storage/samsung8tb/ethereum/reth/reth_index
RETH_HTTP_RPC=http://127.0.0.1:8545
RETH_WS_RPC=ws://127.0.0.1:8546
CHAIN_SERVER_BIND=127.0.0.1:8765
CHAIN_SERVER_AUTO_START_LIVE=true
PROCESSED_BLOCK_DISK_CACHE_DIR=/home/nima/storage/samsung8tb/ethereum/processed-block-cache
MEMPOOL_DATABASE_URL=postgresql://postgres:postgres@localhost:5432/eth_db
ALPHA_DATABASE_URL=postgresql://postgres:postgres@localhost:5432/eth_db
```

## Current Hazards

- The server is a read gateway, not the owner of token logic. Put token state
  rules in `eth_token`, then expose them here.
- The processed-block cache is shared replay input, not a chain-server snapshot.
- `GET /runs/:id/tokens/:address` may include raw cloned token state for
  debugging; production UI should prefer explicit summary/pool DTOs.
- V4 pools use a composite `pool_address` of `pool_manager#pool_id` because V4
  pools are not ERC-20-style pool contracts.
- `GET /tokens/:address/activity-blocks` uses the custom RethIndex
  address-participation table. It expects an ERC-20 token address; a Uniswap v4
  pool id from Dexscreener is a 32-byte identifier and is not queryable as an
  address.
- Set `CHAIN_SERVER_AUTO_START_LIVE=false` for range/detail API work when live
  tailing is not needed.
