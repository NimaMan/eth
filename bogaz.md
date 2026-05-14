# Bogaz

`bogaz.md` is the ETH bottleneck ledger. Keep only active bottlenecks here:
what is limiting the pipeline, the evidence, the owning subsystem, and the next
action. Completed work belongs in focused docs or commit history.

## Current Runtime Snapshot

Snapshot time: `2026-05-14 12:02 Europe/Amsterdam`.

- Chain server user service: `eth-chain-server.service`, active PID `983942`.
- Chain server log run:
  `logs/eth_chain_server/run-20260514-100043Z-pid-983942`.
- Chain server live status after restart: `live`, current block `25,092,641`,
  `786` tracked tokens, `469` tracked pools, `403` V2, `4` V3, `62` V4,
  `1` token tx failure, `last_block_source=live_block_update`.
- Chain server issue log:
  `run-20260514-100043Z-pid-983942/pipeline_issues.jsonl` has `1` row:
  a `token_transaction_update_failed` warning at block `25,091,919`, tx
  `0xd3612113d9f53737c1a6e2892456329d032c73f368e080ad28d9acc6b0067881`,
  because no Uniswap V3 pool was found for token
  `0x8Ef699477219710Ac4540919A374621f1f855510` against WETH at fee tier `100`.
- A transient post-restart failure at block `25,092,345` with detail
  `live chain cache not configured` was fixed. Direct live pool simulations now
  skip optional legacy live-cache/header misses instead of counting the token
  transaction as failed. The fresh run has no such pipeline issue for that
  block.
- Mempool user service: `eth-mempool-signal-detector.service`, active PID
  `557236`.
- Mempool log run:
  `logs/mempool_processor/signal_detector_2026-05-14_10-08-44`.
- Mempool startup now logs:
  `Live data Redis: disabled; simulations use local Reth historical context`.
- Mempool token cache follows chain-server snapshots through HTTP. Latest
  observed fetch was `block=25092084, tokens=832, pools=499`; the external
  update log advanced through `@block 25092084`.
- Mempool simulation target now follows local Reth historical context. The new
  diagnostic line reports
  `Latest simulation block target: 25092083 source=LocalHistoricalContext reth_finished=25092083 historical_context=25092083 live_head=None tracked_state=None`.
- The old stale target `25091681` was caused by long-lived read-only
  `TxSimulator` instances not enabling Reth read-only sync, so their static-file
  header index stayed pinned to the block visible at process start.
- The repeated LP-position approval warning in the previous mempool run was for
  tx `0x229f8213791cc1f4b86711c174cca1bba28ca4e25ae1e733a670bd7cee3376db`,
  a Uniswap V4 PositionManager approval for token id `0x42422`. That token id
  was not present in the tracked position index, so no public LP-position
  approval signal could be enriched. The retry path is now kept out of WARN logs;
  `unresolved_intents.log` remains the record for these pending mappings.
- These are user systemd services. In non-login shells, use
  `XDG_RUNTIME_DIR=/run/user/$(id -u) systemctl --user ...` to inspect them;
  plain system-level `systemctl` will not show these unit names.

## Bogaz Destination Goal

Deploy a real ETH strategy whose policy has been validated by reproducible
recent backtests, live shadow/live-backtest evidence, and audited trade-level
decisions.

This is the Bogaz roadmap target, not the current Codex execution target.

The non-capital path is not the deployment target. It is the calibration layer:
use it to estimate policy behavior against current live data, compare it with
historical backtests, and catch signal/simulation drift before real execution.

Finished means:

- A production strategy policy is selected from the current strategy set.
- The selected policy is backtested over the exact last two-week block window.
- The same policy has live shadow/live-backtest evidence against the current
  chain-server and mempool pipeline.
- Top 10 and worst 10 positions are reviewed with entry reason, skip reason,
  exit reason, gas/slippage assumptions, and PnL snapshot.
- The decision ledger can explain every buy, skip, retry, exit, and failed
  simulation needed for frontend review.
- Execution gates are explicit before real orders are sent: max exposure,
  sizing, retry cadence, exit restrictions, stale-data thresholds, simulation
  freshness, and kill switch behavior.

## Codex Current Goal

Build the live non-capital backtest version of the real strategy evidence
layer.

This workstream should produce a current-regime estimate for the candidate
policy, not a final real-execution approval. It must make coverage explicit:
the latest replay source currently reaches live blocks, but its historical
observation coverage starts at block `25,066,498`, so a true two-week replay
requires either a longer durable observation source or a historical observation
builder.

