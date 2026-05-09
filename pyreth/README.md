# pyreth

Agent operating map for Python bindings over the Rust ETH stack.

## Purpose

- Expose selected Rust capabilities to Python through PyO3/maturin.
- Provide thin wrappers around chain query, transaction processing, simulation,
  DEX helpers, and shared provider instances.
- Preserve stable Python-facing schemas without moving business logic into the
  binding layer.

## Owns

- PyO3 module registration in `src/python.rs`.
- Shared instance/provider management in `src/pyreth_instance.rs`.
- Python wrappers for `reth_chain_query`, `tx_processor`, `tx_simulator`, and
  DEX/simulator helpers.
- Python examples that demonstrate supported wrapper surfaces.

## Does Not Own

- Core Rust behavior. Implement the feature in the owner crate first, then wrap
  it here.
- Transaction decoding, token state, mempool signals, strategy policy, or
  execution.
- Independent schemas that diverge from Rust `ProcessedTransaction` /
  `ProcessedBlock` contracts.

## Data Flow

```text
Python caller
  -> pyreth PyO3 wrapper
  -> shared Rust provider/simulator/query/processor instance
  -> Python dict/dataclass-friendly result
```

## Where To Look First

| Need | Start here |
| --- | --- |
| Module registration | `src/python.rs` |
| Shared runtime/provider state | `src/pyreth_instance.rs` |
| Chain query wrappers | `src/chain_query/` |
| Provider/block examples | `examples/provider/` |
| Processed tx schema | `src/tx_processor/`, `src/tx_processor/README.md` |
| Simulator wrappers | `src/simulator/` |
| Example index | `examples/README.md` |

## Tests And Commands

```bash
cargo test -p pyreth
cargo run -p pyreth --example test_simulation
maturin develop --manifest-path pyreth/Cargo.toml
python pyreth/examples/provider/process_latest_block.py
```

## Current Hazards

- Keep wrappers thin. If logic is useful outside Python, it belongs in the Rust
  owner crate.
- Reuse shared provider/simulator handles to avoid unnecessary MDBX handle churn.
- Preserve Rust-to-Python big integer/address conventions: checksum addresses
  and decimal strings where required by existing Python consumers.
