# eth_token

Rust token-state crate for ERC-20 token tracking, pool state, token health, and block-level token updates.

This crate is the Rust replacement target for `pyeth/eth_token`. It must not own block or transaction processing. Its input boundary is the canonical Rust `tx_processor` output: processed blocks and processed transactions. Token logic starts after block/tx decoding is complete.

## Ownership

- Track ERC-20 token state from processed transaction events.
- Maintain AMM pool state for supported DEX pool types.
- Detect token trading state, volume, control-address activity, and health signals.
- Build token network/activity views from processed events.
- Provide a block-level token processor that consumes processed Rust blocks.

## Non-Goals

- No direct RPC tracing.
- No Python block processor or Python transaction processor behavior.
- No duplicate log decoding already owned by `tx_processor`.
- No live block publication; Rust live block processing remains in `tx_processor`.

## Migration Plan

1. Define Rust data models that match the current Python-facing snapshots.
2. Port pool state machines first: base pool, Uniswap V2, V3, and V4.
3. Port token transfer/control-address state.
4. Port block-level token orchestration from processed transactions.
5. Add PyO3 bindings through `pyreth` after the Rust API is stable.
6. Retire the corresponding Python modules once parity tests pass.
