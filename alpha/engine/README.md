# Engine

Crate: `eth_alpha_engine`

This is the high-level live trading runtime. It consumes typed events, runs strategies, applies risk checks, manages portfolio/order state, and routes approved orders to execution.

## Responsibilities

- Own the main event loop.
- Maintain in-memory portfolio and order state.
- Run one or more strategies against market updates.
- Let strategies react to semantic mempool risk events.
- Apply risk gates before order submission.
- Submit approved `OrderIntent`s through an engine execution adapter.
- Apply `ExecutionReport`s to position/order state.
- Persist state through a `TradingStore`.
- Expose operational snapshots for dashboards and recovery.

## Non-Responsibilities

- It does not parse blocks.
- It does not decode transaction logs.
- It does not simulate pending mempool txs directly.
- It does not sign transactions.
- It does not own live-state schemas.
- It does not use snapshot-price or perfect-fill execution for strategy
  evaluation. No-capital execution goes through chain-state EVM simulation.

## Event Loop

```text
LiveFeedEvent received
  -> update market view
  -> mark open positions to market with chain-sim sell valuation when available
  -> run strategies
  -> convert StrategyDecision to OrderIntent
  -> apply portfolio/risk checks
  -> submit to ExecutionAdapter
  -> persist order state

ExecutionReport received
  -> update order state
  -> update position state
  -> persist trade/snapshot

RiskEvent received
  -> update risk view
  -> pause/cancel/sell if policy requires it
```

## Runtime Event Sequence

This is the Rust-side replacement for the legacy Python event-sequence document. Keep the sequencing, but not the Python ownership model: confirmed feed, mempool risk, trading engine, execution, and persistence remain separate crates or adapters.

### Confirmed Chain Flow

```text
processed block arrives
  -> eth_live_feed updates token/pool read models
  -> eth_live_feed writes canonical live-state snapshots when it owns the write path
  -> eth_live_feed emits MarketEvent / LiveFeedEvent
  -> eth_alpha_engine updates its market view
  -> eth_alpha_engine marks open positions to market with chain-sim sell valuation when available
  -> eth_alpha_engine runs strategies
  -> StrategyDecision becomes OrderIntent when actionable
  -> risk policy checks the OrderIntent against active RiskEvents and portfolio state
  -> EngineExecutionAdapter submits or simulates the order
  -> ExecutionReport returns through the same engine path
  -> engine applies order and position transitions
  -> TradingStore persists order, execution, position, and snapshot data
```

Confirmed market state must be updated before strategies run. Strategy decisions should never mutate position state directly; position transitions come from `ExecutionReport`.

### Shared Trade Lifecycle

The engine uses one lifecycle for live real trading, live backtest, and
historical backtest:

```text
BuyIntentCreated
  -> BuySubmitted
  -> BuyConfirmed | BuyFailed | BuyCancelled
  -> SellIntentCreated
  -> SellSubmitted
  -> SellConfirmed | SellFailed | SellCancelled
```

The adapter decides what counts as execution evidence. Real live trading must
only confirm from receipt/reconciliation evidence after Kartal broadcast. The
chain-sim adapters used by live backtest and historical backtest confirm from
EVM simulation at the configured execution block, usually the next block after
the strategy decision. The engine applies both through the same
`ExecutionReport` path.

### Mempool Risk Flow

```text
mempool transaction observed
  -> mempool_processor reads confirmed state from eth_live_state
  -> mempool_processor simulates speculative pending impact through tx_simulator
  -> mempool_processor emits RiskEvent / MempoolSignal
  -> eth_alpha_engine records the active risk view
  -> strategies and risk policy react by holding, rejecting, canceling, reducing, or forcing exit
```

Mempool risk is speculative. It may read canonical token, pool, block, and portfolio snapshots, but it must not overwrite confirmed live-state keys.

### Persistence And Write Ordering

Critical in-memory decisions happen before lower-priority analytics writes:

1. Confirmed feed updates token/pool state and publishes block readiness.
2. Engine updates market/risk/portfolio state and runs strategies.
3. Engine records `OrderIntent` before execution.
4. Engine records `ExecutionReport` after execution or simulation.
5. Engine applies order and position state transitions from the report.
6. `TradingStore` persists durable trade, position, and snapshot records.
7. Analytics writers can derive PnL, strategy metrics, and dashboard summaries from store records after the critical path.

