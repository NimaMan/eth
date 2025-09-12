# tx_simulator – Detailed Design and API Spec

This document is an engineering-level specification you can implement from. It enumerates public types, method semantics, data/flow contracts, and operational details for deterministic, reproducible behavior.

Status: Stable, in use across multiple crates. Reth v1.6.x, REVM 27.x.

## Core Concepts

- Provider Factory: Builds read-only providers over a local Reth MDBX.
- Forked State: A layered DB (overlay) capturing writes during simulation without touching disk.
- EVM Config: ChainSpec + BlockEnv builder that constructs an `Evm<DB>` configured to a specific header.
- Inspector Fusion: Keep a single TracingInspector instance across a chain to reduce allocations and maintain consistency (we reuse it; no need to re-allocate per step).
- CallFrame: Geth-compatible call trace structure returned by the tracing inspector.

## Data Types

### UnsignedTransaction

Fields:
- from: Option<Address>
- to: Option<Address>
- gas: Option<u64>
- gas_price: Option<u128>
- max_fee_per_gas: Option<u128>
- max_priority_fee_per_gas: Option<u128>
- value: Option<U256>
- data: Option<Bytes>
- nonce: Option<u64>

Notes:
- For EIP-1559 style calls set `max_fee_per_gas` and `max_priority_fee_per_gas` and leave `gas_price` None.
- If `nonce` is None in chain simulations, it is auto-populated from forked state and incremented per success.

### SimulationResult

Fields:
- success: bool
- gas_used: u64
- revert_reason: Option<String>

Semantics:
- `success == false` implies EVM revert. `revert_reason` may be populated via `simulation_revert_decoder` if output is ABI-compatible revert bytes.

### FullSimulationResult

Fields:
- success: bool
- gas_used: u64
- revert_reason: Option<String>
- call_trace: geth::CallFrame

Semantics:
- `call_trace` is produced by `TracingInspector` with `default_geth()` config, optionally with `.with_log()` enabled to include logs.

## Modules and APIs

### simulator.rs (TxSimulator)

Constructor:
- `TxSimulator::new(db_path: &str) -> eyre::Result<TxSimulator>`
  - Opens provider factory pointing at the provided MDBX path.

Block/chain metadata:
- `get_latest_block(&self) -> eyre::Result<u64>`
- `get_base_fee_at_block(&self, block_number: u64) -> eyre::Result<u128>`
- `get_block_metadata(&self, block_number: u64) -> eyre::Result<(timestamp: u64, gas_limit: u64, number: u64, base_fee: Option<u128>)>`

State/Fork management:
- `create_forked_state(&self, block_number: u64) -> eyre::Result<ForkedState>`
  - ForkedState contains: `{ db: InMemoryDBOverlay, block_number: u64, nonces: HashMap<Address, u64> }`.
- `get_nonce_from_state(&self, forked: &mut ForkedState, addr: Address) -> eyre::Result<u64>`

Low-level execution helpers (used by higher modules):
- `simulate_on_fork_with_trace(&self, fork: &mut ForkedState, unsigned: UnsignedTransaction, block_number: u64) -> eyre::Result<FullSimulationResult>`
- `simulate_unsigned_transaction_at_block(&self, unsigned: UnsignedTransaction, block_number: u64) -> eyre::Result<SimulationResult>`
  - Behavior: constructs header, builds EVM env, executes once without a persistent inspector, returns basic result.

Notes:
- Header selection is by `header_by_number(block_number)`. Immediately after a new block import there may be a short canonicalization window where this returns None.

### unsigned_tx_simulator.rs

Single-call, unsigned entry points:
- `simulate_call(&self, unsigned: UnsignedTransaction) -> eyre::Result<SimulationResult>`
  - Uses `get_latest_block()`.
- `simulate_call_at_block(&self, unsigned: UnsignedTransaction, block_number: u64) -> eyre::Result<SimulationResult>`
- `simulate_unsigned_transaction_with_call_trace_at_block(&self, unsigned: UnsignedTransaction, block_number: u64) -> eyre::Result<FullSimulationResult>`

Encoding helpers:
- Selected read-only helpers live in `contract_method_simulator.rs` (e.g., basic ABI-less encoders).

### signed_tx_simulator.rs

Single-call, signed entry points:
- `simulate_signed_transaction(&self, tx: &TransactionSigned) -> eyre::Result<SimulationResult>` (latest block)
- `simulate_signed_transaction_at_block(&self, tx: &TransactionSigned, block_number: u64) -> eyre::Result<SimulationResult>`
- `simulate_signed_transaction_with_call_trace_at_block(&self, tx: &TransactionSigned, block_number: u64) -> eyre::Result<FullSimulationResult>`

Behavior:
- Recovers signer (via alloy consensus), builds `TxEnv`, executes, returns results as above.

### unsigned_tx_chain_simulator.rs (Stateful Unsigned Chain)

Entry point:
- `TxSimulator::start_simulation_chain(&self, at_block: Option<u64>) -> eyre::Result<UnsignedTxChainSimulation>`

Methods:
- `step(&mut self, unsigned: UnsignedTransaction) -> eyre::Result<SimulationResult>`
  - Auto-populates nonce when missing using `get_nonce_from_state`; on success increments tracked nonce.
  - Uses a fused `TracingInspector` internally for efficiency (even if not returning a trace).
