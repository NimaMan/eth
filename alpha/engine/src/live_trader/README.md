# Live Trader

Live trader owns live event processing for both no-capital chain simulation and
real Kartal execution.

- `run_live_backtest()` uses live chain-state simulation and never contacts
  Kartal.
- `run_live_real()` uses the crate-private real execution boundary. It defaults
  to dry-run-only Kartal status and permits public mempool only for the explicit
  Alpha11 hold16 validation service flag. Each entry-enabled strategy must
  resolve to a bankroll of at most `0.555 ETH` during validation. Strategy specs
  provide buy size, liquidity floors, bankroll, entry-pool caps, and hold
  windows; live runs select those parameters by strategy name.

Real tx wiring belongs in `real_execution/`; the top-level `mod.rs` keeps the
main orchestration loop. Supporting code is grouped by responsibility:

- `config/`: CLI structs, shared-config key resolution, and live-trader
  constants.
- `startup/`: binary entrypoints, run-start persistence, startup logs, and run
  metadata.
- `runtime/`: token-server client, loop wait/readiness control, poll-error
  handling, heartbeat emission, and execution-stack assembly.
- `state/`: live simulation-state stream, restored runtime state, and position
  state helpers.
- `strategy/`: live strategy spec resolution, strategy installation, bankroll
  accounting, and gas-policy construction.
- `risk/`: mined pool-risk extraction and mempool-signal evidence enrichment.
- `operator/`: manual close request handling.
- `shared/`: cross-cutting helpers for shared config, watermarks, observation
  persistence, parsing, and shutdown signals.
- `block_frame/`: block-boundary contract between chain-server and alpha.
- `event_processing/`: per-tick event ordering and persistence rules.
- `execution_lifecycle/`: submitted execution settlement for chain-sim
  live-backtests.
- `receipt_reconciliation/`: real receipt RPC reads, vault-event decoding, and
  mined execution evidence.

## Live Event Sequence

The intended live behavior is block-coupled:

1. Chain server processes block `N`.
2. Chain server updates live token, pool, gas-rank, and live simulation state
   for block `N`.
3. Chain server publishes a block-applied event for `N`.
4. Alpha consumes that block event and builds strategy inputs for the updates
   from block `N`.
5. Strategies process those inputs and persist decisions, observations, order
   intents, execution reports, and position updates with explicit block context.
6. Alpha finishes block `N` before moving its strategy-processing cursor to
   block `N+1`.

This contract is meant to be the same for live backtest and real live trading.
The execution backend differs, but strategy event ordering and block context
must not.

Current important implementation detail: `/eth/tokens/api/live/updates` is the
block-applied signal and includes updated token/pool ids for the block.
`/eth/tokens/api/live/pools` is a latest live pool surface, not a block delta.
If alpha handles a block event and then reads the latest pool surface, the
surface can include state newer than the block that triggered the loop. That is
the block-coupling gap to remove: alpha should either consume a chain-server
block frame containing the updated snapshots for block `N`, or fetch snapshots
by the updated ids from a view pinned to block `N`.

Live chain simulation has a second block-coupling requirement. A strategy
decision observed at block `N` may target execution in block `N+1`; the
simulator must select state for that required block, not whatever the global
latest live state is when execution happens.

Chain-sim live backtests now mirror real live trading lifecycle. Submission at
block `N` persists a submitted execution report with
`receipt_status = live_backtest_chain_sim_submitted` and
`expected_confirmation_block = N+1`. When a later tick sees the exact live
simulation state for `N+1`, `execution_lifecycle/ChainSimSettlement` loads that
submitted report from Postgres, reconstructs the stored order intent, simulates
the swap as the last transaction in block `N+1`, and persists the final
execution report. If the exact state for `N+1` is unavailable, settlement stays
pending and logs an infrastructure wait rather than writing `buy_failed`.

## Runtime Config

The normal live trader service path reads live settings from the shared root
`config.env`:

- `ALPHA_LIVE_MEMPOOL_SINCE_DAYS`: lookback window used when fetching stored
  mempool signals from the chain server.
- `ALPHA_LIVE_SIGNAL_LIMIT`: max signal rows fetched per trader loop.
- `ALPHA_LIVE_FLASHBOTS_TAIL_MAX_BLOCK_SPAN`: inclusive target-block window
  length for policy-driven tail-entry MEV-Share bundles.

The CLI flags `--mempool-since-days` and `--signal-limit` are explicit operator
overrides only. The checked-in systemd services do not set separate copies of
these values.

## Real Receipt Reconciliation

`receipt_reconciliation/` owns the first real-live settlement worker. On each
live trader tick it:

1. loads submitted execution reports with tx hashes from `alpha_store`;
2. waits until the live processed-block watermark is at least
   `submitted_block + 1`;
3. calls `eth_getTransactionReceipt` on the same RPC URL Kartal reports in
   `/eth/tx/status`;
4. turns receipt `status = 0x0` into a failed `ExecutionReport`;
5. turns receipt `status = 0x1` into a confirmed `ExecutionReport` only when
   the expected V2 vault event is present:
   - `BoughtV2` for buys;
   - `EmergencySoldV2` for sells.

Successful receipts without matching vault evidence stay unresolved. The trader
logs them but does not mark the position confirmed.

Final receipt reports include `mined_evidence` in the persisted JSON payload:
receipt block number/hash, transaction index, cumulative gas, receipt status,
actual gas used, effective gas price, paid gas cost, selected gas limit,
selected max fee, selected priority fee, selected bribe metadata, and the
live-backtest comparison fields `submitted_block_number`,
`expected_confirmation_block`, and `confirmation_lag_blocks`.

The validation finality policy is explicit in the evidence: a mined receipt is
accepted after `1` confirmation for lifecycle accounting, and should be rechecked
at `3` confirmations during the first live validation trades.

## Gate 3 Validation

The live-readiness Gate 3 tests live in `src/gate3_validation.rs` and
`src/live_trader/receipt_reconciliation/`. Run them with:

```bash
cargo test -p eth_alpha_engine gate3 --lib
```

The suite currently locks these real-live assumptions:

- Kartal `broadcast`, `received`, `signed`, `dry_run`, `broadcast_error`, and
  `rejected` responses never become confirmed without receipt evidence.
- `dry_run`, executor `rejected`, planner policy rejects, and pre-submit
  simulation rejects map to cancelled reports because no transaction was
  broadcast.
- Submitted reports carry the selected gas limit, max fee, priority fee, and
  bribe metadata that will later be merged into mined receipt evidence.
- Exact-simulation min-output is non-zero and reverting/full-slippage cases
  reject before submission.
- Simulator state lag is an infrastructure deferral. It must not write
  `buy_failed`, must not consume bankroll, and must not synthesize a submitted
  tx. Deferred buy attempts are excluded from restored seen-pool state so the
  strategy can retry while the entry window remains valid, but the deferred
  position itself is restored as a no-exposure retry anchor so repeated attempts
  stay under one trade id.
- Successful receipts confirm only when the expected deployed V2 vault event is
  present, and the resulting report uses actual vault event amounts plus receipt
  gas cost.
- Receipt evidence records the mined block hash, transaction index,
  cumulative gas used, effective gas price, paid gas cost, the N+1 backtest
  comparison, and the `1`/`3` confirmation policy.
- A buy that is only submitted, pending, or dry-run-cancelled is not sellable.
