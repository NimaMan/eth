# Block Processor Profiling

This folder contains experiments for measuring block processor performance.
The focus is block-level cost: fetching raw block data, fetching traces,
processing transactions, and comparing sequential versus batched block
processing.

Use this folder for experiments that answer questions like:

- How long does traced block processing take per block?
- How much time is saved by `process_block_batch`?
- Which batch size and concurrency are useful for historical warmup?
- Are slow blocks caused by traces, receipts, transaction count, contract
  creation, or later token metadata work?

## Layout

```text
examples/block/profile/
  README.md
  REPORT.md
  main.rs
  cases/
    historical_warmup_20_blocks.toml
    warmup_batch_size_sweep.toml
  results/
    .gitkeep
```

Current code layout:

```text
main.rs
```

## Investigation Order

Start with the block processor itself. Do not begin from the Python token
manager, because batching can hide several different effects:

1. Measure sequential block processing with traces enabled.
2. Measure `process_block_batch` for the same block list.
3. Sweep batch size and concurrency separately.
4. Split total time into fetch/trace time versus decoding/processing time.
5. Only after the block processor numbers are clear, profile token metadata and
   token state application outside this folder.

`include_traces = false` is useful only as a diagnostic comparison. It is not an
acceptable optimization for the live warmup path because the processor needs
traces.

## Current Hypothesis

Another run reported these warmup batch-size results:

```text
batch 20  -> ~45.9 ms/block
batch 40  -> ~42.1 ms/block
batch 80  -> ~38.7 ms/block
batch 120 -> ~39.8 ms/block
batch 240 -> ~47.6 ms/block
```

That suggests batch size `80` may be near the useful range, but this folder
should verify it with repeatable inputs before treating it as a stable default.
The key check is whether the improvement comes from actual block processor
throughput or from overlapping trace fetch latency.

## Canonical Example

`Cargo.toml` exposes this folder as:

```text
cargo run --manifest-path tx_processor/Cargo.toml \
  -p tx_processor --release --example profile_processed_blocks -- \
  --datadir /path/to/reth \
  --start 25028579 \
  --end 25028598 \
  --mode all
```

CSV output includes coarse totals plus stage columns:

```text
fetch_ms
trace_ms
process_raw_ms
block_load_ms
tx_load_ms
receipt_load_ms
raw_tx_process_ms
trace_convert_ms
internal_extract_ms
balance_calc_ms
```

See `REPORT.md` for the current pipeline assessment and recommended
optimization experiments.
