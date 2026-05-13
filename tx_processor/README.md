# tx_processor

Agent operating map for turning raw/simulated Ethereum execution into decoded
transaction and block facts.

## Purpose

- Produce `ProcessedTransaction` and `ProcessedBlock` values from tx hashes,
  unsigned calls, raw blocks, receipts, and traces.
- Decode logs/events, internal calls, contract creation, balance deltas, bribes,
  token metadata, and tax/tradability facts.
- Own the persistent processed-block disk cache used by live token tracking and
  historical warmups.

## Owns

- `ProcessedTransaction`, `ProcessedBlock`, compact cache shape, and related
  data models.
- `ProcessedTxProvider` helpers for tx hash and unsigned-call processing.
- `BlockProcessor` and `ProcessedBlockProvider` for block/range processing.
- Log decoding, trace conversion, internal transaction extraction, and address
  balance-change calculation.
- Pool buy/approve/sell viability orchestration in `src/simulator/`.
- `live_block_processor` that publishes compact processed blocks to Redis.

## Does Not Own

- Raw EVM execution internals; use `tx_simulator`.
- Pure DB query helpers and calldata builders; use `reth_chain_query`.
- Long-lived token/pool registry state; use `eth_token`.
- HTTP serving, strategy policy, mempool ingestion, or transaction signing.

## Data Flow

```text
tx hash / unsigned tx / raw block
  -> tx_simulator full trace or reth_chain_query raw block fetch
  -> TxProcessor / BlockProcessor
  -> decoded logs + internal calls + balance deltas + metadata/errors
  -> ProcessedTransaction / ProcessedBlock
  -> eth_token, mempool_processor, pyreth, tx_fund_flow, alpha live feed
```

Live path:

```text
live_block_processor
  -> compact ProcessedBlock payload
  -> ProcessedBlockReplayStoreWriter
  -> processed-block disk cache + address_to_blocks
  -> Redis stream eth/live/blocks
```

Backfill path:

```text
refresh_processed_block_disk_cache
  -> load existing ProcessedBlock cache or process missing blocks
  -> ProcessedBlockReplayStoreWriter
  -> processed-block disk cache + address_to_blocks
```

## Where To Look First

| Need | Start here |
| --- | --- |
| Public API and re-exports | `src/lib.rs` |
| Transaction decoding pipeline | `src/tx_processor/` |
| Process tx by hash/unsigned tx | `src/processed_tx_provider/` |
| Process block/ranges | `src/block_processor/`, `src/processed_tx_provider/block/` |
| Persistent block cache | `src/processed_tx_provider/block/disk_cache/` |
| Live Redis publisher | `src/bin/live_block_processor/README.md` |
| Buy/sell/tax simulation | `src/simulator/`, `examples/pool_analysis/` |
| Profiling harness | `examples/block/profile/README.md` |

## Tests And Commands

```bash
cargo run -p tx_processor --example process_transaction_by_hash -- <tx_hash>
cargo run -p tx_processor --example process_block -- --block <block>
cargo run -p tx_processor --bin live_block_processor
cargo run -p tx_processor --release --example refresh_processed_block_disk_cache -- --blocks 100000
cargo run -p tx_processor --release --example profile_processed_blocks -- --mode all
cargo test -p tx_processor
```

## Current Hazards

- Full `ProcessedBlock` output requires call traces; there is no valid no-trace
  path when internal ETH transfers or trace-derived balance deltas are needed.
- Cold block processing is dominated by EVM replay/tracing. Optimize cache and
  cross-block concurrency before micro-optimizing post-processing.
- Cache shape changes must preserve deterministic equality between fresh and
  cached normalized `ProcessedBlock` output.
- Do not duplicate token registry state here. Emit decoded facts; let
  `eth_token` maintain durable token/pool state.
