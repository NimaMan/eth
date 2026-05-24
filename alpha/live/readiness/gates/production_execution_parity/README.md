# Production Execution Parity Gate

## Objective

Before the next public deploy, the live-backtest must prove the same execution
decisions that the real runner will face at the production boundary:

- exact deployed V2 vault calldata simulation;
- gas-rank and economic-policy acceptance;
- stale-state deferral vs terminal cancellation;
- receipt-derived final state for real runs;
- deterministic ordering evidence for same-block market and mempool signals.

The current chain-sim live-backtest is useful for strategy logic and accounting,
but it is not enough by itself. It can mark a trade as confirmed from an EVM swap
simulation even when the production runner would defer, cancel, reject, or later
observe a mined revert.

## Current Real-Run Evidence

Real run:

`live-alpha11-univ2-lp30-pool-update-block-hold16-live-real-public-20260523-152925Z`

Comparable live-backtest run:

`live-alpha11-univ2-lp30-pool-update-block-hold-sweep-chain-sim-bankroll555-20260523-151201Z`

| Token | Real outcome | Live-backtest outcome | Gap |
| --- | --- | --- | --- |
| `0xbDe5cCc2E611251e0330fe89ac14Dc25eF3f915F` | `buy_cancelled`: exact live vault tx simulation reverted at block `25158902` | later `sell_confirmed`, `+0.02556 ETH` | Backtest did not model the failed production preflight or the buy-once effect of that failed attempt. |
| `0x807C37b178F39082B171ba7a8ECAedEC17cD1cdD` | `buy_deferred`: selected sim block `25158989`, current block `25158992`, state lag exceeded limit | later `sell_confirmed`, `+0.00035 ETH` | Backtest entered once confirmed state caught up; real runner correctly deferred instead. |
| `0x8515b1BCb8a800b04FB1570fD93F29DEff20A3af` | buy confirmed, priority sell mined with receipt `status=0x0` | no trade; active LP risk blocked entry | Backtest and real runner saw same-block mempool/market ordering differently, and backtest had no receipt-failure model. |
| `0xFa85A15AC9Bd99EA20c90F9e3A69E6dadF724D53` | buy confirmed, max-hold sell cancelled by exact sell pre-submit simulation revert | `sell_confirmed`, `+0.26471 ETH` | Backtest sell path did not use the production priority-sell planner and exact vault min-output simulation as the lifecycle gate. |
| `0x7e0Cdb817D53E1F8ed87AF55FB6E1819Ad97586f` | buy confirmed, LP-removal sell cancelled by `gas_rank_exceeds_value_cap` | `sell_cancelled` for the same gas-policy shadow reason | This is the one category the current shadow gas policy did catch. |
| `0x57c1e10B255EA41cd59882079dEaD3dBa66bdceC` | `buy_cancelled`: exact live vault tx simulation reverted | later `sell_confirmed`, `+0.04600 ETH` | Backtest did not run the exact production buy preflight. |
| `0xB17163f4F991B2A34D2ef2a99eAB0d1165275862` | `buy_cancelled`: exact live vault tx simulation reverted | later `buy_confirmed`, still open with near-zero value | Backtest admitted a position production would not buy. |
| `0xf2aa3441353Ad483fE0Ea1a77Fa754280b3a03A7` | `buy_cancelled`: exact live vault tx simulation reverted | later `sell_confirmed`, `+0.02432 ETH` | Backtest did not model production preflight rejection. |

## Why The Live-Backtester Missed It

The current live-backtest execution adapter confirms from chain simulation:

```text
decision at block N
  -> simulate buy/sell at selected execution block
  -> Confirmed if the chain-sim swap succeeds
  -> Failed or Cancelled only for the chain-sim path's own errors
```

The real runner does more before it can submit:

```text
strategy intent
  -> exact deployed V2 vault calldata simulation
  -> min-output derivation from that exact simulation
  -> simulated gas extraction, no fallback gas estimate
  -> gas-rank and economic policy selection
  -> Kartal/signer policy
  -> public mempool broadcast
  -> mined receipt reconciliation
  -> expected V2 vault fill event extraction
```

The missing parity points are:

1. The backtest buy path can use confirmed chain state after the pending tx
   lands, while the real path may only have stale local state or pending-only
   state at submission time.
2. The backtest buy/sell path is not the exact deployed V2 vault calldata
   planner used by the real runner.
3. The backtest can confirm a sell from local simulation even when the
   production priority-sell planner rejects the exact calldata or min-output.
4. The backtest has no mined receipt status, tx index, nonce, replacement, or
   missing-vault-event evidence.
5. Same-block mempool and market event ordering is not validated against
   signal id, detection tx hash, transaction index, or created-at ordering.
6. A cancelled or deferred production entry can consume strategy state in real
   trading, while the backtest may later enter the same token after state
   catches up.

## Five Similar Errors To Guard Against

