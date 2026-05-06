# Replay Profiling

This folder is for replay-only investigations inside `tx_simulator`. It focuses
on making local block replay faster while still producing correct call traces.

The `tx_processor/examples/block/profile` harness measures complete
`ProcessedBlock` production. This folder isolates the replay engine:

- local Reth DB state replay,
- call-trace generation,
- state-provider miss behavior below `CacheDB`,
- EVM environment construction,
- transaction sender recovery and tx env creation,
- inspector overhead,
- contiguous-range replay,
- node-integrated replay hooks.

Out of scope:

- Python token state application,
- token metadata fetching,
- `ProcessedBlock` assembly after traces exist,
- tx_processor block-level concurrency,
- cache speedups for repeated ranges,
- disabling traces for production paths.

## Layout

```text
examples/replay/profile/
  README.md
  main.rs
  cases/
    replay_engine_sweep.toml
  results/
    .gitkeep
```

## Baseline

Current production-safe replay:

```text
BlockTracer::trace_block_by_number
  -> resolve block hash
  -> load full block by hash
  -> open history state at parent hash
  -> wrap StateProviderDatabase in CacheDB
  -> execute txs sequentially
  -> build one callTracer result per tx
  -> commit tx state to CacheDB
```

Transactions inside one block remain sequential because each transaction depends
on state writes from previous transactions. The safe parallel unit is the block.

Runnable harness:

```text
cargo run --manifest-path tx_simulator/Cargo.toml \
  -p tx_simulator --release --example profile_replay_engines -- \
  --datadir /path/to/reth \
  --start 25028579 \
  --end 25028598 \
  --mode engine-sweep
```

Useful modes:

- `engine-sweep`: run `baseline-fresh`, `tracing-fused`, and `reth-debug`.
- `stage-breakdown`: run one `--engine`.
- `correctness`: compare candidate traces against `baseline-fresh`.
- `--per-tx`: emit transaction-level timing rows.

CSV includes:

```text
total_ms, block_hash_lookup_ms, block_load_ms, state_open_ms,
sender_recovery_ms, evm_env_ms, tx_env_ms, inspector_build_ms,
evm_exec_ms, trace_build_ms, db_commit_ms, account_reads,
storage_reads, code_reads, block_hash_reads, provider_read_ms
```

From processed-block profiling on `25028579..25028598` before this harness:

| Component | Observation |
| --- | --- |
| Full single-block processing | about `206 ms/block` |
| Trace-only replay | about `218 ms/block` fresh-inspector |
| `process_raw_block` after raw data exists | about `2.5 ms/block` |
| Fused inspector candidate | measured directly in this harness as `tracing-fused` |
| Cross-block concurrency | roughly `28-33 ms/block` around concurrency `16..32` |

Replay dominates cold processing. Post-replay processing is too small to produce
an order-of-magnitude cold-path win.

## Current Measurements

Range: `25028579..25028598`
Iterations: 3
Command: `profile_replay_engines --mode engine-sweep`

Correctness passed for `tracing-fused` and `reth-debug` against
`baseline-fresh` on all 20 blocks after fixing the fresh inspector path to set
transaction gas limit/caller before building the call trace.

| Engine | Avg ms/block | Median | P95 | Max | Trimmed avg | Finding |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| `baseline-fresh` | 925.3 | 247.9 | 7320.4 | 9118.3 | 418.2 | Fresh inspector has high tail stalls |
| `tracing-fused` | 274.3 | 212.3 | 989.2 | 1311.6 | 228.4 | Best current cold replay engine |
| `reth-debug` | 1467.0 | 203.6 | 10534.2 | 28830.1 | 301.5 | Correct but severe tail stalls |

The averages include large storage-provider tail outliers. Median/trimmed avg
are more representative for steady replay, while p95/max show the operational
risk.

Average stage breakdown:

| Engine | EVM exec ms | Provider read ms | Sender recovery ms | Trace build ms | Avg provider misses |
| --- | ---: | ---: | ---: | ---: | ---: |
| `baseline-fresh` | 866.2 | 781.3 | 46.0 | 3.9 | 2985 |
| `tracing-fused` | 259.4 | 234.0 | 12.3 | 0.8 | 2985 |
| `reth-debug` | 1399.2 | 1288.3 | 59.1 | 2.8 | 2985 |

