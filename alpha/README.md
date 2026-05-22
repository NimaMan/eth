# Alpha

Agent operating map for the Ethereum decision layer. `alpha/` consumes confirmed
live token state plus speculative mempool risk, turns those into strategy
events, and persists decisions before execution.

## Purpose

- Run trading strategy state machines over `MarketEvent`, `RiskEvent`, and
  `ExecutionReport`.
- Keep strategy decisions auditable in Postgres before trusting PnL.
- Keep live chain-sim, backtest, and guarded real execution behind the same core
  domain contracts.

## Owns

| Folder | Crate | Owns |
| --- | --- | --- |
| `core/` | `eth_alpha_core` | Domain types, traits, IDs, portfolio/order/position/risk models. |
| `engine/` | `eth_alpha_engine` | Runtime state machine, order/position transitions, risk gating, execution adapter boundary. |
| `block_tx_rank/` | `eth_block_tx_rank` | Rough block-position and gas-before estimates from recent mined transaction fees. |
| `strategies/` | `eth_strategies` | Concrete strategy rules such as `SnipeAllStrategy`. |
| `store/` | `eth_alpha_store` | Durable run, observation, order, execution, position, and risk records. |
| `live/state/` | `eth_live_state` | Shared live-state snapshot schemas, protocol types, and store traits. |
| `live/feed/` | `eth_live_feed` | Confirmed processed-block/token feed used by live services. |
| `live/trading/` | `eth_live_trading` | Live priority-exit policy, tx-prep, value-capped gas planning, and Kartal request/client shape. |
| `backtest/` | `eth_alpha_backtest` | Historical replay over the same core strategy contracts. |

## Does Not Own

- Raw simulation, traces, or calldata building; use `tx_simulator`,
  `tx_processor`, and `reth_chain_query`.
- Canonical token/pool state mutation; use `eth_token` and `eth_chain_server`.
- Signing, nonce management, final fee-cap validation, or broadcast; use
  `tx_executor`.
- Mempool ingestion or signal persistence; use `mempool_processor`.

## Data Flow

```text
eth_chain_server LiveChainRuntime
  -> direct processed-block feed + live token/pool views
  -> eth_alpha_live_backtest_trader or eth_alpha_live_trader polls /live/status, /live/pools, /mempool/signals
  -> strategies emit StrategyDecision / OrderIntent
  -> chain-sim service or guarded kartal-real service
  -> real adapter prepares a Kartal direct-raw request through live/trading
  -> strategy_observations + orders + reports + positions + risk events
```

`eth_alpha_live_backtest_trader` is the no-capital live runner. It must not
become decision-active until `/live/status` is `live`; while warming, it records
heartbeats and primes watermarks only. It always uses live chain-state EVM
simulation and has no Kartal execution path.

`eth_alpha_live_trader` is the separate real-executor runner. It instantiates
`TxExecutorAdapter`, uses the deployed Uniswap V2 trading vault route, and
refuses to start unless Kartal reports `broadcast_mode = dry_run`, except for
the explicit one-pool Alpha11 hold3 mined-validation run. That exception requires
`--allow-public-mempool-live-validation`, strategy
`alpha11-univ2-lp30-pool-update-block-hold3-validation`,
no `--replay-current`, and strategy-spec buy/bankroll caps of `0.01 ETH`.
While real entries are otherwise in validation mode, each strategy must resolve
to a bankroll of at most `0.225 ETH`; Alpha11 carries that default in its
strategy spec. Buys consume that bankroll, confirmed sells replenish it, and
profits can be redeployed. Strategy economics and gates, including buy size,
liquidity floors, entry-pool caps, bankroll, and hold window, are not live-run
CLI parameters; named live strategies own those values in their strategy specs.
The V2 buy and emergency-sell paths derive non-zero min-output from provisional
exact-calldata simulation and simulate the final exact vault calldata before
Kartal submission.

Backtests are not a service and must never be able to broadcast. The backtest
binary stays in `alpha/backtest`, reads historical inputs, and only constructs
`ChainSimExecutionAdapter`.

Snipe All currently supports ETH/WETH and USD-stable quote pools. Use separate
floors for each family: WETH-denominated pools are not comparable to
USDC/USDT/DAI pools by raw reserve amount.

