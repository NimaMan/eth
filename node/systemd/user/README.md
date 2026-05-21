# User Services

These user-level units keep the alpha execution processes separated by
broadcast capability.

| Unit | Purpose | Broadcast capability |
| --- | --- | --- |
| `eth-alpha-backtester.service` | Live no-capital alpha runner using `eth_alpha_live_backtest_trader`. | None; never contacts Kartal. |
| `eth-alpha-real-trader.service` | Live real-executor runner using `eth_alpha_live_trader` with capped entries. | Kartal dry-run only; the binary refuses non-dry-run Kartal status. |
| `eth-live-pipeline.target` | Chain server, mempool signal detector, and chain-sim alpha backtester. | None. |
| `eth-real-trading.target` | Chain server, mempool signal detector, and real-executor alpha runner. | Dry-run until the code and Kartal policy are deliberately changed. |

Backtests are intentionally not systemd services here. They are on-demand
historical jobs from `alpha/backtest` and must stay simulator-only.

Mutable run arguments live in `eth-alpha-backtester.env` and
`eth-alpha-real-trader.env`; the service files stay stable across restarts.
For the first real-executor validation run, `eth-alpha-real-trader.env` targets
`alpha11-live-univ2-lp30-pool-update-block-hold15`, sets `ALPHA_BUY_WEI` to
`0.01 ETH`, and uses the Alpha11 spec default bankroll of `0.225 ETH`. Buys
consume that bankroll; confirmed sells replenish it; profits can be redeployed.

The real-executor service is separate from the chain-sim runner so the process
that can talk to Kartal is not the same process used for historical replay or
no-capital strategy validation.
