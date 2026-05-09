# TX Processor (v0.3.0)

High‑performance Rust processor for Ethereum transactions with direct Reth DB access and integrated simulation. Designed to produce rich ProcessedTransaction objects (events, traces, balance deltas) and power higher‑level analyses like AMM trade viability and tax calculation.

## Architecture

- Reth DB access: Reads chain state from local MDBX via `tx_simulator` and `reth_chain_query` (no RPC required).
- Simulation pipeline: Builds `UnsignedTransaction` calls, simulates with full traces, and converts to `ProcessedTransaction`.
- Processing core (`tx_processor::tx_processor`):
  - Log decoding (ERC20/721/1155 + DEX events)
  - Internal call/trace extraction
  - Address balance change calculation (currency_net/token_net)
  - Transaction classification
- ProcessedTxProvider: One‑shot helpers to simulate + process from an unsigned tx or a tx hash using your shared simulator/DB connection.
- AMM transaction builders (via `reth_chain_query::tx_builders`): Centralized Uniswap V2/V3 buy/approve/sell builders selected by `AmmSwapRoute`.
- Tax calculator (`tx_processor::tx_processor::tax_calculator`): Computes buy/sell taxes from balance deltas of a `ProcessedTransaction`.

## Data Flow

1. Build or obtain an `UnsignedTransaction` (or a tx hash).
2. Simulate with full trace at block N (using `tx_simulator`).
3. Convert to `ProcessedTransaction` (`TxProcessor::process_transaction_from_simulation_result`).
4. Analyze: decode events, internal calls, and address balance changes.
5. Optional: compute buy/sell taxes with the tax calculator.

## Diagrams

Sequence: unsigned tx → processed tx

```
Caller
  │
  ├─ build UnsignedTransaction (or have tx hash)
  │
  ├─ process_unsigned_tx / process_tx_by_hash
  │        │
  │        ▼
  │   ProcessedTxProvider
  │        │  (reuses same ProviderFactory)
  │        │
  │        ├─ simulate_unsigned_transaction_with_full_trace_at_block
  │        │        │
  │        │        ▼
  │        │   TxSimulator (Reth DB)
  │        │      └─ returns FullSimulationResult (logs + trace + gas)
  │        │
  │        └─ TxProcessor::process_transaction_from_simulation_result
  │                 ├─ LogDecoder (events)
  │                 ├─ TransactionTraceProcessor (internal calls)
  │                 └─ AddressBalanceChangeCalculator (currency_net / token_net)
  │
  │        └─ returns ProcessedTransaction
  │
  └─ (optional) tax_calculator::calculate_buy/sell_tax(processed_tx, ...)
```

Module dependency sketch

```
+----------------------+          +--------------------+
|  reth_chain_query    |          |    tx_simulator    |
|  - tx_builders (AMM) |  uses -> |  (Reth DB access)  |
|  - provider utils    |<---------|  simulate + traces |
+----------^-----------+          +---------^----------+
           |                                 |
           | uses                            | returns FullSimulationResult
           |                                 |
+----------+-----------+          +----------+----------+
|      tx_processor    | uses --->| ProcessedTxProvider |
|  - TxProcessor       |<---------|  (or crate helpers) |
|  - tax_calculator    |   returns|  ProcessedTransaction|
|  - utils             |          +----------------------+
+----------------------+                      
```

## Core APIs

Convenience helpers (crate root):

```rust
use tx_processor::{TxSimulator, UnsignedTransaction};
use tx_processor::{process_unsigned_tx, process_tx_by_hash};
use alloy_primitives::{B256, U256};

// From an unsigned tx at a specific block (or latest)
let processed = process_unsigned_tx(&simulator, unsigned_tx, Some(block)).await?;

// From a transaction hash (loads, simulates pre-state, processes)
let processed = process_tx_by_hash(&simulator, tx_hash).await?;
```

One‑shot provider (more control):

```rust
use tx_processor::processed_tx_provider::ProcessedTxProvider;

let provider = ProcessedTxProvider::with_provider_factory(simulator.provider_factory().clone())?;
let processed = provider.process_transaction_from_unsigned_tx(unsigned_tx, Some(block)).await?;
```

Tax calculation:

```rust
use tx_processor::tx_processor::tax_calculator::{
    calculate_buy_tax_from_processed_transaction,
    calculate_sell_tax_from_processed_transaction
};

let buy_tax = calculate_buy_tax_from_processed_transaction(&processed_buy, pool, buyer, token);
let sell_tax = calculate_sell_tax_from_processed_transaction(&processed_sell, pool, buyer);
```

## AMM Trade Viability (Buy → Approve → Sell)

- Builders live in `reth_chain_query::tx_builders` and are selected by `AmmSwapRoute`:
  - `build_buy_swap(route, buyer, token_out, eth_in, slippage_bps, deadline)`
  - `build_approve_for_route(route, owner, token, amount)`
  - `build_sell_swap(route, seller, token_in, token_in_amount, slippage_bps, deadline)`
- Simulator `check_can_buy_sell_pool(...)` orchestrates the sequence and produces `PoolBuySellSimulationResult`.
- Supported routes today: Uniswap V2 (incl. Sushi) and Uniswap V3 (fee tier).

