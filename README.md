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
| `eth_token_server/` | `eth_token_server` | Runtime/API around `eth_token`; live tracker host, processed-block cache reader, HTTP/SSE views, and alpha-facing endpoints. |
| `mempool_processor/` | `mempool_processor` | Reth IPC mempool fetch, function detection, pending simulation, semantic signal detection, DB writers, and ZMQ publishing. |
| `pyreth/` | `pyreth` | PyO3 bindings over the Rust simulator, chain query, tx processor, and selected higher-level helpers. |
| `alpha/core/` | `eth_alpha_core` | Pure trading domain types and traits. |
| `alpha/block_tx_rank/` | `eth_block_tx_rank` | Rough mined-block transaction rank and gas-before estimates for live trading decisions. |
| `alpha/store/` | `eth_alpha_store` | Durable run, observation, order, position, execution, and risk records. |
| `alpha/strategies/` | `eth_strategies` | Built-in strategy implementations. |
| `alpha/engine/` | `eth_alpha_engine` | Strategy runtime, portfolio/order state, risk gating, and execution adapter boundary. |
| `alpha/live/state/` | `eth_live_state` | Shared Redis live-state schemas and protocol. |
| `alpha/live/feed/` | `eth_live_feed` | Live confirmed-chain feed over processed blocks and token updates. |

Important adjacent code that is not currently a root workspace member:

| Folder | Purpose |
| --- | --- |
| `tx_executor/` | Direct transaction submission core. Receives prepared transactions; does not choose strategy, routes, or rank. |
| `tx_fund_flow/` | Fund-flow/network analytics built around processed transactions and DB-backed queries. |
| `token_lab/` | Repeatable token/pool investigations, launch strategy analysis, parity checks, and detector prototypes. |
| `node/` | Reth/Lighthouse node scripts and systemd service helpers. |
| `solidity/` | Archived Solidity executor/contracts and experiments. Current v4 simulation uses deployed Uniswap periphery. |
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

tx_processor live_block_processor
  -> processed-block disk cache
  -> Redis stream eth/live/blocks
  -> eth_live_feed / eth_token_server

eth_token
  <- tx_processor ProcessedBlock / ProcessedTransaction
  -> token state, pool state, health state, network/activity views

eth_token_server
  <- eth_token + eth_live_feed + processed-block disk cache
  -> HTTP live/range views for tokens, pools, mempool signals, alpha strategy input

mempool_processor
  <- Reth pending transactions
  <- eth_token_server token/pool context
  <- tx_simulator / tx_processor for simulation and tax/tradability checks
  -> Postgres signal rows, signal logs, ZMQ notifications

alpha
  <- eth_token_server live pools/status
  <- mempool signal rows
  <- recent mined block fee samples for block-rank evidence
  -> strategy observations, chain-sim orders, positions, risk events in Postgres

pyreth
  -> Python-facing wrappers around simulator/query/processor APIs

tx_executor
  <- prepared direct transactions from a planner/strategy adapter
  -> nonce, fee-cap validation, signing, broadcast, execution records
