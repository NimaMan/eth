# Block Processor Profiling

Agent operating map for the Rust-only `ProcessedBlock` profiling and
correctness harness.

## Purpose

- Measure complete `ProcessedBlock` production: DB fetch, local trace replay,
  raw block processing, batch concurrency, correctness, and persistent cache
  warmups.
- Keep current performance findings close to the harness used to reproduce
  them.

## Owns

- Profiling CLI in `main.rs`.
- Scenario configs under `cases/`.
- Generated CSV/results under `results/`.
- Correctness checks comparing fresh processed blocks, candidate engines, cache
  reads, and RPC traces.

## Does Not Own

- Production processing logic; edit `tx_processor/src/block_processor/` and
  `tx_processor/src/processed_tx_provider/block/`.
- Reth trace engine internals; edit `tx_simulator/src/block_trace/`.
- Token-state warmup behavior; edit `eth_token_server` or `eth_token`.

## Data Flow

```text
profile_processed_blocks
  -> BlockProcessor::process_block or process_block_batch
  -> BlockDataFetcher::fetch_db_block_with_traces
  -> local callTracer replay
  -> process_raw_block
  -> timing/correctness/cache rows
```

## Where To Look First

| Need | Start here |
| --- | --- |
| CLI modes and output columns | `main.rs` |
| Production block processing | `tx_processor/src/block_processor/` |
| Persistent cache implementation | `tx_processor/src/processed_tx_provider/block/disk_cache/` |
| Trace engine | `tx_simulator/src/block_trace/` |
| Historical warmup cases | `cases/` |

## Commands

```bash
cargo run --manifest-path tx_processor/Cargo.toml \
  -p tx_processor --release --example profile_processed_blocks -- \
  --datadir /home/nima/storage/samsung8tb/ethereum/reth \
  --start 25028579 \
  --end 25028598 \
  --mode all
```

Cache correctness:

```bash
TX_PROCESSOR_CACHE_CORRECTNESS_DATADIR=/home/nima/storage/samsung8tb/ethereum/reth \
cargo test --manifest-path tx_processor/Cargo.toml \
  -p tx_processor --test processed_block_cache_correctness \
  -- --ignored --nocapture
```

## Current Findings

- All valid `ProcessedBlock` paths require full call traces. There is no valid
  no-trace path when internal ETH transfers or trace-derived balance changes are
  part of the required output.
- Cold single-block processing on range `25028579..25028598` was dominated by
  local EVM replay/tracing: roughly 206-234 ms/block.
- `process_raw_block` alone was about 2-4 ms/block, so optimizing post-replay
  processing cannot provide a 10x cold-path win.
- Cross-block concurrency produced the useful cold-throughput gain. On this
  machine, the stable range was roughly concurrency `16..32`, with `24`
  slightly best in the measured run.
- The persistent processed-block cache is the repeated-warmup 10x path. On the
  measured range, read-only cache hits averaged about 11.6 ms/block, roughly
  20x faster than the no-cache run.

## Current Hazards

- Do not hard-code the best concurrency from one machine. Tune with real
  MDBX/CPU contention.
- Cache hits must remain equivalent to fresh normalized `ProcessedBlock` output.
- Cache layout is
  `processed-block-cache/ethereum-mainnet/<block_number>.pblock.zst`.
  Block hash and trace config hash are validation fields inside the payload.
