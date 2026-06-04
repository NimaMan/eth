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
- No ETH tx executor client, signer, hot-wallet, deployed-vault, or live nonce
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

## Assumptions and limitations

A backtest result is conditioned on these simplifying assumptions. They are the only
intended differences from real live trading — read every result in their light.

1. **No receipt wait — EVM-simulation confirmation.** Orders emit `Confirmed` when the
   EVM swap simulation for the target block succeeds and `Failed` when it reverts. No real
   tx hash, receipt, nonce, gas auction, reorg, or finality is modeled.
2. **Execution delay (`--execution-delay-blocks`, default 1).** A signal observed at
   block `N` is submitted at `N` and filled against post-block `N + delay` state. So a
   decision made at `N` cannot exploit intra-block ordering at `N`, and an exit submitted
   at `N` races anything that mines within the delay window.
3. **Mempool liquidity-removal exit is live-backtest-only and cannot out-run the removal
   under the default delay.** Historical backtests replay confirmed-chain observations
   only and ignore stored mempool rows; the live-backtest (chain-sim) feeds mempool
   signals. But a mempool removal observed at `N` is acted on with the `N+1` fill delay, so
   the resulting exit generally cannot beat the removal it signals. Under the default delay
   the mempool liquidity-removal exit therefore **verifies the signal path is wired and
   triggers terminalization — it does not model a realizable pre-drain escape.** A
   realizable pre-drain exit would require modeling execution at least as fast as the
   removal (same-block / ordering-aware).
4. **Liquidity-removal exit is fundamental (always-on).** Every strategy exits on a
   liquidity-removal risk — mined `LiquidityRemoval` or `MempoolLiquidityRemoval` — for any
   pool it holds, regardless of config. There is **no** strategy/spec flag for this: the
   former no-op `exit_liquidity_removal` / `exit_on_liquidity_removal` spec/config fields
   have been removed, and the persisted run config JSON no longer carries the key at all.
5. **Mined drain → zero-close (config-independent).** A mined value-destroying drain
   (`LiquidityRemoval` / `ScamConfirmed`) marks every open position on the pool `drained`
   and terminalizes it to `closed_zero_valuation` with no successful sell required. A sell
   already submitted (and simulated against pre-drain state) is **force-failed** at report
   application, so a confiscated balance never realizes synthetic proceeds; the realized
   loss is the full entry cost plus gas.
6. **Within-block ordering: mined drains are applied first.** Within a block, mined
   value-destroying risks are processed before market valuation and execution, so a
   position on a drained pool is marked `drained` before it can be valued positively or
   sold. Mempool projections keep normal order. Otherwise do not assume a strategy decision
   was known before every tx in the same block unless the replay provides transaction-level
   ordering.
7. **Open-position valuation simulates a sell of the stored entry token amount.**
   Mark-to-market values an open position by simulating a sell of the buy report's raw
   token amount against current pool state (real EVM, real taxes/honeypots/limits; no
   `cost_basis / price` fallback). A `drained` position is valued at zero rather than by
   simulating a sell of a confiscated balance.
8. **Transfer-capture completeness.** Net-flow accounting upstream of the backtest assumes
   every transfer is captured from block traces (see `tx_processor` data_models invariant);
   the range/accounting path traces every block by default.

## Prerequisites

The backtester (`eth_alpha_backtest_trader`) fails fast with a single, named
error if any of the following is missing, so a fresh agent gets an actionable
message up front instead of an opaque mid-pipeline failure. Run
`--list-strategy-suites` first to discover valid suites with zero infrastructure.

1. **Chain-server up and synced (sim state source).** Buy/sell fills run real
   swap calldata through the EVM. The chain-server's `live-tx-simulator`
   (`/api/v1/eth/live-tx-simulator/...`) is what provides exact-block simulation
   state for live chain-sim runs; the backtester's EVM simulation reads the same
   synced reth datadir directly via `tx_simulator::TxSimulator`. If reth is not
   synced past the replayed blocks, fills cannot be produced. Bring the
   chain-server / reth node up and let it sync past `--to-block` before running.
