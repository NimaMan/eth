# tx_processor Examples

Examples are grouped by the workflow they exercise. They are registered in
`tx_processor/Cargo.toml` and are expected to compile with:

```bash
cargo check -p tx_processor --examples
```

## Groups

| Directory | Purpose |
| --- | --- |
| `tx_processor/` | Process individual transactions by hash or unsigned tx payload. |
| `block/` | Process blocks, compare block sources, refresh processed-block caches, and profile throughput. |
| `provider/` | Exercise provider-level block and token transaction reads. |
| `pool_analysis/` | Run trade viability simulations against known pools and probe specific pool setups. |
| `token_parameter_extraction/` | Extract token and denomination metadata needed by higher-level workflows. |

## Common Environment

Most examples need a synced local Reth datadir. Prefer `RETH_DATADIR`; several
older examples still fall back to `~/.local/share/reth/mainnet`.

```bash
RETH_DATADIR=/path/to/reth/mainnet cargo run -p tx_processor --example process_block -- --block 19000000
```

Pool analysis examples use the public `tx_processor::trade_simulation` API.
Transaction reconstruction helpers live under `tx_processor::processed_tx_builder`.
