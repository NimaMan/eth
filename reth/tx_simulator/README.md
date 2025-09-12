# Transaction Simulator (tx_simulator)

High-performance, local Ethereum transaction simulation powered by Reth. This crate focuses on fast, deterministic simulation for research and live systems without relying on RPC. It provides unsigned and signed transaction simulation, stateful sequential execution, parallel evaluation, and geth-compatible call traces.

## What It Does

- Directly reads Reth MDBX to build an EVM execution environment at a given block
- Executes transactions against a forked state with optional tracing
- Persists state across steps for realistic multi-tx workflows (buy → approve → sell)
- Supports signed and unsigned flows with consistent outputs and revert decoding
- Produces geth CallFrame traces consumable by downstream analyzers

## Data Flow (Conceptual)

1) Reth DB → Provider → Header at block N  
2) Header → EVM configuration (BlockEnv, ChainSpec)  
3) Forked state (DB overlay) is created once per chain session  
4) For each tx: build TxEnv → run EVM with optional TracingInspector  
5) Commit state changes to fork overlay → return SimulationResult/FullSimulationResult  

```
Reth MDBX ──► Provider ──► Header(N)
                              │
                              ▼
                        EVM Config (BlockEnv)
                              │
                              ▼
                     Forked State (overlay DB)
                              │
                         ┌────┴────┐
                         │  EVM    │ + TracingInspector (optional)
                         └────┬────┘
                              │
                 Commit state │  Return result/trace
                              ▼
                   FullSimulationResult / SimulationResult
```

## Capabilities (Overview)

- Unsigned simulation: debug_traceCall-equivalent at any block
- Signed simulation: execute real signatures (mempool/RPC artifacts)
- Stateful chains: interactive step() / step_with_trace() with nonce tracking
- Parallel evaluation: concurrent unsigned calls at a chosen block with timeouts
- Block tracing: trace every tx in a block with geth-compatible frames
- Revert decoding: human-readable error strings when available

## Modules (Responsibility Map)

- `simulator.rs`: TxSimulator core (DB/provider wiring, block metadata, fork creation)
- `unsigned_tx_simulator.rs`: Unsigned single-call execution and tracing
- `signed_tx_simulator.rs`: Signed single-tx execution and tracing
- `unsigned_tx_chain_simulator.rs`: Stateful unsigned chain (step, trace, nonces)
- `signed_tx_chain_simulator.rs`: Stateful signed chain (step, trace, nonces)
- `unsigned_tx_bundle_simulator.rs`: On-fork helpers and block-level utilities
- `parallel_tx_simulator.rs`: Parallel unsigned evaluation with concurrency/timeout controls
- `block_simulation/`: Block-wide tracing utilities
- `simulation_revert_decoder.rs`: Revert data -> message decoding
- `contract_method_simulator.rs`: Lightweight ABI-less encoders for common reads
- `types.rs`: Core DTOs returned to callers

## Where To Look Next

- High-level usage and scenarios: `examples/` (see examples/README.md for an index)
- Detailed APIs and execution semantics: `docs/DETAILED_DESIGN.md`

## Setup

- Requires a synced Reth database (default: `~/.local/share/reth/mainnet`)
- Read-only access; safe to use alongside running services
- Tested with Reth v1.6.x and REVM 27.x

## Notes on Correctness

- Header lookups are by block number; immediately after import a short window can exist where canonicalization hasn’t committed and number-based lookups return None.
- Tracing uses `TracingInspector::default_geth()` and produces geth CallFrame shapes for consistency with RPC.
- Nonce management in chain simulators reads from forked state and auto-increments after successful steps.

## License

MIT OR Apache-2.0