## Persistent Stores

Alpha durable state lives in PostgreSQL under the `alpha_trading` schema,
configured by `ALPHA_DATABASE_URL`.

| Owner | Tables |
| --- | --- |
| `alpha/store/` | `trader_runs`, `order_intents`, `execution_reports`, `positions`, `position_snapshots`, `backtest_result_sets`, `backtest_result_set_runs`, `trades`, `trade_events`, `trade_snapshots`, `risk_events`, `strategy_decisions`, `strategy_observations` |
| `alpha/lab/` | `strategy_validation_reports` in the same `alpha_trading` schema |

Alpha may read `live_trading.signal_events` through chain-server APIs or replay
tools, but mempool signal persistence and speculative pending-risk ownership
remain in `mempool_processor`, not alpha. Historical/backtest execution also
reads Reth through `RETH_DATADIR`.

Live strategies should be documented as one policy with two sides. The
regular/historical side replays stored confirmed-chain observations and only
includes mempool signals when stored signal rows are selected. That stored
signal replay is mempool-aware history, not live. The live side consumes the
same confirmed-chain updates plus current mempool signals and must live under a
strategy-local `live/` module. The first isolated variants are
liquidity-removal exit and critical LP-approval exit.

## Where To Look First

| Need | Start here |
| --- | --- |
| Event and domain type ownership | `core/src/` and `core/src/README.md` |
| Strategy runtime and execution adapter behavior | `engine/README.md`, `engine/src/lib.rs`, `engine/src/live_trader/` |
| Durable decision ledger | `store/README.md`, Postgres `alpha_trading.*` tables |
| Mined-block transaction rank estimates | `block_tx_rank/README.md`, `block_tx_rank/src/lib.rs` |
| Snipe All entry/exit rules | `strategies/README.md`, `strategies/src/baseline/snipe_all/` |
| Live tx prep and Kartal request shape | `live/trading/README.md`, `live/trading/src/tx_prep/` |
| Live confirmed-chain feed | `live/feed/README.md`, `live/feed/src/` |
| Live-state contract | `live/state/README.md`, `live/state/src/` |
| Service wiring | Thin wrappers in `engine/src/bin/`, shared live runtime in `engine/src/live_trader/`, historical backtest wrapper in `backtest/src/bin/eth_alpha_backtest_trader.rs` |

## Bottleneck Management

The shared bottleneck ledger lives at `../bogaz.md`. Keep alpha architecture and
runtime ownership notes here; move bottleneck measurements, focus order, and
operational mitigation notes to `bogaz.md`.

## Tests And Commands

```bash
cargo test -p eth_alpha_core
cargo test -p eth_block_tx_rank
cargo test -p eth_alpha_engine
cargo test -p eth_alpha_store
cargo test -p eth_strategies
cargo run -p eth_alpha_engine --bin eth_alpha_live_backtest_trader
cargo run -p eth_alpha_engine --bin eth_alpha_live_backtest_trader -- --strategy-set alpha11-univ2-lp30-pool-update-block-hold15
cargo run -p eth_alpha_engine --bin eth_alpha_live_trader -- --strategy-set alpha11-univ2-lp30-pool-update-block-hold15
cargo run -p eth_alpha_engine --bin eth_alpha_live_trader -- --strategy-set alpha11-univ2-lp30-pool-update-block-hold3-validation --allow-public-mempool-live-validation
cargo run -p eth_alpha_backtest --bin eth_alpha_backtest_trader
```

## Current Hazards

- Backtests must never load Kartal config, signer state, hot-wallet balance, or
  deployed vault addresses.
- `kartal-real` public broadcast is only allowed for the one-pool Alpha11 hold3
  mined-validation run. The main hold15 strategy remains blocked from public
  broadcast until mined validation evidence is reviewed.
- Real public execution must include rank evidence through `eth_block_tx_rank`
  before the adapter submits the final prepared transaction.
- `strategy_observations` is the durable input log. In-memory watermarks are
  polling mechanics and must be recoverable from Postgres.
- Fix order for live issues is tracked in `../bogaz.md`.
- Keep confirmed state and speculative mempool risk separate. Strategies consume
  both but do not mutate either.
