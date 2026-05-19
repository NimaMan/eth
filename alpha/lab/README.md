# ETH Alpha Lab

Offline diagnostics for alpha strategy and position sanity checks.

The lab reads existing `alpha_trading` tables and does not participate in the
live trading path. It currently uses:

- `trader_runs`
- `positions`
- `execution_reports`
- `position_snapshots`
- `strategy_observations`
- `backtest_result_sets`
- `trades`
- `trade_events`
- `trade_snapshots`
- `risk_events`
- `strategy_decisions`

The backtest validator writes `alpha_trading.backtest_validation_reports` when a
validation report is persisted. That table belongs to lab diagnostics, but it is
documented in `alpha/store/README.md` because it lives in the shared
`alpha_trading` schema.

## Layout

The lab crate is the umbrella for offline diagnostics. Keep each lab as a
top-level module under `src/` rather than nesting modules under a generic
`labs/` directory.

```text
src/
  bin/eth_alpha_lab.rs
  backtest_validation/
  strategy_assessment/
  strategy_lab/event_trace/
  position_lab.rs
  strategy_lab.rs
  render.rs
```

## Strategy Lab

Run-level checks for PnL, concentration, failures, protocol mix, missing
snapshots, open failed exits, and accounting anomalies.

The lab reads `ALPHA_DATABASE_URL` from `blockchains/eth/config.env` by default.

```bash
eth_alpha_lab strategy \
  --run-id hist-poolonly-mtm-v4-20260512-1021
```

Multiple runs can be reported together:

```bash
eth_alpha_lab strategy \
  --run-id hist-poolonly-mtm-paper-20260512-1021 \
  --run-id hist-poolonly-mtm-v4-20260512-1021
```

## Trade Event Trace

Event-timing diagnostics for a single trade or for all losing trades in a
strategy result. This is the repeatable workflow for asking:

> What happened around the pool before entry, during the buy-confirmation
> block, before sell submission, and before sell confirmation?

Single trade:

```bash
eth_alpha_lab trade-events \
  --result-set historical-25090165-25110164 \
  --strategy snipe-all-risk-atlas-lp-gate-hold15-immediate-lp-exit \
  --trade-id trd_mp8vc73a_23ggp_145
```

Losing-trade scan:

```bash
eth_alpha_lab losing-trades \
  --result-set historical-25090165-25110164 \
  --strategy snipe-all-risk-atlas-lp-gate-hold15-immediate-lp-exit \
  --limit 20
```

The scan groups repeated timing signals across losing trades, including:

- LP approval visible before buy submission;
- LP approval in the buy-confirmation block;
- LP approval after entry but before sell;
- direct liquidity removal before or during sell confirmation;
- max-hold exits that lost money;
- mark-to-market crossing below entry before exit;
- gas costs that materially explain the loss.

Use `--json` when the scan output should feed another tool or a UI page.

## Token Lab

Single position/token checks for entry report consistency, observation joins,
latest valuation, and first/last mark-to-market trajectory.

```bash
eth_alpha_lab position \
  --run-id hist-poolonly-mtm-v4-20260512-1021 \
  --token 0x12a77658112Cf42914cB614D13653ed5852DA1e5
```

If more than one position matches a token, use `--position-id`.

Both commands support `--json` for automation.

## Backtest Validation

Trade-centric validation for a persisted backtest result set. This is the first
gate before trusting a strategy comparison, a top winner, or a reported PnL.

### Objective And Model

The objective is:

> Would this exact strategy, seeing only evidence available at the time, have
> produced these trades and this PnL if we had actually run it?

The validator is not only checking whether rows exist. It is checking whether
the persisted result is a faithful estimate of the strategy under the backtest
execution model.

The current execution model is:

- the backtester loops over blocks in order and writes all position, trade,
  snapshot, event, decision, and risk-evidence rows from that block loop;