Backtest and live no-capital execution use the same sequence and the same
chain-sim fill source. Their confirmation reports mean that the EVM simulation
succeeded at the target execution block, not that a real transaction was mined.
The guarded live-real runner swaps in `TxExecutorAdapter`, but the systemd
service keeps that path in Kartal dry-run until production simulation,
gas-rank, buy-route, and receipt reconciliation gates are complete. Public real
execution must consult `eth_block_tx_rank` before submission and persist the
rank evidence with the order decision; `tx_executor` only receives the final
prepared transaction.

### Python Concept Mapping

| Python concept | Rust home |
| --- | --- |
| `LiveBlockTokenProcessor` confirmed block updates | `eth_live_feed` plus `eth_token` token/pool state |
| `LiveTokensCache` canonical token cache | `eth_live_feed` in-process state and `eth_live_state` snapshots |
| Engine-local `_pool_eth_levels` | confirmed pool snapshots in `eth_live_state` read by `mempool_processor` |
| `MempoolProcessor` pending transaction checks | `mempool_processor` |
| Strategy engines / position managers | `eth_alpha_engine` plus `eth_alpha_core::PortfolioState` and position types |
| `TradeSignal` execution-like messages | `StrategyDecision` converted to `OrderIntent` by the engine |
| Legacy strategy position DB writes | `TradingStore` implementation |
| `TokenPnLWriter` | analytics or reporting writer derived from stored executions and snapshots |

## Current Skeleton

- `EngineEvent::{Market, Risk, Execution}` is the top-level input.
- `AlphaEngine` owns portfolio state, active risks, strategies, risk policy, store, and execution adapter.
- `ChainSimExecutionAdapter` and `LiveChainSimExecutionAdapter` return
  `ExecutionReport`s from EVM simulation against selected chain state.
- `TxExecutorAdapter` is the real submission boundary. It is crate-private to
  the engine, delegated only through `src/live_trader/real_execution.rs`, and
  is not exported for backtest crates.
- `LiveTradingPlannerBridge` adapts the engine's `LiveTxPlanner` trait to
  `alpha/live/trading::PrioritySellPlanner`; `LiveTxPlanningInputResolver` is
  the runtime hook for loading position, pool, wallet, and observation context.
- `BlockCriticalRiskPolicy` rejects new orders when a matching critical token/pool risk is active.
- `MemoryTradingStore`, `AllowAllRiskPolicy`, and `BlockCriticalRiskPolicy` are test/runtime placeholders, not the final persistent store or full risk model.

## Source Layout

Each folder under `src/` has one `README.md` that names its ownership boundary.

| Path | Owns |
| --- | --- |
| `src/bin/` | executable wrappers only |
| `src/execution/` | simulation adapters and crate-private real live adapter |
| `src/live_trader/` | live polling runner and live-real/live-backtest wiring |
| `src/runtime/` | `AlphaEngine` event handling and execution flow |
| `src/decision/` | strategy decision records and persistence |
| `src/valuation/` | position valuation and snapshot helpers |
| `src/store/` | engine-local store implementations |
| `src/wire.rs` | token-server wire types and parsers |

## Real Submission Status

Alpha has separate trader entrypoints for each runtime boundary:

| Binary | Adapter | Broadcast capability |
| --- | --- | --- |
| `eth_alpha_live_backtest_trader` | `LiveChainSimExecutionAdapter` | None; never contacts Kartal. |
| `eth_alpha_live_trader` | `TxExecutorAdapter` via `LiveTradingPlannerBridge` | Kartal dry-run only. The binary refuses to start if Kartal is not in `dry_run`. |
| `eth_alpha_backtest_trader` | `ChainSimExecutionAdapter` | None; historical replay only. |

The explicit binaries in `src/bin/` are intentionally thin wrappers. Shared
live runner code lives under `src/live_trader/`; real/Kartal wiring is isolated
in `src/live_trader/real_execution.rs`.
`eth_alpha_live_trader` calls `run_live_real()` and
`eth_alpha_live_backtest_trader` calls `run_live_backtest()`, so the binaries do
not expose a public mode switch.