Finished means:

- The candidate policy is run against the latest available live observation
  source with a fixed block window and run id.
- The report states the requested window, actual observation coverage, and
  whether the run is a full two-week replay or a partial current-regime replay.
- Strategy Lab output is available for summary, concentration, top 10, worst
  10, buy failures, and open failed exits.
- The no-capital run persists strategy decision rows for buy, skip, hold, risk
  check, retry, and exit decisions, and those rows are exposed through the
  chain-server alpha run API for frontend review.
- Any missing data needed for a true two-week replay is recorded as the next
  limiting factor.

Latest evidence:

- `live-noncapital-maxhold50-twoweek-requested-20260514-1020` ran the current
  best candidate, `snipe-all-v1` with `--max-hold-blocks 50`, against requested
  window `24,991,345..25,092,144`.
- The run completed with `127,878` events, `1,089` execution reports, `983`
  confirmed reports, `106` failed reports, `533` positions, and `59` open
  positions.
- Strategy Lab reports total PnL `+12.876052023155642365 ETH`, PnL excluding
  top 10 `+5.913359147437672841 ETH`, `12` buy-failed positions, and `59`
  sell-failed positions.
- The report is
  `alpha/lab/reports/live-noncapital-maxhold50-twoweek-requested-20260514-1020.md`.
- Coverage is still partial: the requested two-week window starts at
  `24,991,345`, but usable pool observations start at `25,066,498`.
- A bounded retry comparison,
  `live-noncapital-maxhold50-retry20x3-twoweek-requested-20260514-1030`,
  was run after adding opt-in retry cadence. It left PnL and open exposure
  unchanged while increasing failed reports from `106` to `214`, so retry-only
  is not the next useful policy.
- A risk-exit comparison,
  `live-noncapital-maxhold50-riskbundle-twoweek-requested-20260514-1040`,
  included stored mempool signals and enabled liquidity-removal, LP-approval,
  tax, and scam exits. It improved total PnL to
  `+12.928123726556627169 ETH` and PnL excluding top 10 to
  `+5.965430850838657645 ETH`, with the same `59` sell-failed positions.
  Risk exits moved five confirmed sells earlier by up to `51` blocks.
- Decision-ledger persistence has been added to the alpha engine/store and
  exposed through chain-server at
  `/eth/tokens/api/alpha/runs/{run_id}/decisions`. The smoke run
  `decision-ledger-smoke-20260514-1045` wrote `832` decision rows, all with a
  reason, including `27` buy submissions and `25` max-hold monitor sells.
- The full candidate decision-ledger run,
  `live-noncapital-maxhold50-riskbundle-decisions-twoweek-requested-20260514-1053`,
  covered requested window `24,991,500..25,092,298` and completed with
  `128,242` events, `1,091` execution reports, `985` confirmed reports, `106`
  failed reports, `534` positions, and `59` open positions. Strategy Lab reports
  total PnL `+12.927265783819489334 ETH` and PnL excluding top 10
  `+5.964572908101519810 ETH`.
- Sell-failure investigation showed that `53` of the `59` sell-failed positions
  had a later pool observation where reserve fell below `0.1 ETH`, `can_sell`
  became false, and `is_scam` became true before the max-hold-50 exit. The
  transition age distribution was `{6,22,29,41,50}` blocks, so the current
  max-hold-50 policy often waits through the rug transition.
- Two shorter live no-capital variants were run on the same requested window
  after centralizing failed-exit retries in the position-monitor path:
  `live-noncapital-maxhold10-riskbundle-retryfix-twoweek-requested-20260514-092804Z`
  and
  `live-noncapital-maxhold20-riskbundle-retryfix-twoweek-requested-20260514-092804Z`.
  `maxhold10` cut sell-failed positions to `9` with total PnL
  `+7.710342857256795193 ETH` and ex-top-10 PnL
  `+5.661370663780626751 ETH`. `maxhold20` cut sell-failed positions to `23`
  with total PnL `+10.906166875635052281 ETH` and ex-top-10 PnL
  `+7.505512163468218752 ETH`. Current no-capital evidence favors `maxhold20`
  over `maxhold50`: less exit risk and better ex-top-10 PnL, while giving up
  some top-winner upside.
- The full candidate run wrote `27,889` strategy-decision rows, all with a
  reason, including `534` buy submissions and `552` sell submissions. The top
  10 winners and worst 10 losers are explainable from the ledger: all have
  `entry.buy_eligible_pool_once` entries and `exit.max_hold` exits; five of the
  worst 10 still have `TransferHelper: TRANSFER_FROM_FAILED` sell failures.