| Failure class | Example | Why chain-sim may miss it | Required guard |
| --- | --- | --- | --- |
| Public ordering inversion | Our buy lands before the launcher/trading-enabled tx and reverts. | Chain-sim sees post-block state and assumes the dependency already happened. | Mempool overlay evidence with dependency tx hashes, tail-after ordering basis, and receipt tx index. |
| Successful receipt without vault fill | Receipt status is `1`, but no matching `BoughtV2` or `EmergencySoldV2` event exists for our token. | Chain-sim only sees simulation deltas. | Receipt reconciliation must remain required; backtest replay should model missing-event as unresolved, not confirmed. |
| Nonce or replacement failure | Tx is dropped, replaced, nonce-too-low, or stuck pending. | Backtest has no nonce pool or broadcast lifecycle. | Kartal journal replay and pending-age validation before any deploy increase. |
| Stale gas-rank or fee policy | Gas data is stale, missing, or fee cap rejects the only viable policy. | Chain-sim can still price the swap and mark it profitable. | Production gas-policy shadow must be mandatory for every order intent. |
| State mutation between preflight and mining | Token blacklist/tax/liquidity changes after simulation but before mined inclusion. | Backtest assumes one target execution state. | Receipt replay plus late-state simulation at mined block and tx index where available. |
| Same-block risk ordering mismatch | LP approval or liquidity-removal signal appears in the same block as entry. | Different poll order can flip `entry` vs `blocked_by_active_risk`. | Persist and validate signal id, created-at, pending tx hash, mined tx index, and event-source ordering. |

## Required Backtester Improvements

1. Add a production preflight shadow adapter for live-backtests. For every
   `OrderIntent`, run the same production planner used by real trading and
   persist:
   - exact route protocol and vault address;
   - selected simulation block, required state block, and current block;
   - exact simulation revert flag, gas used, expected output, and min-output;
   - gas-rank candidates, selected profile, and rejection reason;
   - final shadow outcome: `would_submit`, `would_defer`, `would_cancel`, or
     `would_fail`.
2. Treat a live-backtest as not deploy-ready if chain-sim confirms a trade where
   production shadow would defer, cancel, fail, or reject.
3. Make buy-once state explicit: a production `buy_cancelled` or `buy_deferred`
   must either be replayed identically by the live-backtest or the strategy must
   explicitly state that the token can be retried.
4. Add a mempool-overlay fixture suite for `trading_enabled` signals where the
   pair/trading state only exists inside the pending tx. The expected outcome is
   `buy_deferred` unless exact overlay V2 vault evidence is present.
5. Add same-block event-order validation for market events vs mempool signals.
   If the ordering evidence is missing, the validator should mark the run
   `assessment`, not `validated`.
6. Replay the public real-run failure window as a regression fixture. The
   improved live-backtest must predict:
   - exact-simulation buy cancellations for the four reverting candidates;
   - stale-state deferral for `0x807C...`;
   - sell preflight cancellation for `0xFa85...`;
   - gas-policy cancellation for `0x7e0C...`;
   - receipt failure or unresolved receipt risk for `0x8515...`.

## Implemented Baseline Fixes

The first backtest/live-backtest parity fixes are now in place:

- backtest core primes the execution adapter with projected pools from
  `mempool_entry_evidence` before a `trading_enabled` risk event is handled;
- live-backtest primes the same projected pool cache before running the strategy
  on a mempool signal;
- historical observation replay includes `trading_enabled` signals and preserves
  `mempool_entry_evidence`;
- tail-entry buys use a distinct `tail_entry_buy` gas-policy context instead of
  being hidden under normal `entry_buy`;
- tail-entry gas shadow evidence records `tail_after_tx_hash`,
  dependency priority/gas-price metadata when present, and explicitly marks the
  validation mode as `post_mine_n_plus_1` with `exact_overlay_simulation=false`.

This is intentionally not same-block overlay proof. Live backtest keeps the
core backtest execution model: observe/submit at block `N`, then simulate the
fill as the last relevant transaction against post-block `N+1` state. Exact
dependency-tx plus deployed-vault calldata overlay simulation remains a
real-live deployment gate, not a live-backtest validation failure.

## Implemented Validation Checks

These checks are wired into the comprehensive strategy validator:

| Check | Fails when |
| --- | --- |
| `tail_entry_intent_has_exact_vault_buy_evidence` | A tail-entry buy intent lacks successful deployed-vault calldata evidence. |
| `tail_entry_buy_has_ordering_evidence` | A tail-entry gas-policy event lacks the dependency tx hash or dependency fee evidence. |
| `tail_entry_buy_uses_live_backtest_n_plus_1_validation` | A tail-entry event is not explicitly marked as post-mine N+1 chain-sim validation. |

## Remaining Validation Checks

Add these checks after the production preflight shadow adapter exists:

| Check | Fails when |
| --- | --- |
| `production_preflight_present_for_all_order_intents` | Any buy/sell intent lacks production shadow evidence. |
| `chain_sim_does_not_confirm_production_reject` | Chain-sim confirms an order that production shadow would defer, cancel, fail, or reject. |
| `priority_sell_planner_parity` | Chain-sim confirms a sell that the production priority-sell planner rejects. |
| `same_block_signal_ordering_evidence_present` | A same-block market/mempool decision lacks ordering evidence. |
| `entry_retry_policy_explicit` | A deferred/cancelled entry can later be retried without explicit strategy policy. |
| `receipt_replay_regression_passes` | A real-run receipt failure is not reproduced as failed/unresolved in replay. |

## Deploy Readiness Rule

Do not deploy another public real run until the selected live-backtest run has
production preflight shadow evidence for every order intent, no chain-sim
confirmation contradicts production shadow, and the real-run failure window
above passes as a regression fixture.