The real live binary is intentionally sell-only for now: it requires
`--disable-entry`, targets the deployed `UniswapV2TradingVault`, and uses shadow
dry-run simulation/gas-rank providers only to prove the live executor boundary.
A receipt reconciliation worker now exists for real submitted tx hashes: it
polls Kartal's configured RPC, requires successful receipts, and confirms V2
vault fills only from `BoughtV2` or `EmergencySoldV2` events. A
public-broadcast deployment still needs:

- production `PreSubmitSimulator` for the exact calldata;
- production `GasRankProvider` backed by recent block-rank evidence;
- real vault buy route planning and position reconciliation;
- operational finality policy and alerting around the receipt worker.

Backtest binaries cannot import `TxExecutorAdapter` through the public engine
API; they should remain on `ChainSimExecutionAdapter` only.

## Trader Binary

`eth_alpha_live_backtest_trader` polls the Rust token server and consumes:

- `/live/pools` as confirmed market updates.
- `/mempool/signals?since_days=14` as speculative risk events.

The trader registers strategy wrappers from each strategy's `live/` module. For
Snipe All that is `LiveSnipeAllStrategy`, which composes the regular
`SnipeAllStrategy` policy while marking the runtime side in code and run config.

`eth_alpha_live_trader --disable-entry` uses the same live input stream but a
Kartal executor adapter. That process is separate from backtests and the
no-capital chain-sim service.

## Live Strategy Model

The engine should treat live and historical strategy runs as the same strategy
policy over different event streams:

- Historical side: stored confirmed-chain observations, plus stored mempool
  signals only when replay enabled them.
- Live side: confirmed-chain observations plus live mempool signals as current
  actionable inputs.

The runtime-level difference is that live can act on mempool signals before the
confirmed pool snapshot reflects the risky transaction. The strategy decides
what that signal means. The first live strategy variants to isolate are:

- Exit immediately on a matching mempool liquidity-removal signal.
- Exit immediately on a matching critical pool LP approval signal.

Only the current token-tracking runtime should be called live. Stored-signal
replay in `eth_alpha_backtest` is mempool-aware historical replay, even when it
tests the same mempool-triggered exits.

Operational events use the shared `eth_ops_events` schema. The
trader reads `ALPHA_TRADER_LOG_DIR` from `blockchains/eth/config.env` and writes
JSONL files there. If that key is omitted, it falls back to:

```text
/home/nima/code/crypto/blockchains/eth/logs/alpha_trader/alpha-trader-<YYYYMMDD-HHMMSSZ>-pid-<pid>/
```

The directory contains `pipeline_health.jsonl`, `pipeline_issues.jsonl`, and
`pipeline_bottlenecks.jsonl`. Poll failures against the token server are emitted
as `stage=alpha_trader`, `component=token_server_poll`, and
`code=alpha_trader_poll_failed`; regular loop heartbeats are emitted as
`PipelineHealth` records.

Default mode only primes current pool/signal watermarks so it does not retroactively trade old state:

```bash
cargo run -p eth_alpha_engine --bin eth_alpha_live_backtest_trader -- --once
```

Use `--replay-current` for a local smoke test that replays the current
token-server snapshot through chain simulation.

The deployed no-capital runtime uses a stable Snipe All chain-sim live run id.
On startup it restores active positions
from `alpha_trading.positions` and restores pool/signal watermarks from
`alpha_trading.strategy_observations`.
The deployed Snipe All thresholds are `--min-liquidity-eth 0.5` for ETH/WETH pools and `--min-liquidity-usd 1000` for USDC/USDT/DAI pools.

Pool matching uses `TokenPoolId` from `eth_alpha_core`: `token_address:pool_identity`. For V2/V3 the pool identity is the pool contract address; for V4 it is `pool_manager#pool_id`. The engine should never coerce V4 pools into fake EVM addresses just to fit order or position keys.

## Lessons From Python

The Python `LiveStrategyEngine` sometimes updated positions when a signal was submitted and later confirmed on the next token update. In this engine, confirmations should come from `ExecutionReport`.

Backtest mode simulates reports against historical chain state, but it still
uses the same report path:

```text
OrderIntent -> ChainSimExecutionAdapter -> ExecutionReport
```

Live no-capital mode uses:

```text
OrderIntent -> LiveChainSimExecutionAdapter -> ExecutionReport
```

This keeps live and backtest behavior aligned.