- Chain-server also exposes the joined top/worst audit endpoint at
  `/eth/tokens/api/alpha/runs/{run_id}/decision-audit`. For the full candidate
  run it returns `20` rows: `10` top and `10` worst positions with entry reason,
  exit reason, sell report status, PnL, and ROI.
- Order lifecycle persistence now records a `submitted` execution report before
  the simulated final report when the adapter returns `confirmed` or `failed`
  directly. The smoke run
  `lifecycle-smoke-20260514-095812Z` wrote `12` `buy/submitted`, `12`
  `buy/confirmed`, `10` `sell/submitted`, `9` `sell/confirmed`, and `1`
  `sell/failed` report. This makes the position lifecycle visible without
  changing existing chain-sim PnL semantics.
- Chain-server report APIs now expose `position_id` and `order_side`, and the
  Asena historical backtest detail route has `Lifecycle` and `Decision Audit`
  tabs. The lifecycle tab groups execution reports into steps such as
  `buy_submitted`, `buy_confirmed`, `sell_submitted`, and `sell_failed`.
  Existing candidate runs need to be rerun to show submitted rows because older
  runs only persisted final execution reports.
- The current `maxhold20+risk exits` candidate was rerun as
  `live-noncapital-maxhold20-riskbundle-lifecycle-twoweek-requested-20260514-100515Z`
  over requested window `24,991,500..25,092,658`. It completed with `129,121`
  events, `546` positions, `1,080` final execution reports, `1,080`
  submitted lifecycle rows, `1,044` confirmed final reports, `36` failed final
  reports, and `24` open sell-failed positions. Strategy Lab reports total PnL
  `+10.937672043030798303 ETH` and PnL excluding top 10
  `+7.537017330863964774 ETH`.
- Failed-buy investigation for that run found `12` failed buy positions:
  `7` Uniswap V4 Universal Router reverts, `3` Uniswap V2
  `TRANSFER_FAILED`, and `2` Uniswap V3 `TF` reverts. All `12` were approved
  by the strategy as `submit_buy / entry.buy_eligible_pool_once`.
- The V4 failures are an eligibility-source bug: the source observations have
  top-level `can_buy=true` / `can_sell=true`, but their `runtime_state` lacks
  direct `can_buy/can_sell` flags. The faulty semantics were that
  `eth_chain_server` pool summaries computed top-level trading flags from direct
  simulation state OR observed third-party flow. The code now removes that
  equivalence: observed flow is exposed as explicit
  `has_observed_buy/has_observed_sell` evidence fields, while strategy-facing
  `can_buy/can_sell` means our own direct simulation path.
- Representative V4 probes failed at the entry block and `block-1` for
  `0.01`, `0.001`, and `0.0001 ETH`, so these should become skipped entries,
  not failed buys. Representative V2/V3 failures are size-sensitive: they fail
  at the fixed `0.01 ETH` buy amount but pass buy/approve/sell probes at
  `0.001 ETH`.
- All `12` failed-buy pools later degraded into bad pool state in the source
  observations: `is_scam=true` or `can_buy/can_sell=false`, often with zero
  denom reserve. These are not attractive missed entries; the issue is that
  pre-buy eligibility accepted them too early.
- After separating observed-flow evidence from direct trading flags, the
  candidate was regenerated as
  `live-noncapital-maxhold20-riskbundle-v4runtimeflags-twoweek-requested-20260514-104701Z`
  over requested window `24,991,500..25,092,869`. The old replay source has no
  stored `runtime_state.can_buy/can_sell` flags, so alpha replay now treats V4
  observations without runtime flags as not buyable/sellable instead of falling
  back to old top-level fields. The regenerated run completed with `129,607`
  events, `427` positions, `849` final reports, `1,698` lifecycle reports,
  `822` confirmed final reports, `27` failed final reports, and `22` open
  sell-failed positions. Strategy Lab reports total PnL
  `+10.937447237497281828 ETH` and PnL excluding top 10
  `+7.536792525330448299 ETH`. Confirmed buys are now `418` Uniswap V2,
  `3` Uniswap V3, `1` PancakeSwap V2, and `0` Uniswap V4. Failed buys dropped
  from `12` to `5`: `3` V2 `TRANSFER_FAILED` and `2` V3 `TF`; the `7` V4
  Universal Router failures are gone.

## Active Bottlenecks

