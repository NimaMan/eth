# Bogaz

`bogaz.md` is the ETH bottleneck ledger. Keep only active bottlenecks here:
what is limiting the pipeline, the evidence, the owning subsystem, and the next
action. Completed work and long investigation notes belong in focused docs or
commit history, not in this file.

## Operating Rule

A run is useful only if it tells us which limit dominates: strategy quality,
chain-sim fill realism, decision auditing, backtest parity, mempool signal
recall, live-feed reliability, or execution readiness.

The live mempool and token-tracking architecture is documented in the related
subsystem READMEs, primarily `mempool_processor/README.md` and
`eth_token_server/README.md`.

## Active Bottlenecks

| Order | Bottleneck | Owner | Evidence | Next Action |
| --- | --- | --- | --- | --- |
| 1 | **Mempool signal persistence and supervision** | `mempool_processor`, `node/systemd`, `eth_token_server` stores | The detector can publish ZMQ/log signals while `/eth/tokens/signals/` stays stale because token-server reads Postgres `live_trading.*`. Latest observed run had `Published > 0`, `ZMQ > 0`, and `DB = 0`; user systemd units also point at obsolete or missing runtimes. | Make `mempool_signal_detector` a service-managed, DB-backed runtime. In live mode, DB persistence is required, not optional. Verify a fresh pending signal appears through `/eth/tokens/api/mempool/signals` and the ASENA signals page. |
| 2 | **Strategy policy quality** | `alpha/strategies`, `alpha/engine` | `Snipe All v1` is intentionally broad; current live chain-sim behavior is measuring infrastructure more than edge. | Replace broad entry with selective pool/token scoring, risk-aware exits, position sizing, and clear skip reasons. Measure taken/skipped ratio and PnL per pool family. |
| 3 | **Chain-sim fill realism** | `alpha/engine`, `tx_processor`, `tx_simulator` | Live trading uses chain simulation, but V4 and adapter accounting still need strict balance-delta based reports. Historical theoretical assumptions must not remain in the path. | Keep `ChainSimExecutionAdapter` only. Derive bought/sold amounts, gas, and failures from simulation receipts, balance deltas, logs, and decoded reverts. Classify zero-received target tokens as route/accounting failures. |
| 4 | **Trader decision ledger completeness** | `alpha/engine`, `alpha/store`, `eth_alpha_trader` | Strategy pages need every skip, entry, exit, risk reaction, order, fill report, and PnL input to be explainable from persisted data. | Persist a complete decision ledger keyed by `run_id`, `strategy_id`, token, `TokenPoolId`, rule id, input snapshot, action, execution report, exit reason, and PnL snapshot. |
| 5 | **Backtest and live replay parity** | `alpha/backtest`, `alpha/engine`, live feed crates | Backtests must use the same strategy state machine and chain-sim fill adapter as live no-capital runs, otherwise results cannot be trusted. | Run historical replay with the same adapter/contracts as live. Compare replay lower-bound PnL to live chain-sim PnL by pool and by signal type. |
| 6 | **Live block apply latency and failure isolation** | `tx_processor`, `eth_token_server`, `eth_token`, `tx_simulator` | May 10-12 token-server logs showed 1,060 slow live block applies, 562 above 5s, a max of 25.9s, and three fatal `failed to process uncached block` tracker failures. Pool-local V2/V3 simulator lookup issues should be warnings, not tracker blockers. | Use `pipeline_bottlenecks.jsonl` plus `token_pipeline_profile.jsonl` to split live block cost by state read, token apply, simulation, and cache work. Keep pool-local metadata/simulation failures nonfatal and gate optional work when Redis live tail falls behind. |
| 7 | **Execution handoff readiness** | `alpha/engine`, `tx_executor` | Real capital should not start until no-capital chain-sim PnL, decision auditing, and signal persistence are stable. | Keep real trade paths separated from backtest/live chain-sim. Add real execution only after the no-capital system is profitable and fully auditable. |

## Current Evidence

- The active mempool architecture should keep the pending-transaction detector
  separate from token-server/live token tracking. Confirmed block state and
  speculative pending-tx simulation have different throughput, restart, and
  failure-isolation needs.
- Token-server mempool pages are read-only views over persisted Postgres
  signals. ZMQ and log output are useful for low-latency consumers and
  diagnostics, but they are not the UI/API source of truth.
- The next live-pipeline fix should focus on service ownership and data
  contracts: live block processor publishes confirmed block/state data,
  token-server exposes token/pool read models, mempool detector persists
  semantic signals, alpha consumes token-server APIs plus persisted signals.
- Log audit on May 12 found pool-local simulator warning families that are now
  handled by structured telemetry grouping: Uniswap V3 factory/config pool
  mismatch, Uniswap V3 pool missing at block, and Uniswap V2 pool missing at
  block. These should not count as whole-tracker blockers in Bogaz.
- The same audit found token-server run-log hygiene problems: hundreds of
  restart/run artifacts, repeated MDBX writer contention warnings, and stale
  bind/address-in-use traces. New runs must use date-prefixed run directories
  with `run_manifest.json`, and old generated run artifacts should be cleaned
  before each focused live assessment.

## Next Actions

1. Deploy and verify the `mempool_signal_detector` config fix: live profile must
   load the DB URL, initialize writers, and fail readiness if DB persistence is
   disabled.
2. Install the service-managed Rust live pipeline units:
   `live_block_processor`, `eth_token_server`, `mempool_signal_detector`, and
   `eth_alpha_trader`.
3. Add health/readiness reporting for the mempool detector: `db_enabled`,
   `db_writes`, `db_errors`, latest detected signal, latest persisted signal,
   token context age, and live block age.
4. Backfill or explicitly mark the DB-disabled signal gap from the latest run.
5. Restart token-server after deploying the telemetry/log-layout change and
   verify the new run folder is named `run-<YYYYMMDD-HHMMSSZ>-pid-<pid>` with
   `run_manifest.json` plus the three `pipeline_*.jsonl` files.
6. Continue strategy work only after fresh mempool signals are visible through
   token-server and ASENA.
