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

## Edge Identification Rules

Ingest creates raw evidence. It does not decide final cluster membership. The
same raw edge can be used differently by holder, pool, control, and risk views.

| Edge kind | How we identify it | What it means | Default confidence | Holder graph use |
| --- | --- | --- | --- | --- |
| `TokenTransfer` | ERC20 `Transfer` for the tracked token | Direct tracked-token movement between two addresses | Observed, strong | Primary direct edge |
| `DenomTransfer` | Native ETH, internal ETH, WETH, stable, or known-denom transfer | Value movement that may indicate funding, sale proceeds, or settlement | Observed, medium | Use as funding edge only when context supports it |
| `FeeSourceTouches` | Transaction sender connected to every touched address in that transaction | Same fee payer/co-presence, matching the old Python `tx owner` edge | Observed, weak to medium | Optional layer, not primary direct-transfer clustering |
| `PoolTrade` | V2/V3/V4 swap event sender to pool | Address traded against a pool | Observed, strong for pool view | Suppress pool in holder graph; use for timing/pool activity |
| `PoolCreation` | V2 pair created, V3 pool created, V4 initialized | Token is linked to a pool | Observed, strong | Suppress pool in holder graph; keep for pool view |
| `LiquidityEvent` | V2 mint/burn, V3 mint/burn/increase/decrease, V4 modify | Address changed pool liquidity | Observed, strong for LP/liquidity view | Do not merge holders solely from this edge |
| `LpTransfer` | LP token `Transfer` once pool LP tokens are routed | LP ownership transfer | Observed, strong | Use in LP view; optional risk overlay |
| `LpApproval` | LP token approval to router/spender | LP control/exit capability | Observed, medium | Risk overlay, not holder clustering |
| `ControlRelation` | creator, owner, pending owner, admin role, proxy admin, trading policy event | Address has control or policy relation to token | Observed, strong | Risk overlay; do not merge unrelated holders by default |
| `Funding` | Derived from denom flows before or near token activity | One address funded another relevant address | Inferred, medium to high | Secondary cluster layer |
| `SharedIntermediary` | Multiple holders connected through a non-holder address | Magic-Node-style connector | Inferred, medium | Separate inferred cluster layer |
| `TemporalCoactivity` | Addresses act through same noisy hub/pool in same block/time window | Time-Node-style coordination signal | Inferred, low to medium | Separate inferred cluster layer |

## Important Assumptions

- Direct token transfers are stronger evidence than fee-source co-presence.
- Denomination transfers are not automatically funding. They become `Funding`
  only when order, timing, amount, and address role make that interpretation
  defensible.
- Pool trades should not directly connect all traders to each other. The pool is
  a hub. Holder clustering through a pool should happen only through explicit
  time-window logic.
- Control relations are risk evidence, not proof that every counterparty belongs
  to the control actor.
- Inferred edges must keep evidence and confidence separate from observed edges.
