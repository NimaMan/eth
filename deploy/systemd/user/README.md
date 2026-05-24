# User Services

These user-level units keep the alpha execution processes separated by
broadcast capability.

Install or refresh the symlinks with `../../node/scripts/install-user-services.sh`.

| Unit | Purpose | Broadcast capability |
| --- | --- | --- |
| `eth-alpha-live-backtest.service` | Live no-capital alpha backtest runner using `eth_alpha_live_backtest_trader`. | None; never contacts Kartal. |
| `eth-alpha-live-real-trading.service` | Live real-executor runner using `eth_alpha_live_trader` with strategy-owned bankroll entries. | Kartal public mempool only when the explicit hold16 deploy guard passes. |
| `eth-alpha-live-backtest.target` | Chain server, mempool signal detector, and live chain-sim alpha backtest. | None. |
| `eth-alpha-live-real-trading.target` | Chain server, mempool signal detector, and live real-executor alpha runner. | Same as the real-trading service. |

Backtests are intentionally not systemd services here. They are on-demand
historical jobs from `alpha/backtest` and must stay simulator-only.

Mutable run arguments live in `eth-alpha-live-backtest.env` and
`eth-alpha-live-real-trading.env`; the service files stay stable across restarts.
For the first public real-executor run, `eth-alpha-live-real-trading.env` targets
`alpha11-univ2-lp30-pool-update-block-hold16`; the strategy spec sets the
`0.01 ETH` buy size and `0.555 ETH` initial bankroll. Buys consume that
bankroll; confirmed sells replenish it; profits can be redeployed. The service
does not pass entry-count or bankroll overrides on the command line.

Kartal still enforces infrastructure safety rails: signer address, target
address, selector allowlist, per-transaction value, gas, fee, simulation
freshness, and transaction-cost caps. The daily spend cap is intentionally set
high enough not to be the active strategy limiter during the initial hold16
deployment; entry capacity is governed by the strategy bankroll.

The real-executor service is separate from the chain-sim runner so the process
that can talk to Kartal is not the same process used for historical replay or
no-capital strategy validation.
