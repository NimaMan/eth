# User Services

These user-level units keep the alpha execution processes separated by
broadcast capability.

| Unit | Purpose | Broadcast capability |
| --- | --- | --- |
| `eth-alpha-trader.service` | Live no-capital alpha runner using `--mode chain-sim`. | None; never contacts Kartal. |
| `eth-alpha-real-trader.service` | Live real-executor runner using `--mode kartal-real --disable-entry`. | Kartal dry-run only; the binary refuses non-dry-run Kartal status. |
| `eth-live-pipeline.target` | Chain server, mempool signal detector, and chain-sim alpha runner. | None. |
| `eth-real-trading.target` | Chain server, mempool signal detector, and real-executor alpha runner. | Dry-run until the code and Kartal policy are deliberately changed. |

Backtests are intentionally not systemd services here. They are on-demand
historical jobs from `alpha/backtest` and must stay simulator-only.

The real-executor service is separate from the chain-sim runner so the process
that can talk to Kartal is not the same process used for historical replay or
no-capital strategy validation.
