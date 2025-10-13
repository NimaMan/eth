# Tx Processing Core – Architecture & Guide

This document explains how a raw transaction (on‑chain or simulated) becomes a rich `ProcessedTransaction` with decoded events, internal calls, and address balance changes. It is the canonical reference for building features on top of the tx processing pipeline.

## Goals

- Provide a fast, reproducible pipeline for decoding and analyzing Ethereum transactions using local Reth DB.
- Produce a single, comprehensive `ProcessedTransaction` struct suitable for downstream analytics (AMM viability, tax calc, tracing, metrics).
- Keep responsibilities clear and composable: simulation, decoding, trace processing, balance calculation.

## High‑Level Flow

1. Source a transaction
   - A) Existing on‑chain tx: load by hash from Reth DB.
   - B) Unsigned call: construct an `UnsignedTransaction` (e.g., AMM swap) for simulation.
2. Simulate (if B) with full trace via `tx_simulator` at a specific block/state.
3. Build `ProcessedTransaction` via the `TxProcessor`:
   - Decode logs → rich, typed events (ERC20/721/1155, Uniswap V2/V3/V4, approvals, etc.).
   - Extract internal calls from call trace.
   - Attach opcode-level `struct_logs` when the simulator ran in full-trace mode.
   - Compute address balance changes (currency_net, token_net).
   - Set fees, metadata, and basic classification.
4. Use `ProcessedTransaction` for higher‑level logic (e.g., AMM tax calc, analytics).

## Sequence Diagram

```
Caller
  │
  ├─ Build UnsignedTransaction (or specify tx hash)
  │
  ├─ ProcessedTxProvider::process_transaction_from_unsigned_tx / _by_hash
  │        │
  │        ├─ TxSimulator::simulate_unsigned_transaction_with_full_trace_at_block
  │        │      └─ Reads Reth DB state, executes EVM, returns FullSimulationResult
  │        │
  │        └─ TxProcessor::process_transaction_from_simulation_result
  │               ├─ LogDecoder → decoded events
  │               ├─ TransactionTraceProcessor → internal calls
  │               └─ AddressBalanceChangeCalculator → currency_net/token_net
  │
  └─ → ProcessedTransaction
            └─ (optional) tax_calculator::calculate_buy/sell_tax(...)
```

## Module Dependency Sketch

```
+----------------------+        +--------------------+
|  reth_chain_query    |        |    tx_simulator    |
|  - tx_builders (AMM) |  --->  |  (Reth DB access)  |
|  - provider helpers  |        |  simulate + traces |
+----------^-----------+        +---------^----------+
           |                               |
           | uses                          | returns FullSimulationResult
           |                               |
+----------+-----------+        +----------+----------+
|      tx_processor    |  --->  | ProcessedTxProvider |
|  - TxProcessor       |  <---  | (or crate helpers)  |
|  - tax_calculator    |  uses  | build, simulate,    |
|  - utils             |        | process             |
+----------------------+        +---------------------+
```

## Key Components

### 1) Simulation (tx_simulator)

Crate: `tx_simulator`

- Reads state from local Reth DB (no RPC) and executes `UnsignedTransaction` at a block.
- `simulate_unsigned_transaction_with_full_trace_at_block` returns logs + full call trace + gas.
- Provides helpers for view calls (e.g., Quoter/getAmountsOut).

Sharing state: always reuse the same `ProviderFactory`/simulator across modules (e.g., via `with_provider_factory`) to avoid DB contention.

### 2) ProcessedTxProvider (one‑shot orchestration)

Module: `tx_processor::processed_tx_provider`

Two main helpers:

```rust
// Simulate UnsignedTransaction at a block (or latest) and return ProcessedTransaction.
async fn process_transaction_from_unsigned_tx(
    &self,
    unsigned_tx: UnsignedTransaction,
    block_number: Option<u64>,
) -> eyre::Result<ProcessedTransaction>;

// Load by tx hash, simulate at pre‑state (block_number - 1), and return ProcessedTransaction.
async fn process_transaction_by_hash(
    &self,
    tx_hash: B256,
) -> eyre::Result<ProcessedTransaction>;
```

Crate‑level convenience:

```rust
use tx_processor::{process_unsigned_tx, process_tx_by_hash};
let processed = process_unsigned_tx(&simulator, unsigned_tx, Some(block)).await?;
let processed = process_tx_by_hash(&simulator, tx_hash).await?;
```

### 3) TxProcessor (decoding, traces, balances)

Module: `tx_processor::tx_processor`

Primary entry used by `ProcessedTxProvider`:

```rust
// Convert simulator result to a fully‑decoded ProcessedTransaction
async fn process_transaction_from_simulation_result(
    &self,
    unsigned_tx: &tx_simulator::UnsignedTransaction,
    simulation_result: &tx_simulator::FullSimulationResult,
    block_number: u64,
    tx_index: u64,
) -> eyre::Result<ProcessedTransaction>;
```

