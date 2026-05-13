# Signal Detector Module

## Purpose
Converts pool‑scoped simulation results into actionable, per‑pool trading signals and publishes them via logs/ZMQ and the live Postgres signal store.

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

2) Honeypot (topic: `honeypot_signal`)
- Trigger: pool simulation can buy and approve, but cannot sell.
- Payload (HoneypotSignal):
  - tx_hash, token_address, pool_address, pool_type, creator_address
  - can_buy, can_sell, buy_tax, sell_tax, failure_reason, confidence, timestamp
- This is intentionally separate from tax. A sell-blocked pool is a trading
  status failure, not a tax bucket transition.

3) TaxSignal (topic: `tax_signal`)
- Trigger: tax moves into a risky bucket, exceeds configured thresholds, changes bucket, or matches a suspicious pattern.
- Payload (TaxSignalRecord):
  - tx_hash, token_address, pool_address, pool_type, creator_address
  - signal_type: "TaxBucketRisk" | "TaxChange" | "SuspiciousPattern"
  - signal_details, confidence
  - buy_tax, sell_tax (optional)
  - buy_tax_bucket_from/to, sell_tax_bucket_from/to, combined_tax_bucket_from/to
  - buy_tax_exceeds_threshold, sell_tax_exceeds_threshold
  - timestamp

4) LiquidityRemoval (topic: `liquidity_removal`)
- Trigger: liquidity removal on a pool (minor, significant, or major) detected via dedicated liquidity removal simulator or state changes.
- Payload:
  - tx_hash, pool_address, pool_type, token_address (optional)
  - remover_address, function_name, estimated_eth_removed (optional)
  - remaining_eth, removal_percentage, timestamp
- Risk label: `DRAINING` for >50% removed, `SIGNIFICANT` for 20-50%,
  `LOW` for measured smaller removals, and `UNKNOWN` when protocol intent maps
  to a tracked pool before reserve impact can be measured.

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
  - Uses can_buy/can_approve/can_sell and taxes from the SimulationResult (calculated upstream)
  - Ensures pool isn’t already trading (via TokenTrackingCache)
  - Emits TradingEnabled signals

- TaxDetector (`tax_signal_detector.rs`)
  - Handles tax bucket risks, tax changes, and suspicious tax patterns
  - Produces TaxSignalRecord for publishing as `tax_signal`

- Honeypot detection (`signal_manager.rs`)
  - Emits `honeypot_signal` only for buy-then-stuck results:
    `can_buy=true`, `can_approve=true`, `can_sell=false`
  - Dedupes per `(token_address, pool_address)` during the process lifetime

- LiquidityDetector (`liquidity_detector.rs`)
  - Detects drains/removals via state changes or LiquidityRemovalResult
  - Emits LiquidityRemoval signals

- LpApprovalDetector (`lp_approval_detector.rs`)
  - Checks non‑simulated transactions classified as tracked-pool LP approvals
  - Emits LpApproval signals directly through SignalManager
  - Uses calldata decoding only; LP approval transactions are not simulated

## Publishing

- ZMQ
  - Endpoint: `tcp://127.0.0.1:5556`
  - Topics: `trading_enabled`, `honeypot_signal`, `tax_signal`, `liquidity_removal`, `lp_approval`
  - Format: JSON serialized signal structs

- Semantic signal logs (files under the run’s `signals/` directory)
  - `trading_enabled.log`
  - `honeypot_signals.log`
  - `tax_signals.log` (actual tax risk signals only)
  - `liquidity_removals.log`
  - `lp_approval_signals.log`
  - `signal_manager.log` (emitted signals and publication summaries)

- Simulation diagnostics
  - Successful simulations are counted in interval metrics and are not written
    one-by-one.
  - Run-level `simulation_errors.log` is the operational artifact for failed
    simulations and buy/sell branch errors such as failed tax calculation.

- Database
  - Live runs require signal persistence through the unified database writer.
  - Use `--allow-database-disabled` only for diagnostic ZMQ/log-only runs.

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
  - Tax buckets follow `eth_token::pools::TaxBucket`: unknown, no_tax, low_tax, moderate_tax, high_tax, extreme_tax

- LiquidityDetector
  - `scam_drain_threshold` (default: 60%)
  - `min_eth_threshold` (default: 0.3 ETH)
  - `major_removal_threshold` (50%), `significant_removal_threshold` (20%)

## Notes & Edge Cases

- Per‑pool semantics: signals are specific to a single pool — multiple pools per token produce multiple independent signals.
- Historical vs latest state: when at‑block simulation is not possible (pruned state), detectors still use best available results; TradingEnabled requires valid tax values and can_buy/can_approve/can_sell.
- Publishing is best‑effort and non‑blocking, but DB writes are required for
  live product runs because token-server and ASENA read persisted signals.
- LP approval diagnostics are included in the periodic mempool health output:
  router approvals seen, tracked pool approvals, pool cache misses, LP approvals
  published, and DB write errors.
- V2/Sushi LP approvals are actionable early sell signals. V3
  `decreaseLiquidity` and V3 pool burn events are treated as direct
  liquidity-removal risk when they map to a tracked token/pool. V4 negative
  `ModifyLiquidity` events are emitted as unknown-severity removal risk when
  they map to the tracked `pool_manager#pool_id` key. V4 simulation/PnL is still
  not validated.
- Unknown reserve impact is not low risk. When the simulator can map a removal
  intent to a tracked pool but cannot measure the drain before mining, the DB
  writer stores the risk level as `UNKNOWN`.
