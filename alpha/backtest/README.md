# Backtest

Crate: `eth_alpha_backtest`

This crate replays historical market data through the same trading core used by live trading.

## Responsibilities

- Replay existing live/paper runs from `alpha_trading.strategy_observations`.
- Feed events into `eth_alpha_engine`.
- Provide a simulated execution adapter.
- Define fill models, slippage models, gas models, and latency assumptions.
- Produce reports, metrics, and snapshots in Postgres.

## Non-Responsibilities

- No separate strategy API.
- No separate position state machine.
- No live signing or broadcasting.
- No mutation of canonical live Redis state.
- No external file formats (JSONL, Parquet, etc.).  Events are read directly from Postgres.

## Core Principle

Backtest should not have special position transition logic.

Use the same shape as live:

```text
StrategyDecision
  -> OrderIntent
  -> SimulatedExecutionAdapter
  -> ExecutionReport
  -> Engine position update
```

The Python version had separate `BacktestStrategyEngine` and `LiveStrategyEngine` paths with duplicated state transitions. This crate removes that duplication.

## Block-Level Execution Model

The token pipeline updates market state at block level. A backtest step should therefore be modeled as:

```text
processed block N
  -> token/pool snapshots for block N
  -> eth_alpha_engine handles MarketEvent
  -> strategy decides from the block N snapshot
  -> engine creates OrderIntent
  -> SimulatedExecutionAdapter applies the configured block-level fill model
  -> ExecutionReport updates order and position state
```

Do not treat a strategy decision as if it had been known before every transaction in the same block unless the replay input explicitly provides transaction-level ordering and the configured latency model allows it.

## Worst-Case Fill Rule

Default backtests should be pessimistic. When a strategy submits a simulated transaction after observing a block-level token update, fill it at the worst price that could plausibly apply within the configured block-level fill window:

- Buy fills use the highest effective price or lowest token output available to the model.
- Sell fills use the lowest effective price or lowest received base amount available to the model.
- If only one block snapshot is available, use that snapshot and label the result as snapshot-based, not exact intra-block execution.
- If transaction-level path data is available, select the adverse executable point allowed by the latency and ordering assumptions.
- Failed buys, failed sells, gas costs, slippage limits, taxes, and liquidity exhaustion must be explicit assumptions, not hidden defaults.

This is intentionally stricter than the old Python backtest flow, which often moved `SUBMIT_*` to `CONFIRM_*` on a later token update. Rust backtests should always route through `ExecutionReport`, even when the report is synthetic.

## Simulation Assumptions

Make assumptions explicit and configurable:

- fill price source
- slippage model
- gas cost model
- confirmation latency
- failed transaction behavior
- pool liquidity threshold
- scam/rug handling
- block-level worst-case price rule
- whether fills are block-snapshot based or transaction-order based

Backtest reports should include the assumptions used for a run.

## Usage

### CLI

```bash
cargo run -p eth_alpha_backtest --bin eth_alpha_backtest -- \
  --database-url "$ALPHA_DATABASE_URL" \
  --replay-run-id "alpha-trader-1715350000-12345" \
  --buy-amount-wei 10000000000000000 \
  --min-liquidity-eth 0.5 \
  --min-liquidity-usd 1000 \
  --slippage-bps 100 \
  --gas-cost-wei 150000 \
  --failure-rate-bps 0
```

### Required arguments

| Flag | Description |
|------|-------------|
| `--database-url` | Postgres connection string |
| `--replay-run-id` | Existing live/paper run to replay from `strategy_observations` |

### Simulation flags

| Flag | Description | Default |
|------|-------------|---------|
| `--slippage-bps` | Additional slippage on sell fills | 100 |
| `--gas-cost-wei` | Gas charged per transaction | 150_000 |
| `--failure-rate-bps` | Random failure probability | 0 |
| `--allow-scam` | Allow trades on scam pools | false |
| `--ignore-liquidity` | Skip liquidity checks | false |
| `--best-case-fill` | Disable worst-case fill reduction | false |

## Architecture

| Module | Purpose |
|--------|---------|
| `config` | `BacktestConfig`, `SimulationConfig` |
| `execution` | `SimulatedExecutionAdapter` implementing `EngineExecutionAdapter` |
| `runner` | `BacktestRunner` that drives events through `AlphaEngine` |
| `bin/eth_alpha_backtest` | CLI binary — reads observations from Postgres, persists results to Postgres |

## Integration with Asena

Backtest runs write to the same Postgres `alpha_trading` schema as live trading:
- `trader_runs` with `mode = 'backtest'`
- `positions`, `order_intents`, `execution_reports`, `risk_events`

Asena's existing performance endpoints (`/alpha/strategies/<id>/performance`) can query these rows by `run_id` and display backtest results alongside live paper-trading results.
