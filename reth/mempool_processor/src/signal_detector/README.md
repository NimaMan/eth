# Signal Detector Module

## Purpose
Converts pool‑scoped simulation results into actionable, per‑pool trading signals and publishes them via logs/ZMQ (and optionally to a database when the `db` feature is enabled).

Key properties:
- Per‑Pool: Every signal is specific to a (token_address, pool_address) pair.
- Integrated: SimulationManager dispatches a SimulationResult per pool to SignalManager, which runs all detectors and publishes immediately.

## Published Signals
Signals are defined in `signal_detector/types.rs` and serialized to JSON for publishing.

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
  - remover_address, function_name, estimated_eth_removed (optional), timestamp

4) ScamDetection (topic: `scam_detection`)
- Trigger: pool drain above threshold (>60%) or remaining ETH below threshold (≤0.3 ETH).
- Payload:
  - tx_hash, pool_address, pool_type, token_address
  - scammer_address, eth_drained, eth_remaining, drain_percentage, timestamp

5) LpApproval (topic: `lp_approval`)
- Trigger: non‑simulated LP token approval transactions by creators (potential rug setup).
- Payload (LpApprovalSignal):
  - tx_hash, creator, lp_token_address, router_address, amount

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
  - Checks non‑simulated transactions classified as LP approvals
  - Emits LpApproval signals directly (or can be called by SignalManager)

## Publishing

- ZMQ
  - Endpoint: `tcp://127.0.0.1:5556`
  - Topics: `trading_enabled`, `tax_signal`, `liquidity_removal`, `scam_detection`, `lp_approval`
  - Format: JSON serialized signal structs

- Logs (files under the run’s `signals/` directory)
  - `trading_enabled.log`
  - `tax_signals.log`
  - `liquidity_removals.log`
  - `scam_detections.log`
  - `lp_approval_signals.log`
  - `signal_manager.log` (summary/activity)

- Database (optional, feature `db`)
  - When built with `db` and `SignalPublisherConfig.enable_database = true`, signals are persisted via the unified database writer used by SignalPublisher.
  - When built without `db` (e.g. `--no-default-features`), publisher disables DB writes automatically.

## Processing Flow (Per Pool)

1) SimulationManager runs the transaction + per‑pool buy/sell sim.
2) Builds a SimulationResult for the specific (token, pool) pair.
3) Calls `signal_manager.process_simulation_result(&result)`.
4) SignalManager runs detectors and immediately publishes any signals via SignalPublisher (ZMQ + logs + optional DB).

Non‑simulated LP approvals are routed directly to LpApprovalDetector.

## Configuration Knobs

- TradingStatusDetector
  - `tax_threshold` (default: 25%)
  - `min_liquidity_threshold` (default: 0.5 ETH)

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