- when the strategy submits an order, the simulator assumes the order is
  accepted in the next block;
- execution is simulated against the post-state of that next block, which
  effectively models our order as the last relevant trade in that block;
- buy fills are not guessed: the recorded token amount must come from the
  chain simulator for that buy path and block;
- sell fills are not guessed: the recorded denomination received must come
  from the chain simulator for the held token amount and sell block;
- historical backtests must use mined/local evidence only, while live
  backtests may also use mempool evidence that was observed before the
  decision.

Under this model, a closed trade is pure cash accounting:

```text
closed PnL = sell denomination received - buy denomination paid - gas
```

A closed trade must not have unrealized PnL unless we explicitly model a
partial sell or residual token balance. If residual tokens exist, they must be
stored and valued as a separate open exposure, not hidden inside a closed trade.

### Validation Layers

Validation should run in layers. A later layer is not meaningful if an earlier
layer fails.

1. **Persistence coherence**: while the block loop runs, are the result-set,
   trade, event, decision, risk, and snapshot rows written consistently?
2. **Decision reproducibility**: if the same event stream is replayed into the
   strategy, do we get the same buy/sell decisions at the same blocks?
3. **No information leakage**: did every decision use only evidence available
   at or before that decision block, or observed mempool evidence for live
   backtests?
4. **Buy execution replay**: can the simulator reproduce the stored token amount
   at the buy-confirmed block under the next-block/post-state execution model?
5. **Sell execution replay**: can the simulator reproduce the stored
   denomination received at the sell-confirmed block for the exact token amount
   held?
6. **Cash and residual accounting**: is closed PnL cash-based, and are any
   residual tokens tracked explicitly as remaining exposure?
7. **Fair comparison**: strategies compared together must share the same block
   window, input stream, execution model, gas model, capital sizing, and token
   universe.
8. **Assessment handoff**: robustness, concentration, exposure materiality, and
   tail-loss questions are assessed after validation by `strategy_assessment`.

```bash
eth_alpha_lab backtest-validation \
  --result-set historical-25090165-25110164 \
  --strategy snipe-all-risk-atlas-lp-gate-hold15-v2-uniswap-v2-only
```

The command has one validation mode: comprehensive. Persisted reports use
`profile = "comprehensive"` for compatibility with the existing report table,
but callers should not choose a validation profile.

Use `--json` for machine-readable reports.

### Validation Procedure

The validation procedure is intentionally ordered. Do not start by inspecting
PnL; first prove the result set is internally coherent.

1. Load the result set, run provenance, strategy config, scoped trades, events,
   decisions, risk events, snapshots, and execution reports from
   `alpha_trading`.
2. Validate metadata and scope:
   - historical result sets should be `completed`;
   - live result sets should be `running` or `stopped`;
   - the selected strategy must have trades;
   - public trade IDs must use the `trd_` prefix;
   - historical result sets must not include pending mempool risk rows.
3. Validate lifecycle:
   - every trade has buy submit and buy confirmation in order;
   - `entry_block` equals the `buy_confirmed` event block;
   - sell submit cannot precede buy confirmation;
   - `exit_block` exists only for `sell_confirmed` trades;
   - `exit_block` equals the `sell_confirmed` event block;
   - duplicate terminal buy/sell confirmations are failures.
4. Validate decision timing:
   - every submitted buy/sell event has a same-block `strategy_decisions` row;
   - risk-driven sell decisions must have local `risk_events` evidence at or
     before the decision block.
5. Validate accounting:
   - `total_pnl_eth = realized_pnl_eth + unrealized_pnl_eth`;
   - for closed trades,
     `realized_pnl_eth = exit_value_eth - entry_cost_eth - gas_cost_eth`;
   - closed trades must have zero `current_value_eth` and zero
     `unrealized_pnl_eth`;
   - sell-confirmed snapshots must also have zero unrealized PnL so a later
     snapshot cannot corrupt the aggregate trade row.
