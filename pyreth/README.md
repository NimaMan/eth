# pyreth

Agent operating map for Python bindings over the Rust ETH stack.

**PyReth is also the agent gateway.** The `tools/` directory contains agent-facing
question-answering modules that compose pyreth's low-level bindings into structured
answers.

## Purpose

- Expose selected Rust capabilities to Python through PyO3/maturin.
- Provide thin wrappers around chain query, transaction processing, simulation,
  DEX helpers, and shared provider instances.
- **Serve as the agent gateway:** `tools/` modules accept JSON questions and
  return structured JSON answers using the pipeline.

## Owns

- PyO3 module registration in `src/python.rs`.
- Shared instance/provider management in `src/pyreth_instance.rs`.
- Python wrappers for `reth_chain_query`, `tx_processor`, `tx_simulator`, and
  DEX/simulator helpers.
- **Agent gateway tools in `tools/`** — semantic question-answering layer.

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

Agent (Rust/beden)
  -> subprocess: python3 pyreth/tools/<tool>.py
  -> tool imports pyreth and orchestrates calls
  -> tool prints structured JSON
  -> agent parses JSON and synthesizes answer
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
| **Agent gateway tools** | **`tools/`** |

## Agent Gateway Tools

Each module under `tools/` is a standalone script that:

1. Accepts JSON on `stdin`
2. Uses pyreth bindings to query/simulate/classify
3. Prints structured JSON on `stdout`

| Tool | Purpose |
|------|---------|
| `tools/assess_token_pool.py` | Full token/pool assessment (metadata + liquidity + viability + classification) |
| `tools/get_pool_liquidity.py` | Raw pool reserves and price |
| `tools/get_eligible_launches.py` | Count eligible token launches in a time range |

Run a tool manually:

```bash
cd /home/nima/code/crypto/blockchains/eth
PYRETH_DATADIR=/home/nima/storage/samsung8tb/ethereum/reth \
  python3 pyreth/tools/assess_token_pool.py <<'JSON'
{"token": "0x0f9f5E9b76AA03e9Ab1dbd76223FC70A322b55Ad",
 "pool": "0x2997a394e02c46A2D00Eb9a004d0145d79c242cc"}
JSON
```

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
- **Gateway tools should not duplicate business logic from Rust crates.**
  They should compose and orchestrate, not reimplement.