It performs:

- Log decoding via `LogDecoder` → populates event vectors (ERC20/721/1155 transfers, approvals, Uniswap events, etc.).
- Trace processing via `TransactionTraceProcessor` → builds internal call list.
- Address balance change calculation via `AddressBalanceChangeCalculator` → currency_net/token_net per address.
- Fee and metadata population.
- Lightweight classification (e.g., ETH_TRANSFER, CONTRACT_INTERACTION).

### 4) Address Balance Changes

Module: `tx_processor::address_balance_change_calculator`

Produces structured deltas per address:

- `currency_net`: symbol → signed delta (I256) for known tokens from the registry; includes ETH as "ETH".
- `token_net`: checksum(token_address) → U256 delta (for arbitrary ERC20s).

Notes:

- U256 values can encode negatives using two's complement in the deltas; helpers in `tax_calculator` handle sign/absolute.
- ETH deltas reflect net ETH movement (consider gas and internal transfers). For sell tax logic, avoid using raw ETH in some contexts.

### 5) Tax Calculator

Module: `tx_processor::tx_processor::tax_calculator`

Functions:

- `calculate_buy_tax_from_processed_transaction(processed, pool, buyer, token)`
  - Pool must show a negative token delta; buyer a positive.
  - Tax = pool_sent_tokens − buyer_received_tokens (basis points returned).

- `calculate_sell_tax_from_processed_transaction(processed, pool, seller)`
  - Finds the sold token by matching seller negative delta and pool positive delta.
  - Tax = seller_sent_tokens − pool_received_tokens (basis points).

This is downstream of the core processor; it consumes `ProcessedTransaction` without requiring further decoding/tracing.

## ProcessedTransaction Structure (selected fields)

Contains:

- Metadata: `hash`, `block_number`, `timestamp`, `tx_index`, `from`, `to`, `value`, `status`, `fees`.
- Decoded events: `erc20_transfers`, `approvals`, `uniswap_v2_swaps`, `uniswap_v3_swaps`, etc.
- Internal calls: extracted call graph entries.
- Address balance changes: `address_balance_changes` map with `currency_net`/`token_net`.
- Raw tracing artefacts: optional `struct_logs` (opcode-level trace) when the simulator ran in full-trace mode.
- Latest states (optional), raw input, and auxiliary arrays for additional protocols.

## AMM Integration & Builders

AMM transactions (buy/approve/sell) are constructed by `reth_chain_query::tx_builders` and selected via `AmmSwapRoute`:

```rust
use reth_chain_query::tx_builders::{self, amm_swap_route::AmmSwapRoute};

let route = AmmSwapRoute::UniswapV2 { pool };
let buy = tx_builders::build_buy_swap(&route, buyer, token_out, eth_in, slippage_bps, deadline);
let approve = tx_builders::build_approve_for_route(&route, buyer, token_out, U256::MAX);
let sell = tx_builders::build_sell_swap(&route, buyer, token_out, tokens_in, slippage_bps, deadline);
```

Multi‑step sequences are orchestrated via the simulators in `src/simulator` (e.g., `buy_swap_simulator.rs`, `sell_swap_simulator.rs`, `pool_buy_sell_simulator.rs`, `cross_venue_buy_approve_sell.rs`) and use the core processor to convert each step to a `ProcessedTransaction`.

## Recommended Usage Patterns

- Always share the same `ProviderFactory`/`TxSimulator` across components to avoid MDBX contention and extra file handles.
- For sequence simulations (buy → approve → sell), use the chain simulator to preserve state across steps.
- Use `ProcessedTxProvider` to keep call‑site code minimal; only drop to raw `TxProcessor` if you must.

## Performance Notes

- Direct DB reads and in‑process simulation dramatically reduce latency vs RPC.
- Start with targeted simulations for specific effects (e.g., exactInputSingle, swapExactETHForTokens) then expand.
- Use release builds for bench; examples run debug to aid iteration.

## Limitations & Future Work

- Some protocol decoders are intentionally lightweight; expand event coverage as needed.
- V2/V3 AMM builders exist; Balancer/Curve routes/builders to be added.
- Slippage minOut currently 0 in builders (acceptable for simulation). Add quoting via V2 `getAmountsOut` and V3 Quoter for stricter guards.

## Examples

See `tx_processor/examples` for end‑to‑end demos:

- Process by hash: `process_transaction_by_hash`
- Unsigned → processed: `processed_tx_from_unsigned_tx`
- AMM viability + taxes: `can_buy_sell_common_tokens_uni_v2`, `can_buy_sell_common_tokens_uni_v3`, `erc20_pool_tax_demo`
- Approval mechanics: `approval_mechanics_demo`

---

This doc reflects version 0.3.0 after centralizing AMM builders in `reth_chain_query`, removing local pool adapters, and unifying tax calculation under `tx_processor::tx_processor::tax_calculator`.
