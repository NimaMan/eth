# ingest

Input adapters for token-network updates.

This folder converts existing chain-processing state into network events. It
should be the only layer that knows about `ProcessedTransaction`,
`TokenTransferTracker`, authority state, and pool state shapes.

## Files

- `transaction.rs`: transaction-level update extraction.
- `transfers.rs`: ERC20, ETH, WETH, stable, and known-denom transfer extraction.
- `pools.rs`: pool trade, LP movement, pool creation, and liquidity extraction.
- `authority.rs`: owner/admin/proxy/role/creator relationship extraction.

## Boundaries

- Does not fetch blocks or historical data directly.
- Does not store graph state.
- Does not decide final cluster membership.
- Should preserve raw evidence needed by later simplification.

## First Implementation Target

Create small typed update records that can be generated from a
`ProcessedTransaction` and applied to the raw graph without leaking processor
internals into the graph layer.
