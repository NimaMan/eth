# Live Trader

Live trader owns live event processing for both no-capital chain simulation and
real Kartal execution.

- `run_live_backtest()` uses live chain-state simulation and never contacts
  Kartal.
- `run_live_real()` uses the crate-private real execution boundary. It defaults
  to dry-run-only Kartal status and permits broadcast only for the explicit
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

```text
reth/node publishes new execution head N
  -> chain-server receives it
  -> chain-server fetches block N by hash
  -> chain-server fetches prestate diffs for the same hash
  -> chain-server updates LiveTxSimulator session for N
  -> chain-server updates token/pool state for N
  -> chain-server builds/publishes LiveBlockFrame N
  -> Alpha live backtest / Alpha real consume next frame N
  -> strategies make decisions pinned to N/hash
  -> chain-sim mode settles via chain-server simulation
  -> real mode builds tx plan and submits through Kartal
  -> Kartal validates/signs/broadcasts according to policy
```

This contract is meant to be the same for live backtest and real live trading.
The execution backend differs, but strategy event ordering and block context
must not.

Current important implementation detail:
`/api/v1/eth/live-trading/block-frames/next` is the confirmed-chain strategy
input. It returns the updated pool snapshots for one applied block and the
status/progress pinned to that block. Alpha does not read
`/api/v1/eth/live-token-tracker/pools` in the live decision path; that endpoint
is a latest read-model surface for UI/context clients. The `next` endpoint is a
replayable push/long-poll boundary: Alpha sends its last processed block and
chain-server returns, or waits for, the next retained frame.

Live chain simulation has a second block-coupling requirement. A strategy
decision observed at block `N` may target execution in block `N+1`; the
simulator must select state for that required block, not whatever the global
latest live state is when execution happens.

Chain-sim live backtests now mirror real live trading lifecycle. Submission at
block `N` persists a submitted execution report with
`receipt_status = live_backtest_chain_sim_submitted` and
`expected_confirmation_block = N+1`. If the live status includes block `N`'s
hash, Alpha stores it in the submitted evidence. When a later tick reaches
`N+1`, `execution_lifecycle/ChainSimSettlement` loads that submitted report from
Postgres, reconstructs the stored order intent, and asks chain-server to
simulate the swap as the last transaction in block `N+1`. Chain-server owns the
only live `LiveTxSimulator`; Alpha does not subscribe to live-state stream
frames or rebuild live state locally. If exact state for `N+1` is unavailable or the
submitted block hash no longer matches the execution block parent, settlement
stays pending and logs an infrastructure wait rather than writing `buy_failed`.

## Per-Block Logs

For each live block frame consumed by Alpha at info level:

- `alpha trader tick`: human-readable per-block summary. It includes
  `live_current_block`, tracked token/pool counts, `pools_seen`,
  `block_frame_pool_count`, mempool `signal_count`, emitted `market_events`,
  `risk_events`, `position_monitor_events`, execution report counts, position
  count, and `chain_sim_state_source`.
- `pipeline_health`: structured health event for the same tick. It carries the
  same counts in a machine-readable metrics map.

At debug level:

- chain-server logs `recorded alpha live block frame` with block number, block
  hash, pool count, and token count when the frame is stored.
- Alpha logs `alpha trader consumed live block frame` with frame event, frame
  block, frame pool count, and last processed frame block.

Upstream chain-server/live-feed logs at info level:

- `block token processor profile`: per-block token processor counts and
  timings, including transaction counts, updated/created token counts,
  candidate routing work, pool update work, and simulation candidate counts.
- `live token apply profile`: per-block timing for block processing, retention,
  token apply, state update, disk cache read/write, and wall time.
- `live token runtime applied block`: progress for each live-tail block,
  including current block, processed counts, tracked token/pool counts, failures,
  disk cache counts, and last token apply timing.

Execution-specific logs are emitted only when work happens: chain-sim
submission/settlement reports, real receipt reconciliation, skipped mempool
signals, manual closes, and warnings for unavailable exact simulation state.

## Runtime Config

The normal live trader service path reads live settings from the shared root
`config.env`:

- `ALPHA_LIVE_MEMPOOL_SINCE_DAYS`: lookback window used when fetching stored
  mempool signals from the chain server.
- `ALPHA_LIVE_SIGNAL_LIMIT`: max signal rows fetched per trader loop.
- `ALPHA_LIVE_TAIL_ENTRY_PRIORITY_UNDERCUT_WEI`: wei amount subtracted from
  the observed dependency tx priority fee for public tail-entry buys.
- `ALPHA_LIVE_TAIL_ENTRY_MAX_FEE_BUFFER_BPS`: base-fee buffer added to the
  selected public tail-entry max fee.

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
