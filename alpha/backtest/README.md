# Backtest

Crate: `eth_alpha_backtest`

This crate replays historical market data through the same trading core used by live trading, using **EVM-backed swap simulation** for chain parity.

## Responsibilities

- Replay existing live chain-sim runs from `alpha_trading.strategy_observations`.
- Feed events into `eth_alpha_engine`.
- Run swaps through the EVM via `eth_alpha_engine::execution::ChainSimExecutionAdapter`.
- Produce reports, metrics, and snapshots in Postgres.

## Non-Responsibilities

- No separate strategy API.
- No separate position state machine.
- No live signing or broadcasting.
- No Kartal client, signer, hot-wallet, deployed-vault, or live nonce
  dependency.
- No mutation of canonical live state.
- No external file formats (JSONL, Parquet, etc.).  Events are read directly from Postgres.
- No model-based fill estimation (slippage math, random failure rolls, etc.).  Fills come from actual EVM simulation.

## Core Principle

Backtest should not have special position transition logic.

Use the same shape as live:

```text
StrategyDecision
  -> OrderIntent
  -> ChainSimExecutionAdapter (EVM-backed)
  -> ExecutionReport
  -> Engine position update
```

The Python version had separate `BacktestStrategyEngine` and `LiveStrategyEngine` paths with duplicated state transitions. This crate removes that duplication.

Backtests use the same lifecycle as real live trading:

```text
BuyIntentCreated -> BuySubmitted -> BuyConfirmed
SellIntentCreated -> SellSubmitted -> SellConfirmed
```

The only semantic difference is the confirmation source. Historical backtest
and live backtest do not wait for real tx receipts. They emit `Confirmed` when
the EVM simulation for the target execution block succeeds, and `Failed` when
that simulation reverts or cannot produce the fill. Strategy decisions must
still never directly mark a position as confirmed.

## Historical Inputs

Historical backtests replay stored confirmed-chain observations from
`strategy_observations`. They intentionally ignore stored mempool signal rows so
the historical result is based on chain history only. Live strategy runners are
the place for current mempool signals and pre-confirmation risk exits.

## Block-Level Execution Model

The token pipeline updates market state at block level. A backtest step runs as:

```text
processed block N
  -> token/pool snapshots for block N
  -> eth_alpha_engine handles MarketEvent
  -> strategy decides from the block N snapshot
  -> engine creates OrderIntent
  -> submitted lifecycle report is recorded at block N
  -> ChainSimExecutionAdapter runs the swap against post-block N+1 state by default
  -> confirmed/failed ExecutionReport updates order and position state at N+1
```

That means a buy that simulates successfully at `N+1` becomes `BuyConfirmed` at
`N+1`. A sell that simulates successfully at its target execution block becomes
`SellConfirmed` at that block. No real tx hash, receipt, nonce, or finality is
implied by a backtest confirmation.

Do not treat a strategy decision as if it had been known before every transaction in the same block unless the replay input explicitly provides transaction-level ordering.

## Chain Parity

Backtests use the same EVM simulation path as live chain-sim trading:

- Buy fills run actual swap calldata through `tx_simulator::TxSimulator` at the historical block.
- Sell fills run actual swap calldata through the EVM using the stored raw token amount from the buy report.
- Token taxes, max-transaction limits, honeypots, and other contract behaviour are captured exactly.
- No hidden theoretical fallbacks (e.g. `cost_basis / price`) are applied.

## Usage

### CLI

```bash
cargo run -p eth_alpha_backtest --bin eth_alpha_backtest_trader -- \
  --strategy-impl snipe-all \
  --strategy-name snipe-all \
  --replay-run-id "alpha-trader-1715350000-12345" \
  --buy-amount-wei 10000000000000000 \
  --min-liquidity-eth 0.5 \
  --min-liquidity-usd 1000
```

The backtest reads `databases.alpha.url` and `databases.risk_atlas.url` from
`blockchains/eth/config.toml`, and `RETH_DATADIR` from
`blockchains/eth/config.env` by default. Risk Atlas replay inputs are loaded
from the Risk Atlas database key, while backtest results are written through
the Alpha database key.

Backtest persistence goes through `alpha/store/` and writes the
`alpha_trading` schema: `trader_runs`, `order_intents`, `execution_reports`,
`positions`, `position_snapshots`, `trades`, `trade_events`,
`trade_snapshots`, and result-set tables. Strategy validation reports are saved
by `alpha/lab` in `alpha_trading.strategy_validation_reports`.

### Required arguments

| Flag | Description |
|------|-------------|
| `--replay-run-id` | Existing live chain-sim run to replay from `strategy_observations` |

### Optional arguments

| Flag | Description | Default |
|------|-------------|---------|
| `--strategy-impl` | Strategy implementation to instantiate | `snipe-all` |
| `--strategy-name` | Strategy instance name persisted on orders, positions, reports, and PnL rows | `snipe-all` |
| `--strategy-suite historical-pool-update-hold` | Run hold1/2/3/5/10 pool-update variants in one replay pass | disabled |
| `--from-block` | Start block (inclusive) | first observation |
| `--to-block` | End block (inclusive) | last observation |
| `--skip-primed` | Skip warmup observations | false |
| `--stop-loss-ratio` | Stop-loss trigger ratio | disabled |
| `--take-profit-ratio` | Take-profit trigger ratio | disabled |
| `--max-hold-blocks` | Force exit after N distinct pool-update blocks while the position is open | disabled |

Suite mode still records one `trader_runs` row, but orders, positions,
strategy decisions, and derived PnL stay separated by `strategy_name`.

## Architecture

| Module | Purpose |
|--------|---------|
| `adapter` | `BacktestAdapter` trait for state sharing between runner and engine |
| `config` | `BacktestConfig` |
| `runner` | `run_backtest` drives events through `AlphaEngine` |
| `bin/eth_alpha_backtest_trader` | Historical backtest trader binary; reads observations from Postgres and persists results to Postgres |

## Integration with Asena

Backtest runs write to the same Postgres `alpha_trading` schema as live trading:
- `trader_runs` with `mode = 'backtest'`
- `positions`, `order_intents`, `execution_reports`, `risk_events`

Asena's existing performance endpoints (`/alpha/strategies/<id>/performance`) can query these rows by `run_id` and display backtest results alongside live chain-sim results.

## Broadcast Guard

Backtests must never be able to broadcast. The only execution adapter in this
crate is `ChainSimExecutionAdapter`; do not add Kartal, `TxExecutorAdapter`,
signer env vars, hot-wallet config, or deployed-vault config to the backtest
binary. If a historical experiment needs to exercise live tx-prep code, run it
as a lab/calibration artifact that writes requests or uses Kartal dry-run, not
as `eth_alpha_backtest_trader`.
