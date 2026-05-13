# Alpha

Agent operating map for the Ethereum decision layer. `alpha/` consumes confirmed
live token state plus speculative mempool risk, turns those into strategy
events, and persists decisions before execution.

## Purpose

- Run trading strategy state machines over `MarketEvent`, `RiskEvent`, and
  `ExecutionReport`.
- Keep strategy decisions auditable in Postgres before trusting PnL.
- Keep live chain-sim, backtest, and future real execution behind the same core
  domain contracts.

## Owns

| Folder | Crate | Owns |
| --- | --- | --- |
| `core/` | `eth_alpha_core` | Domain types, traits, IDs, portfolio/order/position/risk models. |
| `engine/` | `eth_alpha_engine` | Runtime state machine, order/position transitions, risk gating, execution adapter boundary. |
| `block_tx_rank/` | `eth_block_tx_rank` | Rough block-position and gas-before estimates from recent mined transaction fees. |
| `strategies/` | `eth_strategies` | Concrete strategy rules such as `SnipeAllStrategy`. |
| `store/` | `eth_alpha_store` | Durable run, observation, order, execution, position, and risk records. |
| `live/state/` | `eth_live_state` | Redis live-state schemas and protocol types. |
| `live/feed/` | `eth_live_feed` | Confirmed processed-block/token feed used by live services. |
| `backtest/` | planned | Historical replay over the same core strategy contracts. |
| `mempool_risk/` | planned | Future crate boundary for pending-risk events; current service is `mempool_processor`. |

## Does Not Own

- Raw simulation, traces, or calldata building; use `tx_simulator`,
  `tx_processor`, and `reth_chain_query`.
- Canonical token/pool state mutation; use `eth_token` and `eth_chain_server`.
- Signing, nonce management, final fee-cap validation, or broadcast; use
  `tx_executor`.
- Mempool ingestion or signal persistence; use `mempool_processor`.

## Data Flow

```text
tx_processor live_block_processor
  -> Redis eth/live/blocks + processed-block disk cache
  -> eth_chain_server live token/pool views
  -> eth_alpha_trader polls /live/status, /live/pools, /mempool/signals
  -> future real adapter checks eth_block_tx_rank before tx_executor
  -> strategy_observations + orders + reports + positions + risk events
```

`eth_alpha_trader` is no-capital chain-sim right now. It must not become
decision-active until `/live/status` is `live`; while warming, it records
heartbeats and primes watermarks only.

Snipe All currently supports ETH/WETH and USD-stable quote pools. Use separate
floors for each family: WETH-denominated pools are not comparable to
USDC/USDT/DAI pools by raw reserve amount.

## Where To Look First

| Need | Start here |
| --- | --- |
| Event and domain type ownership | `core/src/` and `core/src/README.md` |
| Strategy runtime and execution adapter behavior | `engine/README.md`, `engine/src/lib.rs` |
| Durable decision ledger | `store/README.md`, Postgres `alpha_trading.*` tables |
| Mined-block transaction rank estimates | `block_tx_rank/README.md`, `block_tx_rank/src/lib.rs` |
| Snipe All entry/exit rules | `strategies/README.md`, `strategies/src/snipe_all/` |
| Live confirmed-chain feed | `live/feed/README.md`, `live/feed/src/` |
| Redis live-state contract | `live/state/README.md`, `live/state/src/` |
| Service wiring | `engine/src/bin/eth_alpha_trader.rs` |

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
cargo run -p eth_alpha_engine --bin eth_alpha_trader
```

## Current Hazards

- `eth_alpha_trader` uses chain-state simulation only; do not route it to
  `tx_executor` without an explicit adapter, operator gate, and persistence plan.
- Real execution must include a rank-evidence step through `eth_block_tx_rank`
  before the adapter submits the final prepared transaction.
- `strategy_observations` is the durable input log. In-memory watermarks are
  polling mechanics and must be recoverable from Postgres.
- Fix order for live issues is tracked in `../bogaz.md`.
- Keep confirmed state and speculative mempool risk separate. Strategies consume
  both but do not mutate either.