- `step_with_trace(&mut self, unsigned: UnsignedTransaction) -> eyre::Result<FullSimulationResult>`
- `step_through(&mut self, vec: Vec<UnsignedTransaction>) -> eyre::Result<Vec<SimulationResult>>`
- `current_state(&self) -> ChainStateInfo { block_number, transaction_count, total_gas_used, nonces }`
- `reset(&mut self) -> eyre::Result<()>` (recreates fork at initial block, clears inspector and stats)

Semantics:
- Each step commits writes into the overlay DB, so subsequent steps see previous effects.
- Gas limit for calls may be set by the caller; otherwise a safe default is used by the builder.

### signed_tx_chain_simulator.rs (Stateful Signed Chain)

Entry point:
- `TxSimulator::start_signed_chain(&self, at_block: Option<u64>) -> eyre::Result<SignedTxChainSimulation>`

Methods:
- `step(&mut self, tx: &TransactionSigned) -> eyre::Result<SimulationResult>`
- `step_with_trace(&mut self, tx: &TransactionSigned) -> eyre::Result<FullSimulationResult>`
- View helpers on forked state:
  - `erc20_balance_of_on_fork(&mut self, token: Address, owner: Address) -> eyre::Result<U256>`
  - `eth_balance_of_on_fork(&mut self, owner: Address) -> eyre::Result<U256>`
  - `nonce_of(&mut self, address: Address) -> eyre::Result<u64>`

Semantics:
- Recovers signer to build an accurate `TxEnv` (chain ID as per header/spec).
- Uses a (reused) inspector for consistent traces; commits writes between steps.

### parallel_tx_simulator.rs

Entry point:
- `simulate_unsigned_tx_list_parallel(&self, requests: Vec<(String, UnsignedTransaction)>, options: ParallelTxSimulationOptions) -> eyre::Result<ParallelUnsignedResult>`

Options:
- `ParallelTxSimulationOptions { max_concurrent: usize, timeout_per_tx: Option<Duration>, block_number: Option<u64> }`

Behavior:
- Selects `block_number` (explicit or latest) once.
- Spawns up to `max_concurrent` tasks guarded by a `Semaphore`.
- Each task runs `simulate_unsigned_transaction_at_block(...)` with optional timeout.
- Returns per-id results plus summary timing.

Notes:
- Tracing per-call in parallel is intentionally not provided (heavy); downstream consumers can re-simulate for traces.

### block_simulation (trace entire blocks)

Utilities that:
- Iterate transactions in a block, simulate each in canonical order at that block’s header/env, and emit geth frames.
- Used for RPC equivalence verification and profiling.

### simulation_revert_decoder.rs

Utility to convert EVM revert bytes into human-readable strings when ABI semantics are recognized (e.g., Error(string)). Falls back to hex if not recognized.

## Execution Semantics

Header selection:
- All at-block methods read headers via `HeaderProvider::header_by_number(number)`.
- If None is returned (DB canon lag), callers should retry or use a prior block.

Gas policy:
- Callers can set `gas` on `UnsignedTransaction`; otherwise a safe default is chosen by the builder.
- For signed flows, the gas limit is read from the signed transaction (and enforced by the EVM).

Fee policy:
- For unsigned EIP-1559, supply `max_fee_per_gas` and `max_priority_fee_per_gas` explicitly when needed.
- For examples, a convenience strategy may derive `gas_price` from `base_fee + tip` of the latest block (not part of the library policy).

Nonce policy:
- Chain simulators track nonces in-memory per `from`.
- If `nonce` is None, we read from the fork overlay via `get_nonce_from_state` then set it.
- On success, the tracked nonce increments; on revert it is not incremented.

State commit model:
- After each EVM execution, the resulting `state` diff is committed into the fork overlay (`DatabaseCommit`).
- This makes effects visible to subsequent steps and view helpers.

Tracing:
- Uses `TracingInspector::default_geth()`; for `step_with_trace` variants, logs are enabled (`set_record_logs(true)`) and gas limit is attached on export.
- Export uses `into_geth_builder().geth_call_traces(CallConfig::default().with_log(), gas_used)`.

Errors and timeouts:
- All methods return `eyre::Result<...>`.
- Parallel evaluation can timeout per-tx and returns a timeout error for that item.
- Revert reasons are captured in `SimulationResult.revert_reason` (not an error).

## Integration Contracts

Downstream crates (e.g., `tx_processor`, `reth_chain_query`) may rely on:
- `FullSimulationResult.call_trace` shape matching Geth frames
- Deterministic header/env for a fixed block number
- Nonce behavior in chain simulators for reproducible sequences

## Examples Index (authoritative)

- basic: verify DB access, unsigned call, view call, trace extraction
- sequential: stateful ETH transfers; token workflows (FLOKI/PEPE/USDC); MEV bundle demo
- performance: RPC vs direct benchmark; inspector fusing benchmark
- advanced: timeout handling; revert reason decoder
- block: trace transactions; verify RPC equivalence
- signed chain: `buy_approve_sell_signed_chain_uniswap_v2` (signed end-to-end)

Run any example:
```
cargo run --example <name>
```

## Operational Notes

- Safe to run alongside a live Reth node; read-only MDBX access.
- For freshest state, ensure DB is fully synced and canonicalized to the desired block.
- For heavy workloads, prefer parallel unsigned evaluation and selectively re-simulate with traces.

