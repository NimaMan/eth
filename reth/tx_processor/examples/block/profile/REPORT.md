# Processed Block Pipeline Assessment

## Objective

The target object is `tx_processor::ProcessedBlock`: a block header plus one
`ProcessedBlockTransactions` entry per transaction. A complete processed block
needs transaction metadata, receipts/logs, decoded transaction data, internal
transactions from call traces, balance changes, and contract creation markers.

Because internal transactions come from call traces, `include_traces = false`
is only a diagnostic comparison. It is not a valid optimization for the
production warmup path.

## Implementation Status: 2026-05-06

This folder now contains a Rust-only executable profiler:

```text
cargo run --manifest-path tx_processor/Cargo.toml \
  -p tx_processor --release --example profile_processed_blocks -- \
  --datadir /home/nima/storage/samsung8tb/ethereum/reth \
  --start 25028579 --end 25028598 --mode all
```

Implemented modes:

- `db-current`
- `db-baseline-fresh`
- `trace-only-fresh`
- `trace-only-fused`
- `fetch-only`
- `process-raw-only`
- `batch-sweep`
- `correctness`
- `rpc-trace`

The profiler emits CSV rows with block count, tx count, gas used, trace node
count, internal tx count, processing errors, total time, DB load stage timings,
trace time, and `process_raw_block` stage timings.

The first optimized trace candidate is implemented as
`BlockTraceEngine::RethFusedCallTracer`. It is available for experiments, but
the production default remains the original `FreshInspector` path because fused
tracing did not measure faster on the initial block set.

The DB raw fetch path now passes already-loaded transaction metadata into trace
conversion, removing one duplicated metadata fetch during trace conversion.

`BlockProcessor::process_raw_block_profiled` is available for this harness and
returns timings for:

- raw transaction processing,
- trace conversion,
- internal transaction extraction,
- balance calculation,
- contract creation derivation,
- total raw processing.

## Preliminary Measurements

Range: `25028579..25028598`  
Iterations: 3 unless noted  
Datadir: `/home/nima/storage/samsung8tb/ethereum/reth`

| Scenario | Avg ms/block | Median | P95 | Max | Finding |
| --- | ---: | ---: | ---: | ---: | --- |
| Fresh inspector processed block | 206.7 | 193.6 | 332.3 | 336.2 | Current single-block baseline |
| Fused inspector processed block | 210.5 | 198.9 | 335.3 | 351.0 | No measured win |
| Trace-only fresh | 217.8 | 203.8 | 359.3 | 380.0 | Replay dominates |
| Trace-only fused | 251.5 | 242.4 | 444.8 | 576.7 | Slower in latest run |
| `process_raw_block` only | 2.5 | 2.1 | 5.0 | 6.7 | Not the bottleneck |

Single-block stage example for block `25028579`:

| Stage | Time |
| --- | ---: |
| DB header load | 0.045 ms |
| DB tx metadata load | 6.248 ms |
| DB receipt load | 0.622 ms |
| Local trace replay | 257.494 ms |
| Raw tx processing | 1.161 ms |
| Trace conversion | 0.216 ms |
| Internal tx extraction | 0.468 ms |
| Balance calculation | 1.216 ms |
| `process_raw_block` total | 3.894 ms |

The key conclusion is that optimizing post-replay processing cannot produce a
10x cold-processing win. The cold path is dominated by EVM replay/tracing.

### Batch Sweep

For the same 20 blocks with batch size `20`, cross-block concurrency produced
the only near-10x throughput result:

| Concurrency | Stable avg ms/block |
| ---: | ---: |
| 1 | ~209-216 |
| 2 | ~104-111 |
| 4 | ~58 |
| 8 | ~40-42 |
| 10 | ~35-36 |
| 16 | ~33 |
| 24 | ~28-29 |
| 32 | ~29-31 |