2. **`RETH_DATADIR` (from `blockchains/eth/config.env`).** Must be set and point
   at a readable reth datadir directory. Pre-flight checks the directory exists
   and is readable.
   ```
   RETH_DATADIR=/path/to/ethereum/reth
   ```
3. **`config.toml` database URLs (from `blockchains/eth/config.toml`).** Each
   required connection is probed during pre-flight:
   - `databases.alpha.url` — always required; backtest results are written here,
     and non-`risk-atlas-` replay inputs are read from
     `alpha_trading.strategy_observations` here.
   - `databases.risk_atlas.url` — required when `--replay-run-id` is
     `risk-atlas-` prefixed; replay inputs are read from
     `risk_atlas_observations` here.
   - `databases.token_state.url` — required only when `--token-state-scope` is
     given.
   ```toml
   [databases.alpha]
   url = "postgresql://user:pass@host:5432/db"
   [databases.risk_atlas]
   url = "postgresql://user:pass@host:5432/db"
   [databases.token_state]
   url = "postgresql://user:pass@host:5432/db"
   ```
4. **`--replay-run-id` must exist.** Pre-flight verifies the run id has at least
   one observation row in the target table (`risk_atlas_observations` for
   `risk-atlas-` ids, else `alpha_trading.strategy_observations`) and fails with
   a message naming the table and database key if not.
5. **`--token-state-scope` (optional overlay).** A `token_state.scope_id` whose
   mined terminal/custody pool events are overlaid on top of the replayed
   observations — drain / confiscation evidence used by the mined-drain
   zero-close path (see Assumptions #5). Example value:
   ```
   --token-state-scope "historical:<token-pnl-scope>"
   ```
   It overlays terminal pool events (e.g. liquidity removal / scam confirmation)
   into the event stream so positions on a drained pool terminalize to
   `closed_zero_valuation` exactly as live chain history would have forced.

### Discovery

```bash
# Print every valid --strategy-suite value and exit (no DB / reth needed):
cargo run -p eth_alpha_backtest --bin eth_alpha_backtest_trader -- --list-strategy-suites
```

## Usage

### CLI

Current runs use a named strategy suite (the deployable `snipe-all` identity was removed;
single-strategy runs still accept `--strategy-impl` / `--strategy-name`):

```bash
cargo run -p eth_alpha_backtest --bin eth_alpha_backtest_trader -- \
  --run-id my-backtest-25202408-25202412 \
  --replay-run-id "risk-atlas-<run-id>" \
  --strategy-suite alpha-11-risk-atlas \
  --token-state-scope "historical:<token-pnl-scope>" \
  --from-block 25202408 --to-block 25202412 \
  --buy-amount-wei 5000000000000000 \
  --min-liquidity-eth 0.5 --min-liquidity-usd 1000 \
  --execution-delay-blocks 1
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
| `--strategy-suite` | Named strategy suite to run in one replay pass (e.g. `alpha-11-risk-atlas`, `historical-pool-update-hold`) | disabled |
| `--token-state-scope` | `token_state.scope_id` whose mined terminal/custody pool events are overlaid (drain/confiscation evidence) | none |
| `--list-strategy-suites` | Print every valid `--strategy-suite` name and exit (no DB / reth access) | false |
| `--execution-delay-blocks` | Blocks between observation/submission and simulated fill. `1` = observe N, submit at N, fill against post-block N+1 state (see Assumptions #2). Validated `>= 1` and, when both bounds are set, `<= (--to-block - --from-block)` | `1` |
| `--buy-amount-wei` | Buy amount in wei (also the sell amount for the core engine) | `10000000000000000` |
| `--min-liquidity-eth` | Minimum ETH/WETH reserve for pool eligibility | `0.5` |
| `--min-liquidity-usd` | Minimum stable-denom (USDC/USDT/DAI) reserve for eligibility | `1000` |
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
crate is `ChainSimExecutionAdapter`; do not add the ETH tx executor,
`TxExecutorAdapter`, signer env vars, hot-wallet config, or deployed-vault
config to the backtest binary. If a historical experiment needs to exercise live
tx-prep code, run it as a lab/calibration artifact that writes requests or uses
ETH tx executor dry-run, not as `eth_alpha_backtest_trader`.
