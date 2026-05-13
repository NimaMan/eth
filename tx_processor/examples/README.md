# tx_processor Examples

Examples are organized by the public workflow they exercise. All registered
examples should compile with:

```bash
cargo check -p tx_processor --examples
```

## Layout

| Directory | Purpose |
| --- | --- |
| `processing/` | Process one transaction or synthetic unsigned transaction into `ProcessedTransaction` facts. |
| `blocks/` | Process blocks, compare block sources, refresh disk caches, and profile block throughput. |
| `providers/` | Exercise provider wrappers for processed txs, processed blocks, and token transaction ranges. |
| `trade_simulation/viability/` | Durable buy -> approve -> sell viability examples for known protocols and fixtures. |
| `trade_simulation/probes/` | Parameterized diagnostic CLIs for one-off pool/sell/V4 investigations. |
| `trade_simulation/fixtures/` | Shared token sets used by viability examples. |
| `metadata/` | Token metadata examples, including replaying prior txs before metadata calls. |

## Common Environment

Most examples need a synced local Reth datadir. Prefer `RETH_DATADIR`; several
older examples still fall back to `~/.local/share/reth/mainnet`.

```bash
RETH_DATADIR=/path/to/reth/mainnet cargo run -p tx_processor --example process_block -- 19000000
```

Trade simulation examples use `tx_processor::trade_simulation`. Transaction
rebuild helpers live under `tx_processor::processed_tx_builder`.
