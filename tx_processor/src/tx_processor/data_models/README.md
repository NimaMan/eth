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
- **Balance completeness**: For every address that appears in a transaction, the
  processed transaction must be able to express that address's net balance state
  — *before → after* — for ETH and for every token it touched. This requires
  capturing **all** value movements in the tx, not only those that emit a
  `Transfer` event. Event-derived data alone is incomplete: event-less / backdoor
  moves (e.g. a custody `transferFrom` that mutates balances behind the event
  interface) are invisible to it. See "Capturing all transfers" below.

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

## Capturing all transfers (balance completeness)

Token movement is captured on three layers, ordered by how directly they reflect
true balance state:

1. `erc20_transfers` (`receipt_models.rs`) — transfers decoded from emitted
   `Transfer` **events** (logs). Standard, but incomplete: a token can mutate a
   balance without emitting an event.
2. `internal_erc20_calls` (`trace_models.rs`) — every ERC-20 `transfer` /
   `transferFrom` / `approve` **call** decoded from the trace at any depth,
   including event-less ones. This is raw evidence: it also includes approvals
   (no balance move), failed calls, and calls that simply duplicate an event.
3. `address_balance_changes` (`balance_changes.rs`) — the net ETH/token movement
   per address (the *before → after* view).

To satisfy balance completeness, layer 3 is derived from the **complete** transfer
set, not just events. `internal_erc20_transfers` (`trace_models.rs`) is a
normalized view projected from `internal_erc20_calls` via
`InternalErc20Transfer::event_less_complement`: the value-moving subset
(`kind ∈ {Transfer, TransferFrom}`, `succeeded`, `amount > 0`) that has **no
matching `Transfer` event** — i.e. the event-less complement, deduplicated against
`erc20_transfers`. The canonical "all transfers" set is
`erc20_transfers ∪ internal_erc20_transfers`, and
`AddressBalanceChangeCalculator` consumes **both**, so an event-less custody drain
now appears in `address_balance_changes`.

> Cache: `internal_erc20_transfers` is persisted in the compact disk cache and the
> balance calculation changed, so `CACHE_SCHEMA_VERSION` was bumped (→ 3); old
> entries refresh per block on demand.

**Limit — argument vs. measured delta**: a call's `amount` is the call
*argument*, not a measured balance delta. Plain transfers net exactly, but
fee-on-transfer, rebasing, and balance-rewrite tokens do not. The
principled-complete source for *net state before → after* is balance-slot
(`balanceOf`) diffing during simulation; transfer summing is the cheaper
approximation that covers the common cases (including a standard custody-drain
`transferFrom`).

## Invariant: complete transfer capture ⇒ correct movements

This is the foundational correctness principle every downstream consumer (PnL,
position valuation, net-flow accounting) rests on:

> **If every transfer in a transaction is captured in `ProcessedTransaction`
> (emitted `Transfer` events in `erc20_transfers` *and* event-less internal
> ERC-20 calls folded into `internal_erc20_transfers`), then the net-flow
> movements and the reserve / held-balance accounting derived from it are
> correct.**

The accounting is **event-sourced from the captured transfer set**, never a
`balanceOf`-vs-ledger reconstruction. The downstream chain is pure net flow:
`erc20_transfers ∪ internal_erc20_transfers ∪ eth_transfers ∪ internal_transactions`
→ `TxLedger` (`eth_token::pnl::tx_ledger`) → per-position `token_in/out`,
`denom_in/out` → `token_balance = token_in − token_out`. No step queries on-chain
`balanceOf` or reconstructs a balance outside the captured transfers
(`custody::reconcile_custody_drain` exists but is **not** on this path). A holder
confiscation therefore needs no special oracle: the `transferFrom(vault, …)` that
moves the tokens out is itself a captured transfer, so the vault's net flow falls
to zero on its own.

Because correctness rests *entirely* on capture being complete, there is one hard
precondition and one known limit:

- **Trace precondition (guaranteed on the accounting path, pinned).** Event-less
  internal transfers are captured *only when the block was processed with a call
  trace*. In `block_processor/mod.rs`, when the trace is `None`, `internal_erc20_calls`
  and `internal_erc20_transfers` are left empty (a debug line is logged when token
  activity is present, so the gap is not silent). The accounting paths satisfy the
  precondition by construction: `BlockBatchOptions::default().include_traces == true`
  drives both the chain-server range cache-fill (`load_processed_block_range_with_options`)
  and the live block processor, and a regression test (`accounting_default_traces_every_block`)
  pins that default so it cannot silently flip. The processed-block disk cache keys on the
  trace config (`processed_block_trace_config_hash`), so a traceless block is never served
  as traced. A consumer that needs a hard per-`ProcessedTransaction` guarantee (rather than
  relying on the accounting default) should assert trace presence at ingestion. An absent
  trace on an accounting path is a correctness hole, not a degraded mode.
- **Argument-vs-delta limit.** A captured transfer's `amount` is the call argument,
  not a measured balance delta, so fee-on-transfer / rebasing tokens are
  approximate (see "Limit — argument vs. measured delta" above). This bounds
  *magnitude* exactness, not the *presence* of a transfer — drains/confiscations,
  which are about presence, are unaffected.

## Adding New Models

1. Define the struct in the appropriate module.
2. Derive `Serialize` and `Deserialize`; import checksum helpers for addresses.
3. Annotate large numeric fields with helpers from `serde_helpers.rs`.
4. Export the new struct via `data_models::mod.rs`.
5. Extend serde compatibility coverage when changing persisted or externally
   consumed payloads.
