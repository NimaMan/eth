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

## Current Contract

The ingest layer now exposes `extract_token_network_updates`, which combines the
supported adapters for one processed transaction:

- `transaction.rs`: transaction context, fee/bribe cost updates, and fee-source
  coactivity edges;
- `transfers.rs`: tracked-token transfers, native ETH transfers, internal ETH
  transfers, and known-denomination ERC20 transfers;
- `authority.rs`: creator, owner, pending owner, admin role, proxy admin, and
  trading policy control relations for the tracked token;
- `pools.rs`: V2/V3/V4 pool creation, swap/trade, and liquidity event network
  edges.

The output is a `NetworkIngestBatch` containing:

- address movements to apply to `activity`;
- address fee/bribe costs to apply to `activity`;
- node labels to merge into the raw graph;
- collapsed edges with bounded evidence examples.

The layer still does not mutate graph state. That belongs in `graph/raw.rs`.
