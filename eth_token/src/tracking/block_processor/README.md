# block_processor

Applies a processed block to the token registry in transaction order.

The block processor owns the block-level orchestration only. Transaction-level
token and pool mutations stay in `token_update_router`, while the actual EVM
buy/sell checks stay in `tx_simulator` through the pool simulator APIs.

## Current Historical Simulation Shape

For range indexing and final mined-block token status, the historical path
updates token/pool state inside the transaction loop but defers EVM buy/sell
checks until the full block has been applied:

```text
for tx in processed block order:
  update token state from decoded tx facts
  update pool state from decoded tx facts
  collect pending simulation requests

after all txs are applied:
  coalesce by (token_address, pool_kind, pool_id)
  open one post-block state session for this block
  simulate each affected pool once
  write final buy/sell/tax status back to the registry
```

The semantic target is final post-block tradability, not transient intra-block
tradability. That is the right default for historical range views because the
block is already mined and the API reports the token/pool state after that block
has been applied.

If a feature needs exact launch timing or intra-block transitions, keep that as
an explicit mode. Do not mix it into the default range builder path.

## Legacy Intra-Block Shape

The older historical path ran buy/sell simulation immediately after a triggering
transaction. It used `BlockTxStateSession`, so it did not reopen the block
pre-state for every pool simulation, but it still paid for intra-block prefix
replay up to the simulated transaction index. It could also simulate the same
`(token, pool_kind, pool_id)` multiple times in one mined block when the pool was
not yet marked `can_buy_and_sell` and swaps/mints/burns kept retriggering checks.

## Historical Post-State Numbering

For historical blocks, use `TxSimulator::block_state_session(block_number)` to
branch from the post-state of `block_number`.

Do not pass `block_number + 1` to `block_state_session` for post-state of
`block_number`. In this simulator API, `create_forked_state(n)` loads
`history_by_block_number(n)`, and `BlockTxStateSession` gets the pre-state for
block `n` by explicitly loading `n - 1` before replaying block `n`.

Equivalently, the post-state of block `n` is the pre-state of block `n + 1`, but
that is not how this API should be addressed. Use `block_state_session(n)` for
final state after block `n`.

## Live Blocks

Live processing has the same semantic target: post-block state for the block
being applied. The source can differ:

- use a tracked live state snapshot for block `n` when local historical context is not ready;
- use local historical context for block `n` once available;
- make any fallback to intra-block replay explicit and measurable.

The live path must not silently downgrade to repeated per-transaction simulation
unless the caller selected an intra-block mode.
