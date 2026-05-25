# Ethereum Workspace

This directory is the Ethereum Rust workspace and the operational home for the
local Reth-backed ETH stack. Start here when you need to understand where code
lives, how data moves, and which crate owns a behavior.

The guiding rule for docs is one folder-level `README.md` as the entry point for
that folder. Do not add parallel overview files in the same folder. If a folder
needs more detail, put the summary and navigation in its `README.md`, then link
to focused child-folder READMEs.

## Workspace Shape

The root Rust workspace is this directory. Current workspace members from
`Cargo.toml`:

| Folder | Crate | Purpose |
| --- | --- | --- |
| `tx_simulator/` | `tx_simulator` | Local Reth/MDBX EVM simulation, tracing, live-state replay, tx-chain sessions, and transaction builders. |
| `reth_chain_query/` | `reth_chain_query` | Typed direct DB queries, common addresses, entity views, RethIndex tables, live head helpers, and AMM/Dex helpers. |
| `tx_processor/` | `tx_processor` | Processed transactions and blocks: log decoding, traces, balance deltas, tax math, disk cache, and live block processing. |
| `eth_token/` | `eth_token` | Token, pool, health, lifecycle, and network state built from processed blocks/transactions. |
| `risk_atlas/` | `eth_risk_atlas` | Durable token/pool risk intelligence, Risk Atlas DB/read models, investigations, scam analytics, network analytics, and modeling. |
| `eth_chain_server/` | `eth_chain_server` | Runtime/API around `eth_token`; live tracker host, processed-block cache reader, HTTP/SSE views, and alpha-facing endpoints. |
| `mempool_processor/` | `mempool_processor` | Reth IPC mempool fetch, function detection, pending simulation, semantic signal detection, DB writers, and ZMQ publishing. |
| `pyreth/` | `pyreth` | PyO3 bindings over the Rust simulator, chain query, tx processor, and selected higher-level helpers. |
| `alpha/core/` | `eth_alpha_core` | Pure trading domain types and traits. |
| `alpha/block_tx_rank/` | `eth_block_tx_rank` | Rough mined-block transaction rank and gas-before estimates for live trading decisions. |
| `alpha/store/` | `eth_alpha_store` | Durable run, observation, order, position, execution, and risk records. |
| `alpha/strategies/` | `eth_strategies` | Built-in strategy implementations. |
| `alpha/engine/` | `eth_alpha_engine` | Strategy runtime, portfolio/order state, risk gating, and execution adapter boundary. |
| `alpha/live/state/` | `eth_live_state` | Legacy live-state schemas and protocol helpers; not the normal chain-server live transport. |
| `alpha/live/feed/` | `eth_live_feed` | Live confirmed-chain feed over processed blocks and token updates. |
| `alpha/live/trading/` | `eth_live_trading` | Live tx-prep, priority-exit policy, Kartal direct-raw client shape, and value-capped gas/bribe planning. |

Important adjacent code that is not currently a root workspace member:

| Folder | Purpose |
| --- | --- |
| `tx_executor/` | Direct transaction submission core. Receives prepared transactions; does not choose strategy, routes, or rank. |
| `tx_fund_flow/` | Fund-flow/network analytics built around processed transactions and DB-backed queries. |
| `deploy/` | ETH-owned deployment assets, including node scripts, systemd units, and on-chain deployment ledgers. |
| `solidity/` | Archived Solidity executor/contracts and experiments. Current v4 simulation uses deployed Uniswap periphery. |
| `deploy/onchain/` | ETH mainnet contract deployment runbooks, configs, audit checklists, Kartal dry-runs, receipt evidence, and reproducible signoff records. |
| `vendor/reth/` | Vendored upstream Reth reference tree. Use for source parity and examples, not as normal application code. |

## Main Data Flow