One full sweep also hit first-iteration outliers at concurrency `4`, `8`, and
`10`, so this must be treated as an MDBX/CPU scheduling problem, not a fixed
"higher is always better" rule. On this machine and block set, the useful range
appears to be roughly `16..32`, with `24` slightly best in stable iterations.

### Correctness

The correctness oracle passed for `25028579..25028598`: normalized
`ProcessedBlock` output from the fresh baseline matched the fused tracer
candidate for all 20 blocks, including traces, internal transactions, balance
changes, receipts/logs, metadata, and processing errors.

## Current Pipeline

### Historical DB Path

This is the best current path for historical warmup when a local Reth database
is available.

```text
ProcessedTxProvider::process_block
  -> BlockProcessor::process_block
  -> BlockProcessor::process_block_with_options(include_traces = true)
  -> BlockDataFetcher::fetch_db_block_with_traces
  -> RethQueryProvider::fetch_raw_block_data
      -> fetch header from MDBX
      -> fetch transaction metadata from MDBX
      -> fetch receipts/logs from MDBX
      -> simulate block traces locally
  -> BlockProcessor::process_raw_block
      -> decode logs and classify transaction
      -> convert call trace
      -> extract internal transactions
      -> calculate balance changes
      -> derive contract creation address
```

Relevant code:

- `reth/tx_processor/src/block_processor/mod.rs`
- `reth/reth_chain_query/src/provider/block/block_data_fetcher.rs`
- `reth/reth_chain_query/src/provider/block/db_fetcher.rs`
- `reth/tx_simulator/src/block_trace/block_tracer.rs`

### Batch Path

`BlockProcessor::process_block_batch` runs multiple independent blocks in
parallel with `buffer_unordered`, then sorts results by block number before
returning them. The default options are:

```text
include_traces = true
max_concurrency = 10
```

This is cross-block concurrency. It does not parallelize transactions inside one
block.

### Live RPC Path

The live Rust processor subscribes to `newHeads`, fetches the block and receipts
over RPC, calls `debug_traceBlockByNumber` with `callTracer`, then runs the same
`process_raw_block` stage.

```text
newHeads websocket
  -> eth_getBlockByHash / eth_getBlockByNumber
  -> eth_getBlockReceipts
  -> debug_traceBlockByNumber(callTracer)
  -> process_raw_block
```

This path is useful for immediately processing new blocks before relying on
local DB persistence. It pays RPC and JSON overhead.

### Python/PyReth Path

PyReth exposes `process_block` and `process_block_batch` to Python. The batch
method delegates to Rust, gets ordered processed blocks back, then Python can
apply token state sequentially. This is the right separation: Rust performs
block fetch/trace/process concurrently; Python preserves token state semantics.

## Trace Constraint

For a full processed block, traces require replaying the block. The current
local tracer:

1. Opens state at the parent block.
2. Wraps it in `CacheDB`.
3. Executes transactions in order.
4. Commits each transaction's state into the cache for the next transaction.
5. Emits a `callTracer` call frame per transaction.

Transactions inside one block are state-dependent, so they cannot be naively
parallelized. The safe parallel unit is the block, not the transaction.

## Fetching Options

### 1. Local DB + Local Call Tracing

This is the preferred historical path.

Pros:

- No RPC network latency.
- No JSON serialization/deserialization for traces.
- Uses local Reth state and Reth EVM crates.
- Works well with cross-block batching.

Cons:

- Still must execute every transaction in the block.
- Contiguous block ranges can put real load on MDBX and CPU.
- Current implementation duplicates some DB reads.

Conclusion: this should remain the default for historical warmup.

### 2. RPC `debug_traceBlockByNumber`

This is the fallback/live path.

Pros:

- Works without direct MDBX access.
- Good for latest heads where local DB availability/canonicalization may lag.
- One block trace RPC is much better than tracing each transaction separately.

Cons:

- Large JSON payloads.
- Network and RPC server overhead.
- Response size limits and timeout sensitivity.
- Less control over CPU scheduling and tracing internals.

