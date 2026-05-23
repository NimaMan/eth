# Alpha11 Hold16 Deploy Readiness

This file tracks the readiness criteria for promoting the selected Alpha11
strategy to public real capital.

## Strategy

`alpha11-univ2-lp30-pool-update-block-hold16`

## Scope

| Setting | Value |
| --- | --- |
| Strategy implementation | `alpha11` |
| Protocol scope | Uniswap V2 only |
| Entry signal | Pool-update-block LP30 strategy entry |
| Buy size | `0.01 ETH` |
| Starting bankroll | `0.555 ETH` |
| Max hold | `16` active pool-update blocks |
| LP approval entry gate | Block entry above `30%` LP approval |
| LP approval exit | Enabled; defer only for trading-enabled age `<= 2` blocks |
| Execution route | Deployed Uniswap V2 trading vault |
| Gas policy source | `eth_chain_server_gas_rank` |

## Current Live-Backtest

The current service-backed live-backtest run is:

`live-alpha11-univ2-lp30-pool-update-block-hold-sweep-chain-sim-bankroll555-20260523-134354Z`

It is a clean chain-sim run using the current service env and current
`config.env` gas policy:

| Policy | Current value |
| --- | --- |
| Entry buy ladder | `p85 -> p75 -> p50 -> normal` |
| Normal exit ladder | `p85 -> p75 -> p50 -> normal` |
| LP approval exit ladder | `p90 -> p75 -> p50 -> normal` |
| Mempool race exit ladder | `p95 -> p90 -> p75 -> p50 -> normal` |
| Simulated gas buffer | `2500 bps` |
| Max fee hard stop | `5 gwei` |
| Max priority fee | `3.5 gwei` |
| Entry gas fee cap | `0.0012 ETH` |
| Exit gas fee cap | `0.002 ETH` |

The previous long-running hold16 chain-sim runs remain useful as historical
evidence, but they used the earlier `0.225 ETH` bankroll and/or pre-restart gas
policy state. Do not use those process lifetimes as final promotion evidence for
the `0.555 ETH` bankroll target.

## Current Runtime Checks

As of `2026-05-23 15:44 CEST`, the service-backed live-backtest is running on
`eth-alpha-live-backtest.service` with the run id above and the current release
binary.

Runtime evidence observed after restart:

| Check | Evidence |
| --- | --- |
| Live-backtest process | One `eth_alpha_live_backtest_trader` process with the current run id |
| Real live trader | No `eth_alpha_live_trader` process running |
| Live-backtest status | `live`, no last error, fresh `0.555 ETH` run has `0` positions so far |
| Hold16 validation | Previous `0.225 ETH` validation passed; rerun validation after the fresh `0.555 ETH` run has enough hold16 lifecycle evidence |
| Kartal broadcast mode | `dry_run` |
| Kartal signer | Available through `unix_socket` |
| Kartal caps | max value `0.01 ETH`, max gas `300000`, max fee `5 gwei`, max tx cost `0.0125 ETH`, daily cap `0.06 ETH` |
| Cap/signing preflight | `alpha11-hold16-cap-signing-preflight-20260523T125653Z` returned `dry_run` and tx hash `0x31b17a0d96a93d9a6209a665d67052a0b5d1ed79a3206bf6a40bb0b031d11829` |
| Max-fee enforcement | `alpha11-hold16-maxfee-5gwei-accepted-20260523T131107Z` returned `dry_run`; `alpha11-hold16-maxfee-6gwei-rejected-20260523T131107Z` was rejected because `max_fee_per_gas` exceeded the `5 gwei` policy cap |
| Post-signer-restart preflight | `alpha11-hold16-post-signer-restart-preflight-20260523T133813Z` returned `dry_run` after remounting the `/run/kartal` socket into the Kartal container |
| Dry-run spend accounting | `spent_today=0 ETH`, `remaining_daily=0.06 ETH` after the signing preflight |

## Required Before Public Hold16

- Gate 1 strategy scope and defaults accepted for `hold16`.
- Gate 2 live-backtest spine parity accepted against the current chain-sim run.
- Gate 3 chain-facing assumptions reviewed and either proven or explicitly
  bounded.
- Production gas-rank readiness passes:
  `../../gates/production_gas_rank/README.md`.
- Pre-live mined validation is accepted from the prior hold3 public run, or a
  new one-position validation run is executed and archived.
- Real hold16 can be launched by strategy name with no CLI parameter overrides.
- Kartal, signer, Alpha config, and Asena expose matching vault, signer, gas,
  value, transaction-cost, and daily-spend caps.
- Receipt worker alerting and operator review are in place.
- Initial bankroll remains capped at `0.555 ETH`.

## Current Blockers

- The fresh `0.555 ETH` live-backtest has no completed hold16 simulated
  lifecycles yet. Promotion must wait for same-config lifecycle evidence and a
  non-stale validation report.
- Public-mempool real trading is still guarded to the explicit hold3
  validation strategy. Promoting hold16 requires an intentional code/config
  change after the gates pass.
- The old hold15 readiness file is not the promotion target; hold16 owns this
  readiness record.

## Promotion Rule

Hold16 cannot use public real capital until the current `0.555 ETH` live-backtest
has a non-stale passing validation report and the production gas-rank/pre-live
mined validation gates are accepted.
