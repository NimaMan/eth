# State Access Replay Experiments

This folder tracks investigations for reducing historical state-read cost during
block replay. It belongs under `tx_simulator` because the bottleneck sits below
the simulator's REVM execution loop:

```text
BlockTracer
  -> StateProviderDatabase
  -> CacheDB / State overlay
  -> REVM Database reads during execution
```

`reth_chain_query` is still the right crate for reusable chain data queries:
headers, blocks, receipts, token metadata reads, and higher-level Reth-backed
lookups. This work is narrower. It studies how simulation loads account,
storage, code, and block-hash state while replaying transactions.

If an experiment becomes a reusable state-query abstraction used outside the
simulator, move that production piece to `reth_chain_query`. If it only exists
to make replay faster or more correct, keep it in `tx_simulator`.

## Current Evidence

The replay profiler already shows that cold block replay is state-read
dominated:

- full call-trace replay median is about `200 ms/block` on the sampled range,
- execute-only replay without trace is still about `194 ms/block`,
- trace construction and post-replay block processing are small by comparison,
- oracle prewarm can replay after preload in about `14 ms/block`,
- oracle prewarm including preload is still about `148 ms/block`,
- provider misses average about `702` account reads, `1968` storage reads, and
  `315` code reads per block on the sampled range.

That means the main performance question is not "can we skip tracing?" The main
question is whether the state needed by execution can be available before replay
starts, or fetched with less first-touch latency.

## Layout

```text
examples/replay/state_access/
  README.md
  main.rs
  cases/
  results/
```

Use `cases/` for checked-in experiment inputs and `results/` for small retained
summaries. Large profiler output should stay out of git unless it is reduced to
a short report.

## Harnesses

Replay-only state-access report:

```text
cargo run --manifest-path Cargo.toml \
  -p tx_simulator --release --example profile_state_access_range -- \
  --datadir /path/to/reth \
  --start 25028579 \
  --end 25028598 \
  --profile-kind tracing-fused \
  --concurrency 24
```

End-to-end `ProcessedBlock` throughput report:

```text
cargo run --manifest-path Cargo.toml \
  -p tx_processor --release --example profile_processed_block_range_throughput -- \
  --datadir /path/to/reth \
  --start 25028579 \
  --end 25028598 \
  --mode cold-sweep \
  --concurrencies 1,4,8,16,24,32,48
```

The `ProcessedBlock` harness lives in `tx_processor` because `tx_processor`
owns `ProcessedBlock` and depends on `tx_simulator`. Putting that binary under
`tx_simulator` would create a Cargo dependency cycle. The state-access
investigation remains anchored here, and production state-access code should
still stay in `tx_simulator` unless it becomes a generic Reth query API.

## Experiment Tracks

### Oracle prewarm baseline

Already implemented by `profile_replay_engines --record-keys` and the
`oracle-prewarm` engine. This is not a real production engine because it knows
the exact keys only after a previous replay. It is useful because it measures the
maximum upside from eliminating first-touch provider misses.

Accept if:

- correctness matches `baseline-fresh`,
- preload time and replay-after-preload time are reported separately,
- the result is treated as a bound, not a deployable strategy.

### Access-list prefetch

Prefetch the accounts and storage slots already declared by EIP-2930 access
lists, plus obvious transaction-level accounts such as sender, recipient,
created contract address, coinbase, and known system contracts.

Questions:

- What fraction of provider misses are covered by access lists?
- Does prefetching reduce total wall time, or does it only move reads earlier?
- Does it reduce p95/max tails?

Accept if:

- full traces match `baseline-fresh`,
- total time includes prefetch,
- reports separate hit coverage for account, storage, and code reads.

### Prior-block key prefetch

For contiguous historical ranges, use keys observed in recent adjacent blocks as
a heuristic prefetch set for the next block. This is a realistic version of the
oracle experiment when contracts and storage are repeatedly touched across a
range.

Questions:

- How much key reuse exists across nearby blocks?
- What window size gives useful coverage without excessive preload cost?
- Does it help 100K-block range builds when concurrency is already high?

Accept if:

- no correctness change,
- bounded memory growth,
- measured against open-per-block parallel replay.

### Contiguous range overlay

Open parent state once at `start - 1`, replay `start`, keep the overlay, then
replay `start + 1`. This can reuse warm state across blocks, but it trades away
some cross-block parallelism and may grow memory quickly.

Questions:

- Does retained overlay beat independent per-block replay at realistic
  concurrency?
- How often should the overlay reset?
- Can failure recovery restart from the nearest persisted block boundary?

Accept if:

- state root or trace equivalence is checked at reset boundaries,
- memory is measured,
- comparison uses the same block range and output target as current processing.

### Native Reth state path

Compare the current manual replay loop against Reth's own executor/debug paths.
This should stay focused on whether Reth has a lower-latency state access path or
better caching behavior for historical state.

Questions:

- Does Reth's native executor reduce provider misses or only reorganize the same
  reads?
- Can we preserve the exact trace fields needed by `ProcessedBlock`?
- Is the integration surface stable enough to maintain?

Accept if:

- full trace output is equivalent,
- pre-execution changes and per-tx state commits match Reth behavior,
- maintenance cost is documented before moving into production code.

## Suggested Commands

Record replay keys and oracle-prewarm timing:

```text
cargo run --manifest-path Cargo.toml \
  -p tx_simulator --release --example profile_replay_engines -- \
  --datadir /path/to/reth \
  --start 25028579 \
  --end 25028598 \
  --mode feasibility \
  --record-keys \
  --warmup-iterations 1 \
  --discard-first
```

Profile full processed-block cache refresh:

```text
cargo run --manifest-path Cargo.toml \
  -p tx_processor --release --example refresh_processed_block_disk_cache -- \
  --reth-datadir /path/to/reth \
  --start 25000000 \
  --blocks 100000 \
  --chunk-size 250 \
  --fill-batch-blocks 250 \
  --fill-concurrency 4 \
  --retain-blocks 1000000
```

## Reporting

Every result should include:

- block range and datadir type,
- engine name,
- median, p95, max, and total wall time,
- account, storage, code, and block-hash provider reads,
- preload time if any,
- replay time after preload if any,
- correctness target used for comparison.

For range builds, report both per-block latency and end-to-end throughput. A
single-block optimization only matters if it improves the 100K-block processing
case or reduces tail risk without breaking trace parity.
