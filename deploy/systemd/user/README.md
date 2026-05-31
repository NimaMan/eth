# User Services

These user-level units keep the alpha execution processes separated by
broadcast capability.

Install or refresh the symlinks with:

```bash
/home/nima/code/crypto/blockchains/eth/deploy/node/scripts/install-user-services.sh
```

Chain-server is intentionally not a user unit. It is a system service installed
from `deploy/systemd/eth-chain-server.service` and managed with
`sudo systemctl`. The user alpha services perform runtime readiness checks
against the chain-server API instead of starting a second chain-server process.

| Unit | Purpose | Broadcast capability |
| --- | --- | --- |
| `eth-alpha-live-backtest.service` | Live no-capital alpha backtest runner using `eth_alpha_live_backtest_trader`. | None; never contacts the ETH tx executor. |
| `eth-alpha-live-real-trading.service` | Live real-executor runner using `eth_alpha_live_trader` with strategy-owned bankroll entries. | ETH tx executor public mempool only when the explicit hold16 deploy guard passes. |
| `eth-alpha-live-backtest.target` | Mempool signal detector and live chain-sim alpha backtest. Chain-server must already be running as the system unit. | None. |
| `eth-alpha-live-real-trading.target` | Mempool signal detector and live real-executor alpha runner. Chain-server, `eth-tx-executor`, and `eth-tx-signer` must already be running as system units. | Same as the real-trading service. |

Backtests are intentionally not systemd services here. They are on-demand
historical jobs from `alpha/backtest` and must stay simulator-only.

Mutable strategy and execution arguments live in `eth-alpha-live-backtest.env`
and `eth-alpha-live-real-trading.env`; the service files stay stable across
restarts.
For the first public real-executor run, `eth-alpha-live-real-trading.env` targets
`alpha11-univ2-lp30-pool-update-block-hold16`; the strategy spec sets the
`0.005 ETH` buy size and `0.555 ETH` initial bankroll. Buys consume that
bankroll; confirmed sells replenish it; profits can be redeployed. The service
does not pass entry-count or bankroll overrides on the command line.

The user units do not set `ALPHA_RUN_ID` and do not pass `--run-id`. Live
backtest and real-trading run ids are owned by the trader binaries. Each process
creates a semantic run id from mode and strategy, stores the active session under
`ALPHA_TRADER_SESSION_DIR` or the default `.state/alpha_trader_sessions`, and
reuses that id only for crash auto-restarts. A clean stop or restart marks the
session stopped, so the next service start becomes a new live run.

The ETH tx executor still enforces infrastructure safety rails: signer address, target
address, selector allowlist, per-transaction value, gas, fee, simulation
freshness, and transaction-cost caps. The daily spend cap is intentionally set
high enough not to be the active strategy limiter during the initial hold16
deployment; entry capacity is governed by the strategy bankroll.

The real-executor service is separate from the chain-sim runner so the process
that can talk to the ETH tx executor is not the same process used for
historical replay or no-capital strategy validation.