```text
Reth MDBX database + node IPC/RPC
  -> tx_simulator
       local EVM execution, tracing, view calls, block replay, live-state replay

tx_simulator
  -> reth_chain_query
       typed reads, address/entity helpers, DEX helpers, RethIndex tables
  -> tx_processor
       simulate tx/block -> ProcessedTransaction / ProcessedBlock

eth_chain_server LiveChainRuntime
  -> tx_processor live block processing
  -> processed-block disk cache
  -> direct LiveBlockUpdate handoff to eth_live_feed

eth_token
  <- tx_processor ProcessedBlock / ProcessedTransaction
  -> token state, pool state, health state, network/activity views

eth_chain_server
  <- eth_token + eth_live_feed + processed-block disk cache + execution RPC/WS
  -> HTTP live/range views for tokens, pools, mempool signals, alpha strategy input

mempool_processor
  <- Reth pending transactions
  <- eth_chain_server /live/updates wakeup + /live/tokens and /live/pools context
  <- tx_simulator / tx_processor for simulation and tax/tradability checks
  -> Postgres signal rows, signal logs, ZMQ notifications

alpha
  <- eth_chain_server live pools/status
  <- mempool signal rows
  <- recent mined block fee samples for block-rank evidence
  -> strategy observations, chain-sim orders, positions, risk events in Postgres
  -> prepared direct-raw tx requests only after real planner wiring exists

pyreth
  -> Python-facing wrappers around simulator/query/processor APIs

tx_executor
  <- prepared direct transactions from a planner/strategy adapter
  -> nonce, fee-cap validation, signing, broadcast, execution records
```

Short version: `tx_simulator` executes chain state; `reth_chain_query` reads and
builds DB-backed context; `tx_processor` turns execution into decoded facts;
`eth_token` turns decoded facts into token/pool state; `eth_chain_server` hosts
that state; `mempool_processor` detects speculative risk/opportunity; `alpha`
decides what to do with confirmed state plus mempool risk.

## Where To Look First

Use this map before broad searching:

| Question | Start here | Then inspect |
| --- | --- | --- |
| How do I simulate a transaction, bundle, block, or live head state? | `tx_simulator/README.md` | `tx_simulator/src/lib.rs`, `src/single_tx/`, `src/tx_chain/`, `src/block_trace/`, `src/live/`, `examples/` |
| How do I query balances, blocks, receipts, entities, DEX state, or indexed data? | `reth_chain_query/README.md` | `reth_chain_query/src/lib.rs`, `src/provider/`, `src/reth_index/`, `src/dex/`, `src/tx_builders.rs`, `examples/` |
| How do raw/simulated transactions become decoded transaction facts? | `tx_processor/README.md` | `tx_processor/src/lib.rs`, `src/tx_processor/`, `src/processed_tx_provider/`, `src/block_processor/` |
| How are live processed blocks cached and applied? | `eth_chain_server/README.md` | `eth_chain_server/src/live/`, `tx_processor/src/live/`, `alpha/live/feed/README.md` |
| Where is token/pool state updated from processed blocks? | `eth_token/README.md` | `eth_token/src/README.md`, `src/tracking/`, `src/pools/`, `src/manager/`, `src/health/` |
| How is live token state served to tools and alpha? | `eth_chain_server/README.md` | `eth_chain_server/src/live.rs`, `src/views/`, `src/server/`, `src/mempool_signals.rs` |
| How are pending transactions detected and converted to signals? | `mempool_processor/README.md` | `mempool_processor/src/function_detector.rs`, `src/tx_router/`, `src/simulator/`, `src/signal_detector/`, `src/db_writers/` |
| How does the chain-sim/live alpha loop work? | `alpha/README.md` | `alpha/core/README.md`, `alpha/engine/README.md`, `alpha/store/README.md`, `alpha/strategies/README.md`, `alpha/live/*/README.md` |
| How do I estimate rough tx position from recent mined blocks? | `alpha/block_tx_rank/README.md` | `alpha/block_tx_rank/src/lib.rs`, `reth_chain_query/src/provider/block/` |
| Where are current pipeline bottlenecks tracked? | `bogaz.md` | service memory, cache fill/read metrics, live readiness, mempool timing, alpha decision bottlenecks |
| How do Python callers access the Rust stack? | `pyreth/README.md` | `pyreth/src/lib.rs`, `src/python.rs`, `src/pyreth_instance.rs`, `examples/` |
| How does alpha prepare a live transaction? | `alpha/live/trading/README.md` | `alpha/live/trading/src/tx_prep/`, `alpha/engine/src/execution/real/README.md`, `alpha/block_tx_rank/README.md` |
| How is a prepared real transaction submitted? | `tx_executor/README.md` | `tx_executor/src/executor.rs`, `src/service.rs`, `examples/submit_direct_raw.rs` |
| How do we deploy and audit an ETH on-chain contract? | `deploy/onchain/README.md` | contract-specific folders such as `deploy/onchain/uniswap-v2-trading-vault/` |
| How do I investigate token behavior or launch strategy stats? | `risk_atlas/README.md` | `risk_atlas/token_lab/`, `risk_atlas/investigations/README.md`, `risk_atlas/scam_analytics/`, `risk_atlas/network_analytics/`, `alpha/lab/strategy_analysis/` |
| How are node paths and services configured? | `deploy/node/README.md` | `config.env`, `deploy/node/scripts/`, `deploy/systemd/` |
| How do archived Solidity executor experiments fit? | `solidity/README.md` | current production simulation paths live in `tx_simulator/` and `tx_processor/` |

