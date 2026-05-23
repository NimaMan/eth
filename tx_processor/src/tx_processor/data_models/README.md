# Transaction Data Models

This module tree defines the canonical Rust representation of processed
Ethereum transactions.

## Layout

```text
src/tx_processor/data_models/
├── balance_changes.rs   // Address-level deltas (ETH + tokens)
├── fees.rs              // Gas accounting
├── receipt_models.rs    // Log-based event structs (ERC20/721/1155, Uniswap, etc.)
├── serde_helpers.rs     // Shared serde visitors for large integers
├── trace_models.rs      // Call-trace derived entities
├── tx_models.rs         // ProcessedTransaction envelope + helpers
└── mod.rs               // Re-exports for public use
```

`ProcessedTransaction` is the top-level struct exported via
`tx_processor::ProcessedTransaction`. Block-level payloads compose these data
types through `ProcessedBlockTransactions` and `ProcessedBlock`.

## Core Principles

- **Machine-scale integers**: Value fields use `U256`, `I256`, `u128`, or
  `i128` so on-chain magnitudes do not lose precision. JSON transports may
  stringify large integers; deserializers accept numeric or string forms through
  `serde_helpers.rs`.
- **Checksum addresses**: Address fields use `alloy_primitives::Address` and
  serialize with checksum casing through `reth_chain_query::utils::checksum`.
- **Deterministic schema**: Field names and container shapes are part of the
  processed transaction contract. Update serde compatibility tests when changing
  them.
- **No implicit coercion**: Deserializers reject floats and malformed hex.

## Key Structs

- `ProcessedTransaction` (`tx_models.rs`): transaction metadata, fees, decoded
  events, internal calls, balance changes, raw trace artifacts, and auxiliary
  protocol arrays.
- `TransactionFees` (`fees.rs`): gas accounting and fee protocol fields.
- Receipt event structs (`receipt_models.rs`): decoded log data for ERC tokens,
  Uniswap, approvals, ownership changes, and related events.
- `InternalTransaction` and related call-trace structs (`trace_models.rs`):
  internal value transfers and call depth information.
- `AddressBalanceChange` (`balance_changes.rs`): net ETH/token movement per
  address.

## Adding New Models

1. Define the struct in the appropriate module.
2. Derive `Serialize` and `Deserialize`; import checksum helpers for addresses.
3. Annotate large numeric fields with helpers from `serde_helpers.rs`.
4. Export the new struct via `data_models::mod.rs`.
5. Extend serde compatibility coverage when changing persisted or externally
   consumed payloads.