```

Short version: `tx_simulator` executes chain state; `reth_chain_query` reads and
builds DB-backed context; `tx_processor` turns execution into decoded facts;
`eth_token` turns decoded facts into token/pool state; `eth_token_server` hosts
that state; `mempool_processor` detects speculative risk/opportunity; `alpha`
decides what to do with confirmed state plus mempool risk.

## Where To Look First

Use this map before broad searching:

| Question | Start here | Then inspect |
| --- | --- | --- |
| How do I simulate a transaction, bundle, block, or live head state? | `tx_simulator/README.md` | `tx_simulator/src/lib.rs`, `src/single_tx/`, `src/tx_chain/`, `src/block_trace/`, `src/live/`, `examples/` |
| How do I query balances, blocks, receipts, entities, DEX state, or indexed data? | `reth_chain_query/README.md` | `reth_chain_query/src/lib.rs`, `src/provider/`, `src/reth_index/`, `src/dex/`, `src/tx_builders.rs`, `examples/` |
| How do raw/simulated transactions become decoded transaction facts? | `tx_processor/README.md` | `tx_processor/src/lib.rs`, `src/tx_processor/`, `src/processed_tx_provider/`, `src/block_processor/` |
| How are processed blocks cached and streamed live? | `tx_processor/src/bin/live_block_processor/README.md` | `tx_processor/src/live/`, `tx_processor/src/processed_tx_provider/block/`, `alpha/live/feed/README.md` |
| Where is token/pool state updated from processed blocks? | `eth_token/README.md` | `eth_token/src/README.md`, `src/tracking/`, `src/pools/`, `src/manager/`, `src/health/` |
| How is live token state served to tools and alpha? | `eth_token_server/README.md` | `eth_token_server/src/live.rs`, `src/views/`, `src/server/`, `src/mempool_signals.rs` |
| How are pending transactions detected and converted to signals? | `mempool_processor/README.md` | `mempool_processor/src/function_detector.rs`, `src/tx_router/`, `src/simulator/`, `src/signal_detector/`, `src/db_writers/` |
| How does the chain-sim/live alpha loop work? | `alpha/README.md` | `alpha/core/README.md`, `alpha/engine/README.md`, `alpha/store/README.md`, `alpha/strategies/README.md`, `alpha/live/*/README.md` |
| How do I estimate rough tx position from recent mined blocks? | `alpha/block_tx_rank/README.md` | `alpha/block_tx_rank/src/lib.rs`, `reth_chain_query/src/provider/block/` |
| Where are current pipeline bottlenecks tracked? | `bogaz.md` | service memory, cache fill/read metrics, live readiness, mempool timing, alpha decision bottlenecks |
| How do Python callers access the Rust stack? | `pyreth/README.md` | `pyreth/src/lib.rs`, `src/python.rs`, `src/pyreth_instance.rs`, `examples/` |
| How is a real transaction submitted? | `tx_executor/README.md` | `tx_executor/src/executor.rs`, `src/service.rs`, `examples/submit_direct_raw.rs` |
| How do I investigate token behavior or launch strategy stats? | `token_lab/README.md` | `token_lab/cases/README.md`, `token_lab/strategy/README.md`, `tools/detectors/`, `tools/chain_truth/`, `tools/parity/` |
| How are node paths and services configured? | `node/README.md` | `config.env`, `node/scripts/`, `node/systemd/` |
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
| `eth_token_server` | Process lifetime, warmup/live tail, in-memory token registry hosting, HTTP/SSE views, token-server logs, alpha-facing read endpoints. | Core token state logic, core tx processing, strategy decisions. |
| `mempool_processor` | Pending tx ingestion, selector/function detection, routing, live context hydration, signal decisions, DB/ZMQ publishing. | Canonical token state mutation, duplicate tax/decoding logic, trading strategy state. |
| `alpha` | Market/risk event handling, strategy state machines, chain-sim execution adapters, mined-block rank evidence, decision persistence, position/order lifecycle. | Raw simulation internals, token indexing, direct transaction signing. |
| `pyreth` | Thin Python wrappers and stable schema projection. | Business logic that should live in Rust crates. |
| `tx_executor` | Validate prepared transactions, reserve nonce, enforce fee caps, sign, broadcast, record execution attempts. | Route discovery, quote selection, strategy policy, pool discovery, tx rank estimation. |
| `tx_fund_flow` | Fund-flow network construction, ranking, analytics, visualization. | Core transaction simulation or decoding duplicates. |

## Common Runtime Inputs

Shared defaults live in `config.env` at this repository root. Exported
environment variables with the same names override the file.

Important defaults currently used by local services:

```bash
RETH_DATADIR=/home/nima/storage/samsung8tb/ethereum/reth
RETH_IPC_PATH=/home/nima/storage/samsung8tb/ethereum/reth/reth.ipc
RETH_HTTP_RPC=http://127.0.0.1:8545
RETH_WS_RPC=ws://127.0.0.1:8546
LIVE_BLOCKCHAIN_DATA_REDIS_URL=redis://localhost:6379/0
ETH_PROCESSED_BLOCK_STREAM=eth/live/blocks
PROCESSED_BLOCK_DISK_CACHE_DIR=/home/nima/storage/samsung8tb/ethereum/processed-block-cache
TOKEN_SERVER_BIND=127.0.0.1:8765
MEMPOOL_DATABASE_URL=postgresql://postgres:postgres@localhost:5432/eth_db
MEMPOOL_ZMQ_SIGNAL_ENDPOINT=tcp://127.0.0.1:5556
```

Prefer direct DB access for performance-sensitive paths. RPC is acceptable for
node operations, broadcast, comparisons, and explicit verification examples.

## Useful Commands

Run from this directory:

```bash
cargo check --workspace
cargo run -p tx_simulator --example verify_database_setup
cargo run -p tx_processor --example process_transaction_by_hash -- <tx_hash>
cargo run -p tx_processor --bin live_block_processor
cargo run -p eth_token_server
cargo run -p mempool_processor --bin mempool_signal_detector
```

For PyReth development, use the `pyreth/README.md` workflow. For node setup and
service management, use `node/README.md`.

## Testing And Investigation

Use focused tests/examples near the owner crate:

| Area | Best first checks |
| --- | --- |
| Simulator parity | `tx_simulator/examples/block/verify_block_trace_rpc_equivalence.rs`, `tx_simulator/examples/replay/*` |
| Query parity | `reth_chain_query/examples/block/*`, `reth_chain_query/examples/transactions/*`, `reth_chain_query/examples/reth_index/*` |
| Tx decoding and processed blocks | `tx_processor/tests/`, `tx_processor/examples/processing/*`, `tx_processor/examples/blocks/*` |
| Trade simulation examples | `tx_processor/examples/trade_simulation/*` |
| Token/pool state | `eth_token/tests/`, `eth_token/examples/tracking/token_tracking_range.rs`, `eth_token/examples/validation/*` |
| Live token server | `eth_token_server/README.md`, `logs/eth_token_server/`, `GET /live/status`, `GET /live/pools` |
| Mempool signal behavior | `mempool_processor/examples/signal_detector/*`, `mempool_processor/src/signal_detector/README.md`, `logs/mempool_processor/` |
| Alpha decision loop | `alpha/README.md`, `alpha/store/README.md`, Postgres `alpha_trading.*` tables |
| Token lab cases and strategy cohorts | `token_lab/README.md`, `token_lab/strategy/README.md`, and one case folder under `token_lab/cases/` |

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
  `eth_token_server` views if callers need it.
- New mempool risk signal: detect/rout fast in `mempool_processor`, but consume
  processed transaction facts from `tx_processor` and token context from
  `eth_token_server`.
- New strategy behavior: implement in `alpha/strategies` and persist decision
  evidence through `alpha/store`.
- New Python-facing capability: implement the Rust behavior in its owner crate,
  then expose a thin wrapper in `pyreth`.
- New transaction broadcast mode: add it behind `tx_executor`; strategies should
  still submit intent/prepared transactions through an adapter boundary.

Before adding a new markdown file, ask whether the information belongs in the
nearest folder `README.md`. The root README should stay the navigation map; the
owner folder README should hold the operational details for that folder.