| Order | Bottleneck | Owner | Evidence | Next Action |
| --- | --- | --- | --- | --- |
| 1 | **Real strategy deployment readiness** | `alpha/strategies`, `alpha/engine`, `tx_executor`, frontend | The main target is real live deployment, not a no-capital endpoint. The current best live-aligned no-capital evidence is now the V4-runtime-guarded `maxhold20+risk exits` rerun: `+10.937447237497281828 ETH` total PnL, `+7.536792525330448299 ETH` excluding top 10, `5` buy-failed positions, and `22` sell-failed positions. Evidence is still incomplete for real orders because the source observation coverage starts at `25,066,498`, older observations lack runtime trading flags, and execution gates are not finalized. | Treat this rerun as the current no-capital candidate. Review it in Asena, then resolve remaining fixed-size entry failures, failed-exit policy, and real execution gates before handoff. |
| 2 | **Strategy policy quality and concentration** | `alpha/strategies`, `alpha/engine` | V4-runtime-guarded `maxhold20+risk exits` keeps nearly the same total and ex-top-10 PnL as the prior candidate while removing all V4 failed buys. `maxhold10` still reduces sell-failed positions further, but gives up more total and ex-top-10 PnL. | Keep `maxhold20+risk exits` as the baseline candidate. Compare any new policy against this run's PnL, concentration, failed buys, failed exits, and decision-ledger audit quality. |
| 3 | **Buy-entry eligibility correctness** | `eth_chain_server`, `alpha/engine`, `alpha/backtest`, `alpha/strategies` | V4 observed-flow-only entries are fixed for new chain-server snapshots and old replays: V4 observations without runtime direct-route flags are not treated as buyable/sellable. Failed buys dropped from `12` to `5`; the remaining `3` V2 and `2` V3 failures are fixed-size failures where representative probes fail at `0.01 ETH` but pass at `0.001 ETH`. | Decide adaptive sizing or a strict fixed-size route gate for V2/V3. Keep V4 runtime-flag requirement in place until observations are rebuilt with direct-route probes. |
| 4 | **Sell restriction and exit policy** | `alpha/engine`, `tx_processor`, `tx_simulator` | The latest guarded rerun still has `22` open sell-failed positions. Prior sell-failure analysis showed mostly rug timing: failed exits later had reserve below `0.1 ETH`, `can_sell=false`, and `is_scam=true`, with median transition age `29` blocks. Retry-only did not recover exits. | Do not promote retry-only. Next policies should test stronger early warning from LP approvals/liquidity-removal mapping, stop-loss/reserve-drop exits, and entry filters for pools without robust observed sell support. |
| 5 | **Strategy-review lifecycle completeness** | `alpha/engine`, `alpha/store`, `eth_chain_server`, frontend | Submitted order lifecycle rows are now persisted for new runs and exposed through chain-server reports with `position_id` and `order_side`. The frontend historical backtest detail view renders `Lifecycle` and `Decision Audit` tabs. The full candidate run `live-noncapital-maxhold20-riskbundle-lifecycle-twoweek-requested-20260514-100515Z` verifies submitted/final rows for buy and sell and exposes top/worst audit rows in the frontend. | Review the candidate in Asena, especially the 24 sell-failed positions and worst 10 decision audit rows. Any next policy must improve against this lifecycle-enabled baseline. |
| 6 | **Execution handoff readiness** | `alpha/engine`, `tx_executor` | Real deployment needs a final handoff contract: order sizing, exposure caps, stale-data checks, simulation freshness threshold, retry cadence, kill switch behavior, and failure logging. | Keep execution wiring explicit and gated. The selected strategy can move to real orders only after the policy evidence and runtime gates are both visible in logs/frontend. |
| 7 | **Uniswap V3 pool identity miss in live token apply** | `eth_token`, `eth_chain_server` | The fresh chain-server run now has `tx_failures=1` and one `pipeline_issues.jsonl` row. At block `25,091,919`, token transaction apply failed because a Uniswap V3 pool for token `0x8Ef699477219710Ac4540919A374621f1f855510` / WETH / fee tier `100` was not present in the tracked registry when the token update needed it. | Reproduce that block from disk cache/Reth and inspect whether the V3 `PoolCreated` event was missed, filtered out by retention, mis-keyed by token orientation, or unavailable before the token tx. Decide whether this class should be strict failure or optional pending metadata. Chain-server soak should return to `tx_failures=0`. |
| 8 | **LP position approval mapping coverage** | `eth_token`, `eth_chain_server`, `mempool_processor` | The previous mempool run repeatedly retried a V4 PositionManager approval for token id `0x42422`, but the live pool cache had no tracked position context for that id. Current live pools expose 94 V3/V4 pools and only 13 pools with non-empty `liquidity_positions`, so many valid position approvals cannot be enriched into public LP-position approval signals. | Decide whether missing position mappings should stay as unresolved intents only, or whether chain-server should backfill position context on approval by querying the position manager for token id -> pool key/owner/liquidity. Keep public signals blocked unless a token/pool/share mapping is known. |
| 9 | **Residual Redis code outside the production chain-server path** | `tx_simulator`, `tx_processor`, `alpha/live/state`, docs | Production chain-server live token tracking no longer uses Redis and `TxSimulator::new()` no longer auto-attaches Redis. Mempool startup no longer accepts a Redis live cache. Legacy modules still exist: `tx_simulator::live_chain_cache`, `live_data_registry`, `tx_processor::live::{redis_block_publisher, block_notifier}`, `LiveProcessedBlockProvider`, and `alpha/live/state` Redis key contracts. | Decide which legacy Redis pieces are still needed for diagnostics or compatibility. Feature-gate or remove unused production exports after the mempool simulator freshness fix is in place. Update READMEs that still describe Redis as a normal runtime dependency. |
| 10 | **Mempool startup ordering** | `mempool_processor`, systemd units | On restart, mempool starts after the chain-server process, but before the HTTP listener is ready. It logs one `Connection refused` hydrate failure, then waits for token cache population and recovers. This is noisy and can delay startup diagnostics. | Add a readiness wait or health-check loop before first hydrate, or use a chain-server health endpoint in service startup. Keep the current retry path, but make the first connection-refused warning less alarming if startup is still inside the grace window. |
| 11 | **Strict processed-block replay readiness** | `tx_processor`, `tx_simulator`, `reth_chain_query` | Prior warmup/live replay hit mined transaction validation failures such as `lack of funds` when local state context lagged or was sparse. Current chain-server run has one V3 identity failure, but no stale-Reth validation failure. | Keep block processing strict. Add readiness/parity regression checks around historical context startup and known problematic blocks `25,078,746` and `25,033,700`. |
| 12 | **Token candidate-selection scaling** | `eth_token` | The current live run is mostly healthy, but the known algorithmic risk remains: V3/V4 ERC721 transfer/approval candidates can scan tracked registry state instead of using a position-manager/token-id index. This can reappear as tracked pool count grows. | Implement indexed V3/V4 LP-position and approval lookup keyed by position manager, token id, owner, and operator. Candidate selection should scale with events in the transaction, not total tracked tokens/pools. |