When navigating as an agent, the fastest useful sequence is:

1. Read this README and the target folder README.
2. Check the relevant `Cargo.toml` for local path dependencies and binaries.
3. Read `src/lib.rs` for public module boundaries and re-exports.
4. Read `src/bin/*` or `examples/*` for real call paths.
5. Read tests only after you know the API boundary you are changing.
6. Search with `rg`, excluding `target/`, `logs/`, and `vendor/reth/` unless the
   question is explicitly about generated output or Reth upstream parity.

## Ownership Boundaries

Keep new code inside the crate that owns the behavior:

| Owner | Put here | Do not put here |
| --- | --- | --- |
| `tx_simulator` | Raw EVM execution, DB provider/fork setup, view calls, traces, block replay, live-state simulation, signed/unsigned tx-chain state. | Protocol/business decoding, token state, tax logic, mempool decisions, strategy decisions. |
| `reth_chain_query` | Typed Reth DB reads, provider abstractions, entity views, time/block helpers, DEX state readers, stateless calldata builders, RethIndex tables/writers. | Simulation orchestration, processed transaction semantics, tax math, live strategy logic. |
| `tx_processor` | `ProcessedTransaction`, `ProcessedBlock`, event/log decoding, internal calls, balance deltas, bribe/tax calculations, pool buy/approve/sell viability orchestration, disk cache. | Long-lived token registry state, HTTP serving, strategy decisions, transaction signing. |
| `eth_token` | Token and pool state machines, token health, control-address/activity state, network views, block-level token update logic from processed blocks. | Direct tracing/RPC, duplicate transaction decoding, live service hosting. |
| `eth_chain_server` | Process lifetime, warmup/live tail, in-memory token registry hosting, HTTP/SSE views, chain-server logs, alpha-facing read endpoints. | Core token state logic, core tx processing, strategy decisions. |
| `mempool_processor` | Pending tx ingestion, selector/function detection, routing, live context hydration, signal decisions, DB/ZMQ publishing. | Canonical token state mutation, duplicate tax/decoding logic, trading strategy state. |
| `alpha` | Market/risk event handling, strategy state machines, chain-sim execution adapters, mined-block rank evidence, live tx-prep, decision persistence, position/order lifecycle. | Raw simulation internals, token indexing, direct transaction signing. |
| `pyreth` | Thin Python wrappers and stable schema projection. | Business logic that should live in Rust crates. |
| `tx_executor` | Validate prepared transactions, reserve nonce, enforce fee caps, sign, broadcast, record execution attempts. | Route discovery, quote selection, strategy policy, pool discovery, tx rank estimation. |
| `tx_fund_flow` | Fund-flow network construction, ranking, analytics, visualization. | Core transaction simulation or decoding duplicates. |

