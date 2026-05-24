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

`live-alpha11-univ2-lp30-pool-update-block-hold-sweep-chain-sim-bankroll555-20260523-151201Z`

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

As of `2026-05-23 17:36 CEST`, the service-backed live-backtest is running on
`eth-alpha-live-backtest.service` with the run id above and the current release
binary.

Runtime evidence observed after restart:

| Check | Evidence |
| --- | --- |
| Live-backtest process | One `eth_alpha_live_backtest_trader` process with the current run id |
| Real live trader | `eth_alpha_live_trader` running `alpha11-univ2-lp30-pool-update-block-hold16-live-real-public-20260523-152925Z` |
| Live-backtest status | `live`, no last error |
| Hold16 validation | Current `0.555 ETH` live-backtest validation passes `50 / 50` checks with `5` trades, `4` closed, `1` open |
| Kartal broadcast mode | `public_mempool` |
| Kartal signer | Available through `unix_socket` |
| Kartal caps | max value `0.01 ETH`, max gas `300000`, max fee `5 gwei`, max tx cost `0.0125 ETH`; Kartal daily cap disabled |
| Cap/signing preflight | `alpha11-hold16-cap-signing-preflight-20260523T125653Z` returned `dry_run` and tx hash `0x31b17a0d96a93d9a6209a665d67052a0b5d1ed79a3206bf6a40bb0b031d11829` |
| Max-fee enforcement | `alpha11-hold16-maxfee-5gwei-accepted-20260523T131107Z` returned `dry_run`; `alpha11-hold16-maxfee-6gwei-rejected-20260523T131107Z` was rejected because `max_fee_per_gas` exceeded the `5 gwei` policy cap |
| Post-signer-restart preflight | `alpha11-hold16-post-signer-restart-preflight-20260523T133813Z` returned `dry_run` after remounting the `/run/kartal` socket into the Kartal container |
| Public spend accounting | Kartal daily spend budget disabled; Alpha's `0.555 ETH` initial bankroll is the active budget limiter |
| First real candidate | Pre-submit exact vault calldata simulation reverted at block `25158902`; no tx hash, no Kartal spend, no gas |

## Public Hold16 State

Hold16 has been promoted to the guarded public-mempool runner. The active
strategy limiter is the strategy-owned `0.555 ETH` initial bankroll. Kartal
continues to enforce signer, target, selector, value, gas, fee, simulation
freshness, and per-transaction cost safety rails.

## Current Blockers

- No capital has been deployed yet in the public run. The first candidate was
  rejected by pre-submit exact calldata simulation before signing.
- Continue monitoring the first successful buy, mined receipt, LP-approval
  response, hold16/max-hold exit, sell receipt, gas spend, and PnL rollup.

## Promotion Rule

Do not relax the hold16 guard, signer/target/selector allowlists, value cap,
gas cap, fee caps, simulation freshness, or per-transaction cost cap until at
least one full public buy/sell lifecycle is mined and reconciled.