## Redis Cleanup Status

Done:

- `eth_chain_server` owns the live block loop, direct `LiveBlockUpdate` handoff,
  live token runtime, recent processed-block ring, and live APIs.
- Chain-server live tail uses `last_block_source=live_block_update`; it does
  not tail `eth/live/blocks` or load live blocks through
  `LiveProcessedBlockProvider`.
- `TxSimulator::new()` and `TxSimulator::with_provider_factory()` no longer
  auto-attach Redis live state. Redis live cache is explicit opt-in via
  `with_default_live_chain_cache()` or `with_live_chain_cache(...)`.
- `mempool_signal_detector` no longer constructs a Redis `LiveChainCache` at
  startup and no longer has `simulation.live_data_redis_url` in its config.
- Long-lived `TxSimulator::new()` instances enable Reth read-only sync, so
  local historical simulation context can advance with the live Reth node
  instead of staying pinned to process-start static files.
- Redis-free optional metadata misses such as `live chain cache not configured`
  are classified as optional discovery misses, not live token transaction
  failures.
- Direct live pool trading simulation no longer falls back to the legacy live
  cache when a previous-block direct state session is unavailable; it skips that
  optional pool simulation and keeps the token block apply successful.
- Direct live pool trading simulation also treats internal
  `live chain cache not configured` / missing live header misses as optional
  context misses in Redis-free live mode. Verified on block `25,092,345`: the
  fresh run has `transaction_error_count=0` and no pipeline issue for that
  block.

Still present by design or migration debt:

- `tx_simulator/src/block_context/live_chain_cache/` and
  `live_data_registry/` remain as legacy Redis snapshot helpers.
- `tx_processor/src/live/redis_block_publisher.rs`,
  `tx_processor/src/live/block_notifier.rs`, and
  `tx_processor/src/processed_tx_provider/block/live.rs` still implement the
  old Redis block transport.
