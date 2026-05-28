# eth_chain_server

Agent operating map for the runtime/API layer around `eth_token`.

## Purpose

- Host range and live token tracking in process memory.
- Warm from the processed-block disk cache, then process and apply live blocks
  through the in-process `LiveChainRuntime`.
- Expose read-only HTTP/SSE views for tokens, pools, live status, mempool
  signals, and alpha-facing market inputs.

## API Surfaces

The server now has three explicit client surfaces:

- Frontend/lab: versioned HTTP JSON/SSE under `/api/v1/eth/...`.
- Trading: committed-state events inside the runtime, with a narrow HTTP facade
  under `/api/v1/eth/live-trading/...` for supervision and future bridges.
- Agents: stable automation/orientation routes under `/api/v1/eth/agents/...`.

Agents should use the agent HTTP surface, not the internal trading boundary. The
internal path is optimized for low-latency committed state and can change with
the trading runtime; agents need stable, discoverable, tool-friendly JSON.

The old `/eth/tokens/api/...` paths remain compatibility routes. New clients
should prefer the versioned paths. See
[`docs/api-surfaces.md`](docs/api-surfaces.md) for the route map and ownership
rules.

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
  -> direct live block-session ring for current headers/state diffs
  -> eth_token block application
  -> LiveTokenEvent::BlockApplied broadcast
  -> RecentLiveBlocks ring
  -> LiveBlockFrame ring for alpha block-pinned inputs
  -> in-memory token/pool views
  -> /live-trading/block-frames/next, /live-token-tracker/processed-blocks, /live-token-tracker/tokens, /live-token-tracker/pools
  -> HTTP/SSE clients, mempool context, alpha block-frame consumption
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
- `mempool_signal_detector` consumes `/live-token-tracker/tokens` and `/live-token-tracker/pools` for
  token context. `/live-token-tracker/block-applied-updates` is a notification-only wakeup so it can refresh
  that context immediately after chain-server applies a block. It also consumes
  `/live-tx-simulator/status`,
  `/live-tx-simulator/simulations/unsigned-transaction-sequence`, and
  `/live-tx-simulator/simulations/pool-buy-sell` so live pending-tx replay uses
  the chain-server-owned exact `LiveTxSimulator` state.
- `eth_alpha_trader` consumes `/api/v1/eth/live-trading/block-frames/next`
  for block-pinned updated pool inputs, exact live simulation APIs for
  execution checks, and persisted mempool signals. It must not use the latest
  pool list as its confirmed-chain strategy input.
- ASENA reads chain-server/trade APIs only.

The mempool signal endpoints read `live_trading.signal_events` plus typed detail
tables. The JSON response keeps the old `pool_address` field name for clients,
but it is backed by protocol-aware `pool_identifier` values such as an EVM pool
address, `pool_manager#pool_id`, or `vault#pool_id`. ZMQ and signal logs are
diagnostics; they are not the chain-server or ASENA source of truth.

Live pool API shape:

- `GET /eth/tokens/api/live-token-tracker/pools` returns all retained pools.
- `GET /eth/tokens/api/live-token-tracker/pools?status=active` returns non-scam pools.
- `GET /eth/tokens/api/live-token-tracker/pools?status=scam` returns scam/liquidity-removal
  pools.
- `GET /eth/tokens/api/live-token-tracker/pools/active` and
  `GET /eth/tokens/api/live-token-tracker/pools/scam` are explicit aliases for clients that
  should not depend on query strings.

Pool-list responses include `count` for the returned set plus `total_count`,
`active_count`, and `scam_count`, so clients can show active/scam separation
without issuing multiple requests.

## Hot Path And Read-Model Contract

Block application must stay a small, deterministic hot path. Anything needed for
future trading may run on that path; anything needed only for pages, analysis, or
large historical exports must be moved to a durable writer or throttled read
model.

The target event flow is:

```text
processed block / live block update
  -> tx_processor facts and prestate diffs for block B
  -> build BlockStateSession B with the processed header and block context
  -> publish BlockStateSession B into chain-server LiveTxSimulator
  -> eth_token block apply
  -> TokenBlockUpdateReport
  -> record live simulation state frame for the processed block
  -> critical state commit
       - restore/update token processor ownership
       - increment cheap progress counters
       - publish BlockApplied/RangeBlockApplied event
       - record replayable alpha LiveBlockFrame for block B
  -> async/durable sinks
       - Risk Atlas observation writer
       - token/pool page snapshot refresher
       - ops/profile logs
  -> read APIs and Asena pages
```

The critical state commit is allowed to do only O(block delta) work. It must not
scan the full token registry, materialize large token/pool DTOs, or rebuild
strategy/atlas surfaces every block. Those jobs are read-model work.

The live simulation state frame and exact `LiveTxSimulator` state are available
before `BlockApplied` is published. For real live trading, chain-server owns
live state, `LiveTxSimulator`, and simulation sessions. Alpha owns strategy
decisions, tx planning, gas policy, and Kartal submission. The mempool detector
owns pending-tx routing and signal interpretation. Both clients should request
exact-block simulation results from chain-server instead of rebuilding the live
state frame locally.

For live-tail blocks, the `LiveTxSimulator` update is immediate within the
processed-block apply path: after chain-server has the processed block header,
block context, and prestate diffs for block `B`, it builds and publishes
`BlockStateSession B` into the chain-server `LiveTxSimulator` before token/pool
state is updated and before the `BlockApplied` event is visible to Alpha. If the
exact session cannot be built, chain-server does not silently fall back to an
older historical state; exact-block simulation requests for that block fail.

The practical ownership split is:

- `eth_token` owns canonical token/pool state and emits compact per-block
  update reports.
- Range runs own cheap progress and run lifecycle, not expensive page
  materialization on every block.
- Live runs own the confirmed-chain in-memory state needed by trading and page
  snapshots, but UI snapshots should refresh after a full block commit and at a
  bounded cadence.
- Risk Atlas owns durable row-level analytics tables and page story tables.
  Range runs should stream or batch observations into that DB instead of keeping
  the whole atlas export as in-memory server state.
- Asena renders read models. It should not compute features, targets, or
  eligibility rules.

For a live trading system, the event order must be explicit:

```text
new confirmed block
  -> process complete block
  -> update token/pool state for that block
  -> publish chain-state-applied event with compact updated token snapshots
  -> chain-server records LiveBlockFrame N for the updated pools/tokens
  -> trading/risk gates consume LiveBlockFrame N, not the latest pool surface
  -> UI/read-model snapshots refresh after the trading state is committed
```

This means the trading path should never wait for `/live-token-tracker/tokens`,
`/live-token-tracker/pools`, Risk Atlas page rendering, or any full-registry statistics. It
should consume the committed state/event stream directly, while the frontend uses
snapshot/read-model endpoints.

Resolved live-tail bottleneck target: V2 pool identity/metadata lookups became
expensive when they fell back through external live-state snapshots. See
[`docs/live-v2-metadata-bottleneck.md`](docs/live-v2-metadata-bottleneck.md)
for evidence and the migration plan that led to the direct handoff.

## Range Run Performance Path

Historical range builds are split into three distinct costs:

```text
processed-block disk cache read
  -> token block apply
  -> state commit / read-model updates
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

Candidate routing is also a measured part of token apply. For unknown V2
`Swap`/`Sync`/`Mint`/`Burn` pair addresses, `eth_token` should use the cheap V2
pool identity path (`token0`, `token1`, known protocol validation) before full
metadata. Full metadata includes decimals and is only needed when registering a
pool on a tracked token. The profile summary exposes the split as
`applier_candidate_v2_transfer_route_*` and
`applier_candidate_v2_identity_lookup_*`, plus candidate-cache counters such as
`applier_candidate_v2_identity_cache_hits`,
`applier_candidate_v2_irrelevant_cache_hits`, and
`applier_candidate_v2_irrelevant_cache_inserts`.

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
- Keep progress counters incremental. Do not recompute tracked pool/token totals
  by scanning the full registry on every block.
- Stream or batch Risk Atlas observations into the Risk Atlas DB during range
  generation, or export once from compact row batches. Do not require the
  range-run state lock to retain and transform all atlas rows for the frontend.
- Cache token/pool/strategy view DTO snapshots or refresh them on a slower
  cadence while the build is running. A terminal run can refresh the final view
  once.
- Keep `/runs/active` cheap and poll it frequently; fetch the heavy token/pool
  lists only periodically, on manual refresh, or when the run reaches a terminal
  status.

Range-builder UI contract:

- `/runs/active` is the high-frequency endpoint and should stay cheap.
- `/runs/:id/tokens`, `/runs/:id/pools`, `/runs/:id/surface`, and launch stats
  are read-model endpoints. During an active build they may be stale by a few
  seconds or a configured block interval.
- Risk Atlas pages should read from the Risk Atlas DB, not from the active
  range-run lock.
- A range run intended only for Risk Atlas/model generation should be able to
  run headless with durable DB writes and no token-builder page materialization.

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
  the range `run_id` when the block was processed by a range run. V2 candidate
  fields distinguish pair-created routing, ERC-20-transfer routing, and V2 pool
  identity lookup so metadata regressions are visible at block granularity.
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
cargo run -p eth_chain_server -- --config /home/nima/code/crypto/blockchains/eth/config.env
curl -s http://127.0.0.1:8765/health
curl -s http://127.0.0.1:8765/api/v1/eth/live-token-tracker/status
cargo run -p eth_chain_server --example processed_block_disk_cache_size -- --fill-missing-then-read
```

Runtime values come from the config file. `--config <path>` is only a path
selector for isolated runs such as profiling; it is not an environment fallback.
Without `--config`, the server reads `blockchains/eth/config.env`.

## Persistent Store Dependencies

`eth_chain_server` is a host and read gateway; it opens stores owned by other
crates and exposes API read models over them.

| Store | Config | Used for | Owner |
| --- | --- | --- | --- |
| Reth datadir | `RETH_DATADIR` | Direct chain reads and simulation provider setup through `RethQueryProvider`. | `tx_simulator` / `reth_chain_query` |
| RethIndex | `RETH_INDEX_DIR` or `<RETH_DATADIR>/reth_index` | `address_to_blocks` reads for token activity-block lookups, and optional address-index writes when processed-block replay writes are enabled. | `reth_chain_query/src/reth_index/` |
| Processed-block disk cache | `PROCESSED_BLOCK_DISK_CACHE_DIR`, `PROCESSED_BLOCK_DISK_CACHE_BLOCKS` | Live warmup, historical range reads, and processed-block replay writes. | `tx_processor/src/processed_tx_provider/block/` |
| Mempool signals | `databases.mempool.url` | Reads `live_trading.signal_events` and typed detail tables for mempool signal API routes. | `mempool_processor/src/db_writers/` |
| Alpha trading | `databases.alpha.url` | Reads and resets `alpha_trading.*` strategy/run/trade tables for alpha pages and APIs. | `alpha/store/` |
| Risk Atlas | `databases.risk_atlas.url` | Reads/writes `risk_atlas_*` tables for Risk Atlas pages and range exports. | `risk_atlas/` |

The server should not compute these stores' business semantics in HTTP routes.
Routes should call the owner crate/read model and return backend-provided fields
as-is. If a value is missing from the backend, the frontend should show it empty
rather than re-derive it client-side.

Path/runtime defaults still come from `config.env`; database URLs come from
`config.toml`:

```bash
RETH_DATADIR=/home/nima/storage/samsung8tb/ethereum/reth
RETH_INDEX_DIR=/home/nima/storage/samsung8tb/ethereum/reth/reth_index
RETH_HTTP_RPC=http://127.0.0.1:8545
RETH_WS_RPC=ws://127.0.0.1:8546
CHAIN_SERVER_BIND=127.0.0.1:8765
CHAIN_SERVER_AUTO_START_LIVE=true
PROCESSED_BLOCK_DISK_CACHE_DIR=/home/nima/storage/samsung8tb/ethereum/processed-block-cache
```

```toml
[databases.mempool]
url = "postgresql://<user>:<password>@<host>:<port>/<database>"

[databases.alpha]
url = "postgresql://<user>:<password>@<host>:<port>/<database>"

[databases.risk_atlas]
url = "postgresql://<user>:<password>@<host>:<port>/<database>"
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
