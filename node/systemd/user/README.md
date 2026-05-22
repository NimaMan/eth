# User Services

These user-level units keep the alpha execution processes separated by
broadcast capability.

| Unit | Purpose | Broadcast capability |
| --- | --- | --- |
| `eth-alpha-live-backtest.service` | Live no-capital alpha backtest runner using `eth_alpha_live_backtest_trader`. | None; never contacts Kartal. |
| `eth-alpha-live-real-trading.service` | Live real-executor runner using `eth_alpha_live_trader` with capped entries. | Kartal dry-run only; the binary refuses non-dry-run Kartal status. |
| `eth-alpha-live-backtest.target` | Chain server, mempool signal detector, and live chain-sim alpha backtest. | None. |
| `eth-alpha-live-real-trading.target` | Chain server, mempool signal detector, and live real-executor alpha runner. | Dry-run until the code and Kartal policy are deliberately changed. |

Backtests are intentionally not systemd services here. They are on-demand
historical jobs from `alpha/backtest` and must stay simulator-only.

Mutable run arguments live in `eth-alpha-live-backtest.env` and
`eth-alpha-live-real-trading.env`; the service files stay stable across restarts.
For the first real-executor validation run, `eth-alpha-live-real-trading.env` targets
`alpha11-univ2-lp30-pool-update-block-hold15`; the strategy spec sets
the `0.01 ETH` buy size and default bankroll of `0.225 ETH`. Buys
consume that bankroll; confirmed sells replenish it; profits can be redeployed.

The real-executor service is separate from the chain-sim runner so the process
that can talk to Kartal is not the same process used for historical replay or
no-capital strategy validation.