Conclusion: acceptable for live or fallback, not the fastest historical path.

### 3. RPC `debug_traceTransaction` Per Transaction

This should not be used for whole processed blocks.

Each transaction trace must replay prior transactions in the block to get the
correct state. Doing this for every transaction repeats work and adds one RPC
round trip per transaction.

Conclusion: only use for one targeted transaction, never for block warmup.

### 4. DB-Only Without Traces

This gives metadata, receipts, logs, decoded ERC20 events, and contract creation
addresses, but misses internal ETH transfers and trace-derived balance changes.

Conclusion: useful for measuring the lower bound, not valid for complete
processed blocks.

### 5. Processed Block Cache / Snapshot Store

If a processed block has already been computed, the fastest path is to read the
processed block from a cache keyed by:

```text
chain_id
block_number
block_hash
processor_schema_version
trace_config
```

This can be Redis for recent live blocks and a persistent local store for
historical warmup ranges.

Conclusion: this is the fastest repeated-access path and should be considered a
separate product feature from raw block processing.

### 6. Native Live Processing During Import

The theoretical fastest live path is to produce the processed block while the
execution client is already executing/importing the block. That avoids replaying
the same block after import.

Pros:

- Avoids duplicate EVM execution for live blocks.
- Can emit processed-block snapshots immediately.
- Removes RPC trace overhead entirely.

Cons:

- Requires deeper integration with the Reth node/import pipeline.
- More reorg/canonicalization complexity.
- Higher maintenance cost against Reth internals.

Conclusion: high upside for live blocks, but not the first optimization for the
current warmup problem.

### 7. Contiguous Historical Range Replayer

For a contiguous range, another possible design is to open state at
`start_block - 1`, replay block `N`, keep the overlay, then replay block
`N + 1` from that state instead of reopening state at every parent block.

Pros:

- Reduces repeated parent-state setup.
- Could improve sequential historical scans.

Cons:

- Loses most cross-block parallelism.
- The overlay can grow large over thousands of blocks.
- More complex reorg/error recovery.
- Unknown win because EVM execution still dominates.

Conclusion: benchmark only after fused inspector and duplicate-fetch cleanup.

### 8. Indexed Block Selection

When the goal is not every block, existing account-history indexes can reduce
the number of blocks that need full processing. The address and token providers
already use this shape before calling `process_block_batch`.

Conclusion: useful for entity-specific queries, not for full warmup ranges where
every block must be processed.

## Current Bottlenecks

### Trace Simulation Dominates

Previous profiling of `25028579..25028598` showed traced block processing was
orders of magnitude slower than no-trace block processing. That is expected:
without traces we only fetch/decode stored data; with traces we replay EVM
execution.

The important question is no longer whether traces are expensive. They are. The
question is how much duplicate work exists around the required replay.

### Duplicate DB Work

`fetch_raw_block_data` currently fetches:

- header once directly,
- transaction metadata once,
- receipts, which fetch header and transactions again,
- traces, which fetch transaction metadata again,
- block tracer, which fetches the full block again.

The no-trace path is already relatively cheap, so this may not be the largest
cost, but it is real work and it is easy to measure. A better raw fetch shape is:

```text
fetch block/header/transactions once
fetch receipts once
build metadata and receipts from those shared values
pass already-known transaction gas limits into trace conversion
```

### Fresh Inspector Per Transaction

`BlockTracer::trace_single_transaction` creates a new `TracingInspector` for
each transaction. Another simulator path, `tx_chain/sequential.rs`, already uses
a fused inspector pattern across transactions.

For block tracing, we should test a fused-inspector version:

```text
create callTracer inspector once
for each tx:
  execute tx
  extract this tx call frame
  commit state
  fuse inspector for next tx
```

This experiment has now been implemented as
`BlockTraceEngine::RethFusedCallTracer`. Initial results do not justify making
it the production default. Keep it as a candidate and rerun on other block
classes before deleting or promoting it.