6. Validate snapshots:
   - `latest_snapshot_block` must equal the max persisted trade snapshot block;
   - closed trades should have a final `sell_confirmed` snapshot at `exit_block`.
7. Validate replay readiness:
   - closed trades must retain buy-confirmed token amount, sell order amount,
     and sell-confirmed filled amount. These are the required inputs for
     independent chain-sim replay.
8. Review samples:
   - top winners and worst losers are printed as supporting forensic rows for
     debugging validation failures. Profit concentration and tail-risk
     questions are answered by `strategy_assessment`, not by validation.

### Verdicts

- `pass`: the invariant holds.
- `fail`: do not trust the scoped result until fixed.
- `blocked`: the check could not run because required external state or replay
  support is unavailable.

`warn` can still appear in older persisted reports, but current validation
checks should produce only `pass`, `fail`, or `blocked`.

## Strategy Assessment

Strategy assessment runs after validation and asks quality questions that do not
belong in correctness validation. The implemented questions are documented in
`src/strategy_assessment/README.md` with their formulas and thresholds.

Current questions:

- Does positive PnL survive removing top winners?
- Is gross profit large enough to absorb gross loss?
- Are losses concentrated in a small tail we can explain or avoid?
- Does unresolved exposure materially affect the strategy conclusion?
- Are failed exits leaving unresolved exposure?

```bash
eth_alpha_lab strategy-assessment \
  --result-set historical-25090165-25110164 \
  --strategy snipe-all-risk-atlas-lp-gate-hold15-v2-uniswap-v2-only
```

### EVM Replay Policy

The current `backtest-validation` command validates DB coherence and replay
readiness. It does not yet prove full strategy reproducibility or independent
EVM execution replay for every sampled trade.

The next validation layer is EVM replay over sampled trades using the existing
tx-processor buy/sell simulators. Replay must use the same next-block,
post-state execution model as the backtest. Until that is wired into the lab
command, use the printed top/worst samples with `probe_sell_swap` and
`probe_pool_buy_sell` for forensic replay.

For each sampled closed trade, replay should prove:

- the recorded token amount is obtainable from the simulator at the
  buy-confirmed block;
- the recorded token amount is sellable in the simulator at the sell-confirmed
  block;
- the simulated denomination received matches `sell_confirmed.filled_amount`;
- gas accounting uses the same approval-plus-swap policy as the backtest;
- the exit signal was visible no later than the sell-submitted block.

For profitable strategies, sample replay must include:

- top winners;
- worst losers;
- liquidity-removal or LP-warning exits;
- failed sells;
- random normal trades.

For strategy-level replay, the lab should eventually replay the complete input
event stream into the strategy and compare generated decisions against
`strategy_decisions`. That is the check that proves the result is not only
internally coherent, but reproducible from the historical/live evidence stream.

## Current Strategy Issue Ledger

Scope: latest clean 15k historical baseline
`hist-snipe-all-poolonly-15k-recipient-net-20260513-145954`, range
`25,065,694..25,080,693`, source replay
`snipe-all-v1-chain-sim-live-v4`.

Run facts from Strategy Lab:

- 449 total positions, 353 open positions, 19 failed positions.
- 11 buy-failed positions and 8 sell-failed positions.
- 554 execution reports: 523 confirmed, 31 failed.
- Snapshots across 438 positions; zero open positions without a snapshot.
- Total PnL `+9.693440469949935980 ETH`.
- PnL excluding top 5 positions is `-0.219822746621256746 ETH`.
- PnL excluding top 10 positions is `-2.606516897707689237 ETH`.
- Earlier May 13 reruns confirmed the same remaining failure buckets; the
  balance-overlay run cleared the 3 sell-only
  `unsupported_balance_storage_layout` infrastructure failures before the
  recipient-net accounting rerun became the current baseline.
