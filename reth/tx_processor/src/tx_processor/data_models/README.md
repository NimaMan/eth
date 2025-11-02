# Transaction Data Models (Rust)

This module tree defines the canonical Rust representation of processed
Ethereum transactions. It mirrors the Python schema (see
`py/eth_data/eth_data/tx_processor/data_models/README.md`) so payloads can flow losslessly between runtimes.

## Layout

```
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
`tx_processor::ProcessedTransaction`. All helpers (`ProcessedBlockTransaction`,`ProcessedBlock`, etc.) ultimately compose these data types.

## Core Principles

- **Machine-scale integers**: All value fields use `U256`, `I256`, `u128`, or `i128` to keep parity with on-chain magnitudes. When JSON transports enforce 64-bit limits (e.g. RabbitMQ payloads), publishers stringify large integers and deserialisers rely on the helpers in `serde_helpers.rs` to accept either numeric or string forms.
- **Checksum addresses**: All addresses are stored as `alloy_primitives::Address` and serialised/deserialised with checksum casing via the utilities in `reth_chain_query::utils::checksum`.
- **Deterministic schema**: Field names and container shapes match the Python
  models 1:1. Any change here must be reflected in the Python README and data
  classes to keep cross-runtime compatibility.
- **No implicit coercion**: Deserialisers reject floats and malformed hex. The
  bridge (`rust/pyreth/src/tx_processor/processed_tx_bridge.rs`) enforces the
  same rules when accepting Python dictionaries.

## Key Structs

- `ProcessedTransaction` (`tx_models.rs`)
  - Core transaction metadata (`hash`, `block_number`, `from_address`, …)
  - Nested `TransactionFees`
  - Collections of transfers (`eth_transfers`, `erc20_transfers`, …)
  - Event vectors for Uniswap V2/V3/V4, approvals, ownership changes
  - `address_balance_changes` / `latest_states` maps for downstream analytics
- `TransactionFees` (`fees.rs`)
  - `gas_price`, `gas_used`, `tx_fee`, `protocol_type`,
    `max_fee_per_gas`, `max_priority_fee`
- `ERC20TransferEvent`, `UniswapV3SwapEvent`, etc. (`receipt_models.rs`)
  - Each struct is a direct representation of decoded log data
  - Large numeric fields use the shared serde helpers for robust deserialisation
- `InternalTransaction` and related call-trace entities (`trace_models.rs`)
  - Captures miner bundles, internal value transfers, and call depth information
- `AddressBalanceChange` (`balance_changes.rs`)
  - Summarises net movement per address in ETH and token units

## Adding New Models

1. Define the struct with machine-scale types inside the appropriate module.
2. Derive `Serialize`/`Deserialize` and import checksum helpers if addresses
   are present.
3. If any numeric field may exceed 64 bits, annotate it with the functions from
   `serde_helpers.rs`.
4. Export the new struct via `data_models::mod.rs` and document it here (and in
   the Python README) so both runtimes stay aligned.

Keeping this directory in sync with the Python data models guarantees that the
live pipeline (simulators, block processors, alert systems) can exchange
processed transactions without precision loss or ad-hoc conversions.