- `alpha/live/state` still owns the old `eth/live/...` Redis key contract.
- Several READMEs and docs still describe Redis as a normal live-state path.

Do not delete the remaining Redis modules until all remaining consumers are
confirmed dead or explicitly migrated; otherwise we risk breaking diagnostics,
examples, or back-compat tooling.

## Verification Commands

Recent checks that passed during this cleanup:

- `cargo test -p eth_token token_update_router --lib`
- `cargo check -p eth_chain_server`
- `cargo check -p mempool_processor`
- `cargo build -p eth_chain_server -p mempool_processor --release --bin eth_chain_server --bin mempool_signal_detector`
- `cargo run -p tx_simulator --release --example verify_database_setup`
- `cargo fmt --check`
- `cargo test -p eth_alpha_engine --lib`
- `cargo check -p eth_alpha_engine -p eth_alpha_store -p eth_chain_server -p eth_alpha_backtest`
- `npm run build` from `interface/new_Asena`
- `cargo run -q -p eth_alpha_backtest --bin eth_alpha_backtest -- --run-id lifecycle-smoke-20260514-095812Z --replay-run-id snipe-all-v1-chain-sim-live-v4 --from-block 25073540 --to-block 25073620 --skip-primed --include-mempool-signals --buy-amount-wei 10000000000000000 --min-liquidity-eth 0.5 --min-liquidity-usd 1000 --max-hold-blocks 20 --exit-liquidity-removal --exit-lp-approval --exit-tax --exit-scam`
- `cargo run -q -p eth_alpha_backtest --bin eth_alpha_backtest -- --run-id live-noncapital-maxhold20-riskbundle-v4runtimeflags-twoweek-requested-20260514-104701Z --replay-run-id snipe-all-v1-chain-sim-live-v4 --from-block 24991500 --to-block 25092869 --skip-primed --include-mempool-signals --buy-amount-wei 10000000000000000 --min-liquidity-eth 0.5 --min-liquidity-usd 1000 --max-hold-blocks 20 --exit-liquidity-removal --exit-lp-approval --exit-tax --exit-scam`
- `cargo run -q -p eth_alpha_lab --bin eth_alpha_lab -- strategy --run-id live-noncapital-maxhold20-riskbundle-v4runtimeflags-twoweek-requested-20260514-104701Z --limit 10`
- `cargo check -p eth_chain_server -p eth_token`
- `cargo build --release -p eth_chain_server --bin eth_chain_server`

## Immediate Next Actions

1. Decide whether the next blocker is policy quality or missing historical
   observation coverage for a true two-week replay.
2. Fix the remaining fixed-size entry failures: either adaptive sizing based on
   direct-route probes or a strict route gate at the configured `0.01 ETH` buy
   amount.
3. Implement the next failed-exit policy improvement against the current
   `maxhold20+risk exits` baseline: chunked exits, no-observed-sell exposure
   limits, or earlier risk-driven exits. Retry cadence exists now, but
   retry-only was not useful in the latest comparison.
4. Review the lifecycle-enabled candidate in the frontend, including top
   winners, worst losers, skipped entries, lifecycle rows, and failed exits.
   Use that review to decide whether the next policy change is an exit rule,
   entry filter, or missing historical coverage task.
5. Define the real production strategy candidate and its execution gates:
   sizing, max exposure, retry cadence, stale-data threshold, simulation
   freshness threshold, exit restrictions, and kill switch behavior.
6. Backtest the selected candidate over the exact last two-week block window and
   keep the run reproducible from the frontend once observation coverage exists.
7. Run the same policy in live shadow/live-backtest mode against the current
   chain-server and mempool pipeline. Treat this as a rough current-regime
   estimate and drift detector, not as the deployment target.
8. Reproduce the block `25,091,919` V3 pool identity miss and restore
   chain-server soak expectations to `tx_failures=0` and `pipeline_issues=0`.
9. Decide the policy for unmapped V3/V4 position approvals: unresolved-only, or
   on-demand position-manager backfill before public signal publication.
10. Add a mempool readiness wait against chain-server HTTP health.
11. Run a longer chain-server and mempool soak:
   chain-server must keep `tx_failures=0` and `pipeline_issues=0`, while
   mempool simulation target lag should stay within the accepted threshold.
12. Remove or feature-gate legacy Redis modules only after the production
   mempool path no longer needs any Redis-compatible fallback.
13. Continue the broader strategy comparison as supporting evidence, but let the
   exact two-week backtest and live shadow/live-backtest drive the real
   deployment decision.