- Sell proceeds now prefer the strategy recipient's net ETH/WETH balance
  increase before falling back to gross denomination-token pool output. The
  gross-output baseline fixed zero-fill sells but overcounted auto-swap WETH
  paid to third parties, so the old
  `hist-snipe-all-poolonly-15k-weth-only-20260513-134623` result is
  superseded by the recipient-net run.
- `pancakeswap-v2` protocol strings are now parsed as Pancake V2 instead of
  unsupported. The affected Pancake entry buys successfully, then fails every
  sell probe down to 1% with no token-to-pool sell logs through current head.
- V3 Universal Router sell calldata now matches the observed router variant.
  The remaining V3 failed exit is no longer a route gap: it succeeds at the
  observed sell block, but at the strategy max-hold exit block the pool is
  drained and the simulator returns `V3InvalidSwap`.
- Stable-denom entries are excluded from the executable baseline until we add
  quote conversion and ETH-equivalent PnL. This removed the two USDC/USDT
  empty-router buy failures and the one confirmed USDT-denom position from the
  previous run; the current run has only WETH-denominated confirmed positions.
- Source replay rows for this 15k range have `payload.pool.runtime_state`,
  but only with `last_sync_block` / `last_update_block`; none preserve
  `runtime_state.can_buy` or `runtime_state.can_sell`.
- Strategy Lab now prints `Buy Failed Entries` and `Open Failed Exits` tables
  so failed entries and retryable failed exits are visible without ad hoc SQL.

Keep this ledger short and move fixed rows into tests, focused investigation
folders, or commit history.

| Priority | Status | Issue | Evidence | Fix / Next Check |
| --- | --- | --- | --- | --- |
| P0 | confirmed | Failed sells remain open, retryable exposure | The run has 20 failed sell reports but 8 sell-failed positions. BCB2 failed at block `25,074,899` and then sold successfully at `25,074,918`; the 8 still-open failed exits all have zero-value snapshots and remain `sell_failed`. | Keep this lifecycle behavior; next work is policy, not accounting |
| P1 | investigating | V4 observed-flow-only entries polluted strategy eligibility | 6 V4 Universal Router buy failures were selected from observed third-party flow even though same-route buy probes failed at entry block and `block-1` for `0.01`, `0.001`, and `0.0001 ETH`; all six stored observations say top-level `can_buy=true` / `can_sell=true`, but lack `runtime_state.can_buy/can_sell` | Rebuild the source observations from a token-server response that includes runtime trading flags, then rerun the 15k backtest; these should become skips, not failed buys |
| P1 | confirmed | Sell exits are root-cause bucketed | Current failed-position split: 7 V2 `TRANSFER_FROM_FAILED` exits and 1 V3 `V3InvalidSwap`. DF1A/BE74/2834 are address-specific: observed chain sellers simulate successfully with gross proceeds, while the strategy seller fails at every tested size. 11AE sells in 5%/2%/1% chunks. Pancake V2 fails down to 1% and has no token-to-pool sell logs through current head. BCB2 confirms after a later retry. The V3 case is drained zero-liquidity exposure at exit block. | Choose strategy rules for retry cadence, chunked exits, and address-specific/no-observed-sell exposure |
| P1 | confirmed | Fixed entry size fails on thin WETH pools | The 3 V2 `TRANSFER_FAILED` and 2 V3 `TF` WETH buy failures fail at `0.01 ETH` but representative same-route probes succeed at `0.001 ETH` on both `block-1` and the entry block | Decide whether this strategy remains fixed-size, adds adaptive entry sizing, or skips pools that cannot support the target size |
| P2 | investigating | PnL is too concentrated for strategy conclusions | Total PnL is positive, but excluding the top five positions is `-0.219822746621256746 ETH`; excluding the top ten is `-2.606516897707689237 ETH` | Add concentration-aware reporting to the strategy table and use it as a baseline gate before treating `Snipe All v1` as profitable |
