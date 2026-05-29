# HTTP Route Modules

This folder keeps handlers grouped by domain. `v1.rs` and `mod.rs` are route
aggregators only: they should expose paths, compose filters, and delegate to the
domain modules.

## Naming Contract

Route names must describe the owner of the data:

- `live-token-tracker`: confirmed token/pool registry and read-model surfaces.
- `live-tx-simulator`: chain-server-owned exact live `LiveTxSimulator` state
  and simulation requests.
- `live-trading`: committed block-applied trading frames and supervision.
  Alpha consumes `/live-trading/block-frames/next` for block-pinned
  confirmed-chain strategy input.
- `mempool/pending-transaction-signals`: speculative public-mempool signal rows.
  These rows expose detector-time chain-head fields
  (`detected_at_head_block_number` and optional hash); consumers must use those fields for
  pending-signal observed-block semantics instead of substituting the current
  live-trading block frame.

Do not add new generic `live/state`, `live/updates`, `live/simulations`, or
`mempool/signals` routes. Those names hide whether the caller is reading token
read models, simulator state, committed trading events, or speculative mempool
evidence.