## Common Runtime Inputs

Shared defaults live in this repository root. New durable service settings
should go in `config.toml`; legacy path/process settings still live in
`config.env` until they are migrated.

Important defaults currently used by local services:

```bash
RETH_DATADIR=/home/nima/storage/samsung8tb/ethereum/reth
RETH_IPC_PATH=/home/nima/storage/samsung8tb/ethereum/reth/reth.ipc
RETH_HTTP_RPC=http://127.0.0.1:8545
RETH_WS_RPC=ws://127.0.0.1:8546
PROCESSED_BLOCK_DISK_CACHE_DIR=/home/nima/storage/samsung8tb/ethereum/processed-block-cache
CHAIN_SERVER_BIND=127.0.0.1:8765
MEMPOOL_ZMQ_SIGNAL_ENDPOINT=tcp://127.0.0.1:5556
```

Database URLs are TOML settings:

```toml
[databases.alpha]
url = "postgresql://<user>:<password>@<host>:<port>/<database>"

[databases.mempool]
url = "postgresql://<user>:<password>@<host>:<port>/<database>"

[databases.token_pnl]
url = "postgresql://<user>:<password>@<host>:<port>/<database>"
```

Prefer direct DB access for performance-sensitive paths. RPC is acceptable for
node operations, broadcast, comparisons, and explicit verification examples.

## Persistent Stores And Databases

The ETH module uses a small number of persistent stores with different owners.
Do not add a new DB or schema until this table and the owner README are updated.

| Store | Type / config | Owner README | Purpose |
| --- | --- | --- | --- |
| Reth node datadir | Local Reth data under `RETH_DATADIR`; includes `db/` MDBX, `static_files/`, and Reth `rocksdb/` provider data. | `tx_simulator/README.md`, `reth_chain_query/README.md` | Canonical local Ethereum chain source for state, headers, transactions, receipts, and simulation forks. Opened read-only by normal application code. |
| Processed-block disk cache | File store at `PROCESSED_BLOCK_DISK_CACHE_DIR`; optional retention via `PROCESSED_BLOCK_DISK_CACHE_BLOCKS`. | `tx_processor/README.md`, `tx_processor/src/processed_tx_provider/block/README.md` | Compact `.pblock.zst` replay cache for `ProcessedBlock` payloads. Speeds live warmup, historical ranges, Risk Atlas exports, and token/network analysis. |
| RethIndex | Sidecar MDBX directory at `RETH_INDEX_DIR`, defaulting to `<RETH_DATADIR>/reth_index`. | `reth_chain_query/src/reth_index/README.md`, `reth_chain_query/src/reth_index/tables/README.md` | Custom low-latency indexes missing from canonical Reth. Current active tables are `address_to_blocks` and `mempool_tx_arrival_times`. |
| Fund-flow `eth_db` | PostgreSQL schema `eth_db`, read with `DATABASE_URL`. | `reth_chain_query/src/postgres_db/README.md`, `tx_fund_flow/README.md` | Legacy/curated relational chain analytics: addresses, transactions, tx participants, related addresses, token metadata, pool metadata, and trade aggregates used by fund-flow graph discovery and analytics. |
| Token PnL store | PostgreSQL schema `token_pnl`, configured by `databases.token_pnl.url`. | `eth_token_pnl_store/README.md` | Pool-scoped address PnL ledger and rollups: calculation runs, pool conservation totals, address-level PnL, and movement rows. |
| Mempool signal store | PostgreSQL schema `live_trading`, configured by `databases.mempool.url`. | `mempool_processor/README.md`, `mempool_processor/src/db_writers/README.md` | Source of truth for public pending-transaction signals. Core row table is `live_trading.signal_events`; typed detail tables hang off `signal_id`. |
| Alpha trading store | PostgreSQL schema `alpha_trading`, configured by `databases.alpha.url`. | `alpha/store/README.md`, `alpha/README.md` | Durable decision ledger for runs, observations, orders, execution reports, positions, position snapshots, trades, trade events/snapshots, risk events, decisions, result sets, performance views, and validation reports. |
| Risk Atlas read model | PostgreSQL tables `risk_atlas_*` in the same database used by `databases.alpha.url` in `eth_chain_server`. | `risk_atlas/README.md` | Durable scam/risk analytics read model for Risk Atlas pages: runs, eligibility, observations, distributions, active targets, decision questions, review examples, model readiness, and page snapshots. |

