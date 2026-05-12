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
- It does not own Redis live-state schemas.
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

### Mempool Risk Flow

```text
mempool transaction observed
  -> eth_mempool_risk reads confirmed state from eth_live_state
  -> eth_mempool_risk simulates speculative pending impact through tx_simulator
  -> eth_mempool_risk emits RiskEvent / MempoolSignal
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
chain-sim fill source. Real trading later swaps in a `tx_executor` adapter.

### Python Concept Mapping

| Python concept | Rust home |
| --- | --- |
| `LiveBlockTokenProcessor` confirmed block updates | `eth_live_feed` plus `eth_token` token/pool state |
| `LiveTokensCache` canonical token cache | `eth_live_feed` in-process state and `eth_live_state` snapshots |
| Engine-local `_pool_eth_levels` | confirmed pool snapshots in `eth_live_state` read by `eth_mempool_risk` |
| `MempoolProcessor` pending transaction checks | `eth_mempool_risk` |
| Strategy engines / position managers | `eth_alpha_engine` plus `eth_alpha_core::PortfolioState` and position types |
| `TradeSignal` execution-like messages | `StrategyDecision` converted to `OrderIntent` by the engine |
| Legacy strategy position DB writes | `TradingStore` implementation |
| `TokenPnLWriter` | analytics or reporting writer derived from stored executions and snapshots |

## Current Skeleton

- `EngineEvent::{Market, Risk, Execution}` is the top-level input.
- `AlphaEngine` owns portfolio state, active risks, strategies, risk policy, store, and execution adapter.
- `ChainSimExecutionAdapter` and `LiveChainSimExecutionAdapter` return
  `ExecutionReport`s from EVM simulation against selected chain state.
- `BlockCriticalRiskPolicy` rejects new orders when a matching critical token/pool risk is active.
- `MemoryTradingStore`, `AllowAllRiskPolicy`, and `BlockCriticalRiskPolicy` are test/runtime placeholders, not the final persistent store or full risk model.

## Trader Binary

`eth_alpha_trader` is the first runnable alpha runtime inside this crate. It
runs in `chain-sim` mode, polls the Rust token server, and consumes:

- `/live/pools` as confirmed market updates.

Operational telemetry uses the shared `eth_pipeline_telemetry` schema. The
trader writes JSONL files under `ALPHA_TRADER_LOG_DIR` or, by default:

```text
/home/nima/code/crypto/blockchains/eth/logs/alpha_trader/<run-id>/
```

The directory contains `pipeline_health.jsonl`, `pipeline_issues.jsonl`, and
`pipeline_bottlenecks.jsonl`. Poll failures against the token server are emitted
as `stage=alpha_trader`, `component=token_server_poll`, and
`code=alpha_trader_poll_failed`; regular loop heartbeats are emitted as
`PipelineHealth` records.
- `/mempool/signals?since_days=14` as speculative risk events.

Default mode only primes current pool/signal watermarks so it does not retroactively trade old state:

```bash
cargo run -p eth_alpha_engine --bin eth_alpha_trader -- --once
```

Use `--replay-current` for a local smoke test that replays the current
token-server snapshot through chain simulation.

The deployed no-capital runtime uses a stable
`--run-id snipe-all-v1-chain-sim-live`. On startup it restores active positions
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