Provider misses averaged about `702` account reads, `1968` storage reads, `315`
code reads, and less than `1` block-hash read per block. The top bottleneck is
not `process_raw_block`, env construction, trace build, or DB commit; it is EVM
execution dominated by state-provider reads.

## Lower Bound

For cold, correct call traces, the block must be executed. The lower bound is:

```text
time to execute all txs with state reads/writes
+ minimum call-trace capture work
+ minimum result materialization work
```

The only ways to beat this lower bound for first-time blocks are:

- capture traces during node block import when execution is already happening,
- reduce state-provider miss cost during replay,
- reduce trace work while preserving the fields needed by `ProcessedBlock`,
- reduce the trace correctness target.

The last option is not acceptable for current `ProcessedBlock` parity.

## Bottleneck Map

### State reads

Every tx execution touches accounts, storage slots, bytecode, and block
hash/history lookups. `CacheDB` helps inside one replay, but first touches still
reach the provider.

Possible work:

- prefetch predictable accounts/storage/code from access lists,
- retain warmed state across adjacent historical blocks,
- test a contiguous-range replay overlay,
- benchmark alternative state wrappers and Reth native executor paths.

### EVM execution

Execution is unavoidable, but surrounding overhead may be reducible.

Possible work:

- avoid rebuilding reusable env/config objects,
- avoid unnecessary transaction clones,
- reuse recovered senders and tx env data,
- compare Reth native executor/debug internals against the manual loop.

### Inspector and trace capture

The current output requires a full call tree. The `tracing-fused` candidate is
correct and is currently the fastest measured replay engine.

Possible work:

- keep testing fused tracing on wider block classes,
- implement a minimal internal-transfer tracer as a separate correctness target,
- build `TransactionTrace` directly from replay without intermediate
  `GethTrace` materialization,
- return call frames and internal ETH transfers from replay in one pass.

### Contiguous range replay

For adjacent historical blocks, a candidate engine can open state at
`start - 1`, replay block `start`, keep the overlay, and replay `start + 1`.

Potential upside:

- fewer state opens,
- warmer state across adjacent blocks.

Potential downside:

- loses cross-block parallelism,
- overlay memory may grow quickly,
- more complex recovery and reset behavior.

This should be tested against parallel open-per-block replay, not assumed faster.

### Node-time processing

For live blocks, the highest-upside path is capturing processed/replay data
while the node is already executing/importing the block. ExEx may not expose full
call traces after the fact; executor-level hooks are more invasive but can avoid
duplicate replay.

## Candidate Engines

- `BaselineFresh`: current default, one inspector per tx.
- `TracingFused`: current best measured engine; reuses one call tracer inspector.
- `RethDebug`: mirrors Reth `DebugInspector`; correct but high-tail on this run.
- `RecoveredBlockReplay`: accept preloaded recovered txs to avoid duplicate
  block load/sender recovery.
- `AccessListPrefetchReplay`: prefetch access-list accounts and storage.
- `ContiguousRangeReplay`: replay adjacent blocks from a retained overlay with
  periodic reset.
- `MinimalInternalTransferTracer`: record only fields needed for internal ETH
  transfers and contract creation markers; separate correctness target.
- `NativeRethExecutorReplay`: test whether Reth executor/debug internals can use
  a more efficient state path than the current manual loop.

## Experiment Sequence

1. Investigate why `reth-debug` and `baseline-fresh` had provider-read tail
   stalls while `tracing-fused` was stable.
2. Test contiguous range replay to reuse the warmed overlay across adjacent
   blocks.
3. Test access-list and sender/touched-account prefetching before EVM execution.
4. Build a direct `ProcessedBlock` integration using `tracing-fused`, then verify
   full `ProcessedBlock` equality in `tx_processor`.
5. Test a minimal internal-transfer tracer only as a separate correctness target.

Current recommendation: use `tracing-fused` as the candidate cold replay engine
and focus the next optimization on reducing provider miss cost. Cache is not a
first-time block processing solution and is intentionally not used as evidence in
this folder.