The Postgres schemas may live in the same physical database during local
development, but their ownership is separate. `databases.mempool.url` should not
be used as a fallback for alpha state, and frontend pages should read backend
APIs backed by these stores instead of reconstructing DB semantics client-side.

## Useful Commands

Run from this directory:

```bash
cargo check --workspace
cargo run -p tx_simulator --example verify_database_setup
cargo run -p tx_processor --example process_transaction_by_hash -- <tx_hash>
cargo run -p eth_chain_server
cargo run -p mempool_processor --bin mempool_signal_detector
```

For PyReth development, use the `pyreth/README.md` workflow. For node setup and
service management, use `deploy/node/README.md`.

## Testing And Investigation

Use focused tests/examples near the owner crate:

| Area | Best first checks |
| --- | --- |
| Simulator parity | `tx_simulator/examples/block/verify_block_trace_rpc_equivalence.rs`, `tx_simulator/examples/replay/*` |
| Query parity | `reth_chain_query/examples/block/*`, `reth_chain_query/examples/transactions/*`, `reth_chain_query/examples/reth_index/*` |
| Tx decoding and processed blocks | `tx_processor/tests/`, `tx_processor/examples/processing/*`, `tx_processor/examples/blocks/*` |
| Trade simulation examples | `tx_processor/examples/trade_simulation/*` |
| Token/pool state | `eth_token/tests/`, `eth_token/examples/tracking/token_tracking_range.rs`, `eth_token/examples/validation/*` |
| Live chain server | `eth_chain_server/README.md`, `logs/eth_chain_server/`, `GET /live/status`, `GET /live/pools` |
| Mempool signal behavior | `mempool_processor/examples/signal_detector/*`, `mempool_processor/src/signal_detector/README.md`, `logs/mempool_processor/` |
| Alpha decision loop | `alpha/README.md`, `alpha/store/README.md`, Postgres `alpha_trading.*` tables |
| Risk Atlas investigations and strategy cohorts | `risk_atlas/README.md`, one case folder under `risk_atlas/token_lab/cases/`, and `alpha/lab/strategy_analysis/README.md` |

Generated output and heavy directories are not orientation sources. Avoid
starting from `target/`, `logs/`, `.pytest_cache/`, or `vendor/reth/` unless the
task requires them.

## Adding New Functionality

Examples of correct placement:

- New AMM calldata builder: add it to `reth_chain_query::tx_builders` or the
  relevant DEX helper, then call it from `tx_processor` or a strategy.
- New buy/sell viability rule: orchestrate in `tx_processor::simulator`, using
  builders from `reth_chain_query` and execution from `tx_simulator`.
- New decoded event or balance-derived fact: add it to `tx_processor`, then
  let `eth_token`, `mempool_processor`, or `alpha` consume the processed fact.
- New token or pool state field: add it to `eth_token` and expose it through
  `eth_chain_server` views if callers need it.
- New mempool risk signal: detect/rout fast in `mempool_processor`, but consume
  processed transaction facts from `tx_processor` and token context from
  `eth_chain_server`.
- New strategy behavior: implement in `alpha/strategies` and persist decision
  evidence through `alpha/store`.
- New Python-facing capability: implement the Rust behavior in its owner crate,
  then expose a thin wrapper in `pyreth`.
- New transaction broadcast mode: add it behind `tx_executor`; strategies should
  still submit intent/prepared transactions through an adapter boundary.

Before adding a new markdown file, ask whether the information belongs in the
nearest folder `README.md`. The root README should stay the navigation map; the
owner folder README should hold the operational details for that folder.
