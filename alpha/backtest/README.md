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
- No mutation of canonical live Redis state.
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

## Block-Level Execution Model

The token pipeline updates market state at block level. A backtest step runs as:

```text
processed block N
  -> token/pool snapshots for block N
  -> eth_alpha_engine handles MarketEvent
  -> strategy decides from the block N snapshot
  -> engine creates OrderIntent
  -> ChainSimExecutionAdapter runs the swap against block N state via EVM
  -> ExecutionReport updates order and position state
```

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
cargo run -p eth_alpha_backtest --bin eth_alpha_backtest -- \
  --replay-run-id "alpha-trader-1715350000-12345" \
  --buy-amount-wei 10000000000000000 \
  --min-liquidity-eth 0.5 \
  --min-liquidity-usd 1000
```

The backtest reads `ALPHA_DATABASE_URL` and `RETH_DATADIR` from
`blockchains/eth/config.env` by default.

### Required arguments

| Flag | Description |
|------|-------------|
| `--database-url` | Postgres connection string |
| `--replay-run-id` | Existing live chain-sim run to replay from `strategy_observations` |

### Optional arguments

| Flag | Description | Default |
|------|-------------|---------|
| `--reth-datadir` | Path to synced Reth database | `/home/nima/storage/samsung8tb/ethereum/reth` |
| `--from-block` | Start block (inclusive) | first observation |
| `--to-block` | End block (inclusive) | last observation |
| `--skip-primed` | Skip warmup observations | false |
| `--include-mempool-signals` | Replay stored mempool risk signals | false |
| `--stop-loss-ratio` | Stop-loss trigger ratio | disabled |
| `--take-profit-ratio` | Take-profit trigger ratio | disabled |
| `--max-hold-blocks` | Force exit after N blocks | disabled |

## Architecture

| Module | Purpose |
|--------|---------|
| `adapter` | `BacktestAdapter` trait for state sharing between runner and engine |
| `config` | `BacktestConfig` |
| `runner` | `run_backtest` drives events through `AlphaEngine` |
| `bin/eth_alpha_backtest` | CLI binary — reads observations from Postgres, persists results to Postgres |

## Integration with Asena

Backtest runs write to the same Postgres `alpha_trading` schema as live trading:
- `trader_runs` with `mode = 'backtest'`
- `positions`, `order_intents`, `execution_reports`, `risk_events`

Asena's existing performance endpoints (`/alpha/strategies/<id>/performance`) can query these rows by `run_id` and display backtest results alongside live chain-sim results.
