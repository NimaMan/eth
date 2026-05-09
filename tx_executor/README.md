# tx_executor

Agent operating map for gas-first Ethereum transaction submission.

## Purpose

- Receive prepared direct transactions from planners/strategies.
- Validate request shape, reserve nonce, apply gas/priority-fee policy, sign,
  broadcast, and record the result.
- Provide the execution boundary that alpha can eventually call through an
  adapter.

## Owns

- `DirectRawTransactionRequest`, `SimulationReference`, bribe request metadata,
  and submit results.
- Nonce reservation, signer integration, validation, gas policy, broadcast mode,
  and optional event recording.
- Mempool position estimation from local Reth `txpool_content`.

## Does Not Own

- Route discovery, quoting, slippage math, pool discovery, or strategy policy.
- Calldata construction except validating a prepared direct transaction.
- Token/mempool analysis or risk decisions.

## Data Flow

```text
planner/strategy adapter
  -> DirectRawTransactionRequest + simulation reference
  -> EthTxExecutor::submit_direct_raw
  -> validate -> reserve nonce -> sign -> broadcast/dry-run
  -> SubmitDirectRawResult / execution record
```

## Where To Look First

| Need | Start here |
| --- | --- |
| Public API and exports | `src/lib.rs` |
| Submit flow | `src/executor.rs`, `src/service.rs` |
| Request/response types | `src/request.rs`, `src/types.rs` |
| Validation | `src/validation.rs` |
| Gas and mempool position | `src/gas.rs`, `src/position.rs` |
| Nonce/signing/broadcast | `src/nonce.rs`, `src/signer.rs`, `src/broadcast.rs` |
| Standalone example | `examples/submit_direct_raw.rs` |

## Tests And Commands

```bash
cargo run --manifest-path tx_executor/Cargo.toml --example submit_direct_raw -- --dry-run
cargo test --manifest-path tx_executor/Cargo.toml
```

Live broadcast must be explicit:

```bash
cargo run --manifest-path tx_executor/Cargo.toml --example submit_direct_raw -- --broadcast
```

## Current Hazards

- This crate starts from prepared calldata. If route/quote/slippage decisions
  are missing, fix the planner or strategy adapter, not the executor.
- Direct EOA priority fee is the normal validator/builder payment. Explicit
  `block.coinbase` payments require contract calldata and are not direct raw
  mode.
- A submitted payload should include simulator block/hash and expected/min
  output metadata so execution can be traced back to the off-chain decision.
