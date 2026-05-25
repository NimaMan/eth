# Backtest Validity

Position and strategy-result investigations that validate whether PnL,
accounting, lifecycle state, and exit semantics match the declared backtest
execution model.

This folder belongs to Alpha Lab, not Risk Atlas. Risk Atlas supplies token,
pool, chain, and risk facts; Alpha Lab owns validation of strategy positions,
strategy PnL, policy semantics, and backtest accounting.

## Cases

| Status | Case | Current Disposition |
| --- | --- | --- |
| fixed | [sell_proceeds_recipient_net_25065694](cases/sell_proceeds_recipient_net_25065694/) | Sell proceeds now prefer strategy-recipient net ETH/WETH before gross pool output. |
| explained | [top5_winner_liquidity_after_exit_25074419](cases/top5_winner_liquidity_after_exit_25074419/) | Winners sold before later collapse; realized PnL is valid for max-hold semantics. |
| explained | [dbb_top_winner_scam_after_exit_25077629](cases/dbb_top_winner_scam_after_exit_25077629/) | DBB exited before scam observation; PnL valid under non-mempool historical semantics. |
| explained | [worst_loser_position_checks_25073543](cases/worst_loser_position_checks_25073543/) | Representative losses match dead-pool and full-size failed-exit behavior; chunked exit remains a separate strategy-policy question. |

Resolved implementation details should be turned into tests or validation checks
when possible. Keep cases here only while they explain strategy-result behavior
or define a recurring position-review pattern.
