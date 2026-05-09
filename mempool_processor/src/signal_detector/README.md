# Signal Detector Module

## Purpose
Converts pool‑scoped simulation results into actionable, per‑pool trading signals and publishes them via logs/ZMQ (and optionally to a database, enabled by default).

Key properties:
- Per‑Pool: Every signal is specific to a (token_address, pool_address) pair.
- Integrated: SimulationManager dispatches a SimulationResult per pool to SignalManager, which runs all detectors and publishes immediately.

## Published Signals
Signals are semantic trading/risk events defined in `signal_detector/types.rs` and serialized to JSON for publishing. Function-detector matches are classification/debug output; they are only promoted to signals after routing, simulation, and detector checks.

1) TradingEnabled (topic: `trading_enabled`)
- Trigger: buy/sell both succeed for the pool AND taxes are within threshold (≤ 25% by default) AND the cache does not already mark the pool as trading.
- Payload:
  - tx_hash, token_address, pool_address, pool_type
  - creator_address, buy_tax, sell_tax, timestamp

2) TaxSignal (topic: `tax_signal`)
- Trigger: consolidated tax issues per pool (high buy/sell tax beyond thresholds, honeypot pattern “can’t sell”), or suspicious patterns.
- Payload (TaxSignalRecord):
  - tx_hash, token_address, pool_address, pool_type, creator_address
  - signal_type: "HighTaxOrHoneypot" | "TaxChange" | "SuspiciousPattern"
  - signal_details, confidence
  - buy_tax, sell_tax (optional), buy_tax_exceeds_threshold, sell_tax_exceeds_threshold, cant_sell
  - timestamp

3) LiquidityRemoval (topic: `liquidity_removal`)
- Trigger: liquidity removal on a pool (minor, significant, or major) detected via dedicated liquidity removal simulator or state changes.
- Payload:
  - tx_hash, pool_address, pool_type, token_address (optional)
  - remover_address, function_name, estimated_eth_removed (optional)
  - remaining_eth, removal_percentage, timestamp
- Risk label: `DRAINING` for >50% removed, `SIGNIFICANT` for 20-50%, otherwise `LOW`.

4) ScamDetection (topic: `scam_detection`)
- Trigger: pool drain above threshold (>60%) or remaining ETH below threshold (≤0.3 ETH).
- Payload:
  - tx_hash, pool_address, pool_type, token_address
  - scammer_address, eth_drained, eth_remaining, drain_percentage, timestamp

5) LpApproval (topic: `lp_approval`)
- Trigger: non‑simulated LP token approval transactions for tracked pools,
  regardless of whether the approver is the token creator.
- Payload (LpApprovalSignal):
  - tx_hash, creator/approver, lp_token_address, router_address, amount
  - token_address, pool_address, pool_type
  - denom_address, denom_currency, approval_percentage (when known)

## Detectors
Detection happens inside `signal_manager.rs`, which coordinates the following:

- TradingStatusDetector (`trading_status_detector.rs`)
  - Uses can_buy/can_sell and taxes from the SimulationResult (calculated upstream)
  - Ensures pool isn’t already trading (via TokenTrackingCache)
  - Emits TradingEnabled signals

- TaxDetector (`tax_signal_detector.rs`)
  - Consolidates high tax, honeypot, and suspicious tax patterns
  - Produces TaxSignalRecord for publishing as `tax_signal`

- LiquidityDetector (`liquidity_detector.rs`)
  - Detects drains/removals via state changes or LiquidityRemovalResult
  - Emits ScamDetection or LiquidityRemoval signals

- LpApprovalDetector (`lp_approval_detector.rs`)
  - Checks non‑simulated transactions classified as tracked-pool LP approvals
  - Emits LpApproval signals directly through SignalManager
  - Uses calldata decoding only; LP approval transactions are not simulated

## Publishing

- ZMQ
  - Endpoint: `tcp://127.0.0.1:5556`
  - Topics: `trading_enabled`, `tax_signal`, `liquidity_removal`, `scam_detection`, `lp_approval`
  - Format: JSON serialized signal structs

- Semantic signal logs (files under the run’s `signals/` directory)
  - `trading_enabled.log`
  - `tax_signals.log`
  - `liquidity_removals.log` (also contains ScamDetection entries)
  - `lp_approval_signals.log`
  - `signal_manager.log` (summary/activity)

- Database
  - When `SignalPublisherConfig.enable_database = true` (the default), signals are persisted via the unified database writer used by SignalPublisher.
  - Disable database writes by setting `enable_database = false` when constructing the publisher.

## Processing Flow (Per Pool)

1) SimulationManager runs the transaction + per‑pool buy/sell sim.
2) Builds a SimulationResult for the specific (token, pool) pair.
3) Calls `signal_manager.process_simulation_result(&result)`.
4) SignalManager runs detectors and immediately publishes any signals via SignalPublisher (ZMQ + logs + optional DB).

Non‑simulated LP approvals are routed directly to LpApprovalDetector. The
transaction router only promotes `approve(spender, amount)` when `to` is a
tracked pool/LP token and `spender` is a known router from
`reth_chain_query::common_addresses::ROUTERS` or Permit2. SignalManager then
enriches the payload from `TokenTrackingCache` before publishing.

## Configuration Knobs

- TradingStatusDetector
  - `tax_threshold` (default: 25%)

- TaxDetector (via `TaxDetectionConfig`)
  - `max_acceptable_buy_tax`, `max_acceptable_sell_tax`
  - Honeypot determination uses can_sell=false

- LiquidityDetector
  - `scam_drain_threshold` (default: 60%)
  - `min_eth_threshold` (default: 0.3 ETH)
  - `major_removal_threshold` (50%), `significant_removal_threshold` (20%)

## Notes & Edge Cases

- Per‑pool semantics: signals are specific to a single pool — multiple pools per token produce multiple independent signals.
- Historical vs latest state: when at‑block simulation is not possible (pruned state), detectors still use best available results; TradingEnabled requires valid tax values and can_buy/can_sell.
- Publishing is best‑effort and non‑blocking; DB writes are optional and gated by build features.
- LP approval diagnostics are included in the periodic mempool health output:
  router approvals seen, tracked pool approvals, pool cache misses, LP approvals
  published, and DB write errors.
- V2/Sushi LP approvals are actionable early sell signals. V3
  `decreaseLiquidity` is treated as a direct liquidity-removal risk when it maps
  to a tracked token/pool. V4 detection is partial and should not be treated as
  validated for simulation/PnL yet.
- **Open issue – missing TradingEnabled output:** The latest prod run (`logs/signal_detector_2025-10-26_21-43-44/`) shows multiple simulation results with `can_buy=true`/`can_sell=true` and finite taxes (e.g. `simulation_results.log` entries for tx `0x3a48c4...` on pool `0xBD2067...` and tx `0x60fa30...` on pool `0x36BFC4...`), yet `trading_enabled.log` remains empty and `signal_manager.log` records `TRADING_STATUS | No change`. This means `TradingStatusDetector::detect` is returning `None` even when all success criteria appear satisfied. The detector currently short-circuits only when either leg fails, taxes are `None`/above threshold, or the pair has already been emitted in-process, so one of those guards is tripping unexpectedly. Next steps: add DEBUG instrumentation (or temporarily bump log level) around the early returns in `trading_status_detector.rs` and confirm whether the dedupe set (`emitted_trading_pairs`) or tax gating is blocking emission. Until that’s fixed, live runs won’t produce TradingEnabled signals even though the simulator proves the pool is tradeable.