### Trace Conversion and Cloning

The current path materializes a provider `TransactionTrace`, then converts it
back into a tx-simulator call frame so `TransactionTraceProcessor` can extract
internal transactions. This copies call frames, bytes, and nested subcalls.

A faster shape would either:

- keep the call trace in one canonical type, or
- extract internal transactions directly from the tracer output, or
- build `ProcessedBlockTransactions` during the trace pass.

This should be tested after the fused-inspector experiment, because trace
execution is likely larger than conversion.

## Recommended Fastest Path

### Historical Warmup

Use:

```text
local Reth DB
local callTracer simulation
process_block_batch
include_traces = true
bounded cross-block concurrency tuned per machine
Python token application sequential after Rust returns ordered blocks
```

The reported batch-size sweep:

```text
batch 20  -> ~45.9 ms/block
batch 40  -> ~42.1 ms/block
batch 80  -> ~38.7 ms/block
batch 120 -> ~39.8 ms/block
batch 240 -> ~47.6 ms/block
```

supports batch size `80` as a reasonable working default, but the Rust-only
sample above suggests concurrency is the stronger lever than batch size.
Batch size is not the same as block concurrency: batch size controls how many
blocks Python hands to Rust at once; Rust concurrency controls how many blocks
are actively processed at once.

### Live Blocks

If a processed block is already published by the live block processor, consume
that processed snapshot instead of recomputing it in Python. That moves the
trace cost to one upstream service and makes downstream token processing much
cheaper.

If the live service itself needs the block immediately, RPC by block hash is
reasonable. If a small delay is acceptable, test a hybrid live path that waits
for local Reth persistence and then uses the local DB path. The faster choice
depends on canonicalization delay versus RPC trace overhead.

### Repeated Historical Access

Build a persistent processed-block cache. For repeated warmups or overlapping
consumers, this beats every fetch/trace strategy because it avoids replaying EVM
execution entirely.

## Experiments To Run Next

1. Baseline current local DB path:

```text
include_traces=true
batch_concurrency=[1,2,4,10,20]
batch_sizes=[20,40,80,120,240]
iterations=3
```

2. Add detailed timings:

```text
header_fetch_ms
tx_metadata_fetch_ms
receipt_fetch_ms
trace_sim_ms
trace_convert_ms
process_raw_ms
log_decode_ms
internal_extract_ms
balance_calc_ms
```

3. Compare DB local versus RPC block trace on the same blocks.

4. Implement and benchmark fused inspector in `BlockTracer`.

5. Remove duplicated raw fetches and benchmark again.

6. Test direct trace-to-processed-block construction to avoid intermediate
trace conversion.

7. Test processed-block cache lookup for repeated warmup ranges.

Items 1, 2, 4, and part of 5 now have initial implementations. Items 3, 6, and
7 remain open.

## Decision Table

| Need | Best Path |
| --- | --- |
| Historical full processed blocks | Local DB + local callTracer + block batch |
| Live immediate block | RPC by block hash + `debug_traceBlockByNumber` |
| Live downstream consumers | Consume already published processed snapshot |
| Repeated warmup or overlapping consumers | Persistent processed-block cache |
| One transaction only | Targeted transaction trace |
| Diagnostic lower bound | DB-only, `include_traces=false` |
| Entity-specific history | RethIndex/account-history filter, then block batch |
| Future fastest live path | Native processing during block import |

## Main Recommendation

Treat batching/concurrency as the current throughput fix, not the final answer.
The next best improvements are inside the block processor:

1. keep the fresh inspector as default until a candidate beats it,
2. finish duplicate DB read cleanup,
3. avoid unnecessary trace type conversions and clones,
4. add a processed-block cache for repeated access,
5. investigate node-time processing for live blocks.

These preserve traces and target the actual cost of producing complete processed
blocks.
