# mempool_processor/src/simulator

Local operating map for pending-transaction simulation used by mempool signal
detection.

## Purpose

- Delegate live pending transactions and per-pool buy/sell probes to the
  chain-server-owned `LiveTxSimulator`.
- Keep a shared local `TxSimulator` only for Reth-provider utilities and
  explicit offline/local fallback runs.
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
- `MempoolSimulator`: chain-server live-simulator client, local fallback
  wrapper, nonce retry, and pool buy/sell delegation.
- `LiquidityRemovalSimulator`: focused liquidity-removal replay.
- `SimulationQueue`: priority and bounded job management.
- Pending creator history used as `prior_txs` for viability probes.

## Does Not Own

- Function classification; use `function_detector` and `tx_router`.
- Tax/decoded transaction logic; use `tx_processor`.
- Canonical token/pool state; fetch context from `eth_chain_server`.
- Exact live block simulation state; chain-server owns and publishes
  `LiveTxSimulator` sessions.
- Signal threshold decisions; use `signal_detector`.

## Data Flow

```text
TxSimulationJob from tx_router
  -> SimulationQueue
  -> SimulationManager
  -> MempoolSimulator / LiquidityRemovalSimulator
  -> chain-server live-tx-simulator endpoints for live runs
  -> tx_processor pool viability or processed tx facts
  -> SignalManager::process_simulation_result
```

Creator transactions are expanded per pool:

```text
creator tx
  -> resolve token and pools from token context
  -> send bounded prior creator/token sequence to chain-server for exact replay
  -> run buy/approve/sell probe per pool through chain-server
  -> emit one SimulationResult per pool
```

## Where To Look First

| Need | Start here |
| --- | --- |
| Queue/orchestration | `simulation_manager/` |
| Shared tx simulator wrapper | `mempool_simulator.rs` |
| Chain-server live simulator client | `chain_server_live_tx_simulator.rs` |
| Liquidity removal path | `liquidity_removal_simulator.rs` |
| Job/result types | `types.rs` |
| Downstream detectors | `../signal_detector/README.md` |
| Router input | `../tx_router/` |

## Current Hazards

- Same-block deployment, approval, and liquidity helpers can be truncated if
  they are classified as not requiring simulation and never enter pending
  sequences.
- `MEMPOOL_LIVE_TX_SIMULATOR_SERVER_URL=none` falls back to local historical
  Reth context and should only be used for diagnostics/offline examples.
- The pending sequence buffer is bounded per `(creator, token)`; old helpers can
  be dropped under flood conditions.
- When at-block historical state is pruned, liquidity-removal simulation falls
  back to latest-block behavior with best-effort nonce handling.
- V4 buy/sell probing is intentionally still gated. The simulator needs the
  full V4 pool key (`pool_manager`, `pool_id`, currencies, fee, tick spacing,
  hooks), while the mempool token cache currently only has the display key-level
  pool identity needed for removal-risk routing.
