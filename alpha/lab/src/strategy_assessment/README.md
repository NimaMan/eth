# Strategy Assessment

This module answers strategy-quality questions after a result set has passed
strategy validation. Assessment is not validation: it does not decide whether
the lifecycle, accounting, event sequence, or snapshots are correct. It assumes
the backend persisted rows are usable and asks whether the strategy conclusion
is robust enough to trust.

## Entry Points

- `runner.rs`: builds the assessment report and owns the question thresholds.
- `db.rs`: computes aggregate metrics from `alpha_trading.trades`.
- `report.rs`: serializable report types and CLI rendering.

The public API is `assess_strategy(pool, StrategyAssessmentOptions)`.

## Questions And Quantification

### Does the positive PnL conclusion survive removing the biggest winners?

Quantification:

- Rank positive-PnL trades by `total_pnl_eth`.
- Compute `top1`, `top5`, and `top10` winner PnL.
- Compute `topN_share_of_net_pnl = topN_winner_pnl / total_pnl_eth`.
- Compute `pnl_ex_topN = total_pnl_eth - topN_winner_pnl`.

Answer:

- `fragile` when the result is positive but `top5_share_of_net_pnl > 100%`
  or `pnl_ex_top5 < 0`.
- `review` when `top5_share_of_net_pnl > 60%` or `pnl_ex_top1 < 0`.
- `stable` when the result remains positive after removing the top five
  winners.

### Is gross profit large enough to absorb gross losses?

Quantification:

- `gross_profit_eth = sum(max(total_pnl_eth, 0))`.
- `gross_loss_eth = abs(sum(min(total_pnl_eth, 0)))`.
- `profit_factor = gross_profit_eth / gross_loss_eth`.

Answer:

- `fragile` for positive net PnL with `profit_factor < 1.2`.
- `review` for positive net PnL with `profit_factor < 1.5`, or any
  non-positive net result.
- `stable` above those thresholds.

### Are losses concentrated in a small tail we can explain or avoid?

Quantification:

- Rank negative-PnL trades by absolute loss.
- Compute `top5_loser_share = top5_loser_loss_eth / gross_loss_eth`.

Answer:

- `review` when at least five losing trades exist and the top five explain
  `>= 70%` of gross loss.
- `stable` otherwise.

### Does unresolved exposure materially affect the strategy conclusion?

Quantification:

- Exposure states are `buy_confirmed`, `sell_intent_created`,
  `sell_submitted`, `sell_failed`, and `sell_cancelled`.
- Compute exposure entry cost, current value, unrealized PnL, and drawdown.
- Compute `exposure_to_capital = exposure_entry_cost_eth / total_entry_cost_eth`.
- Compute `exposure_unrealized_to_net_pnl =
  exposure_unrealized_pnl_eth / total_pnl_eth`.

Answer:

- `fragile` when exposure exceeds `35%` of deployed capital or unrealized
  exposure exceeds `50%` of net PnL.
- `review` when exposure exceeds `20%` of deployed capital or unrealized
  exposure exceeds `25%` of net PnL.
- `stable` below those thresholds.

### Are failed exits leaving unresolved exposure?

Quantification:

- Count exposure trades in `sell_failed` or `sell_cancelled`.
- Compute `failed_exit_rate = failed_exit_exposure_trades / exposure_trades`.

Answer:

- `fragile` when failed exits exceed `10%` of exposure trades.
- `review` when any failed-exit exposure exists.
- `stable` when no failed-exit exposure remains.

## Running

```bash
cargo run -p eth_alpha_lab -- strategy-assessment \
  --result-set <result_set_id> \
  --strategy <strategy_name>
```

JSON output:

```bash
cargo run -p eth_alpha_lab -- strategy-assessment \
  --result-set <result_set_id> \
  --strategy <strategy_name> \
  --json
```
