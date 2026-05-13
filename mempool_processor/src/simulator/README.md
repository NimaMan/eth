# mempool_processor/src/simulator

Local operating map for pending-transaction simulation used by mempool signal
detection.

## Purpose

- Simulate pending transactions and per-pool buy/sell probes using a shared
  `TxSimulator`.
- Convert routed mempool jobs into `SimulationResult` values consumed by
  `SignalManager`.
- Preserve short creator/token pending sequences when launch helpers depend on
  earlier unmined transactions.
- Route protocol-specific liquidity risk from processed transaction events:
  V2/Sushi reserve drains, V3 `decreaseLiquidity`/pool burns, and V4 negative
  `ModifyLiquidity` events when they map to a tracked pool key.

## Owns

- `SimulationManager`: queueing, per-pool orchestration, and handoff to signal
  detection.
- `MempoolSimulator`: shared simulator wrapper, nonce retry, and pool buy/sell
  delegation.
- `LiquidityRemovalSimulator`: focused liquidity-removal replay.
- `SimulationQueue`: priority and bounded job management.
- Pending creator history used as `prior_txs` for viability probes.

## Does Not Own

- Function classification; use `function_detector` and `tx_router`.
- Tax/decoded transaction logic; use `tx_processor`.
- Canonical token/pool state; fetch context from `eth_chain_server`.
- Signal threshold decisions; use `signal_detector`.

## Data Flow

```text
TxSimulationJob from tx_router
  -> SimulationQueue
  -> SimulationManager
  -> MempoolSimulator / LiquidityRemovalSimulator
  -> tx_processor pool viability or processed tx facts
  -> SignalManager::process_simulation_result
```

Creator transactions are expanded per pool:

```text
creator tx
  -> resolve token and pools from token context
  -> replay bounded prior creator/token sequence
  -> run buy/approve/sell probe per pool
  -> emit one SimulationResult per pool
```

## Where To Look First

| Need | Start here |
| --- | --- |
| Queue/orchestration | `simulation_manager/` |
| Shared tx simulator wrapper | `mempool_simulator.rs` |
| Liquidity removal path | `liquidity_removal_simulator.rs` |
| Job/result types | `types.rs` |
| Downstream detectors | `../signal_detector/README.md` |
| Router input | `../tx_router/` |

## Current Hazards

- Same-block deployment, approval, and liquidity helpers can be truncated if
  they are classified as not requiring simulation and never enter pending
  sequences.
- Historical replays may succeed while live pending replay fails because the
  canonical DB already contains pair/allowance/reserve writes.
- The pending sequence buffer is bounded per `(creator, token)`; old helpers can
  be dropped under flood conditions.
- When at-block historical state is pruned, liquidity-removal simulation falls
  back to latest-block behavior with best-effort nonce handling.
- V4 buy/sell probing is intentionally still gated. The simulator needs the
  full V4 pool key (`pool_manager`, `pool_id`, currencies, fee, tick spacing,
  hooks), while the mempool token cache currently only has the display key-level
  pool identity needed for removal-risk routing.
