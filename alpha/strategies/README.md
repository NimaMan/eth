# Strategies

Crate: `eth_strategies`

This crate contains built-in strategies. Strategies are decision logic only.

## Current Implementations

- `MarketTrackerStrategy`: submits one chain-sim buy per tradable pool or `TradingEnabled` risk event, then suppresses repeat buys for that pool. It blocks itself when a matching critical risk is active.
- `SnipeAllStrategy`: regular Snipe All policy. It buys every newly observed eligible pool once; exit rules are configurable and include liquidity-removal, LP approval, tax/honeypot, and scam risk events.
- `LiveSnipeAllStrategy`: live runtime wrapper under `baseline/snipe_all/live/`. It composes the regular Snipe All policy and is the only Snipe All type registered by the current live trader.

## Snipe All V1

`SnipeAllStrategy` is intentionally simple and explicit. Mempool signals are assumed to be a normal part of every serious strategy, so the strategy name describes the entry posture rather than the signal source.

Current layout:

```text
baseline/snipe_all/
  config.rs
  strategy.rs
  live/
    config.rs
    strategy.rs
```

Any strategy that runs against current live token tracking must have a `live/`
module. Rust does not use class inheritance here; the live side composes the
regular side and delegates shared policy behavior to it. Historical replay uses
the regular side directly.

## Live And Historical Sides

Each strategy should be written as one policy with two runtime sides:

- Regular/historical side: replay stored confirmed-chain observations, and
  include stored mempool signal rows only when the backtest config opts in.
  When stored mempool rows are included, call the run mempool-aware historical
  replay, not live.
- Live side: consume confirmed-chain observations plus live mempool signals as
  current actionable inputs.

The difference between live and historical is therefore input timing. Live can
act on mempool signals before the confirmed pool snapshot reflects the risky
transaction; historical replay can only test that behavior when the stored
signal stream is included. After inputs arrive, behavior is strategy-specific.

Initial live backtest strategy candidates:

- Liquidity-removal exit: buy from the normal entry policy, then exit a matching
  open position as soon as a mempool liquidity-removal signal arrives.
- Critical LP-approval exit: buy from the normal entry policy, then exit a
  matching open position as soon as a critical pool LP approval signal arrives.

Entry:

- Buy each eligible live pool once.
- Skip pools that cannot buy, cannot sell, use an unsupported quote currency, or are below the denomination-specific liquidity floor. Scam and risk labels are later outcomes or exit inputs, not first-pass eligibility gates.
- Supported quote currencies are `ETH`, `WETH`, `USDC`, `USDT`, and `DAI`. ETH/WETH pools use `min_denom_reserve`; USDC/USDT/DAI pools use `min_stable_denom_reserve`.
- The shared rule contract lives in `alpha/pool_classification`; strategy-specific config only overrides that contract's thresholds.
- Skip historical warmup state in the live trader; the runtime primes watermarks and only sends new live changes once the token tracker reports `live`.

Exit:

- Sell a matching open position on enabled exit risk kinds such as
  `RiskKind::LiquidityRemoval`, `RiskKind::LpApproval`, `RiskKind::TaxChange`,
  `RiskKind::Honeypot`, and `RiskKind::ScamConfirmed`.
- Keep liquidity-removal and critical LP-approval exits as explicit strategy
  variants or run configs first so each live/historical pair can be backtested
  independently.

Real execution status: these strategies stop at `StrategyDecision` /
`OrderIntent`. That is correct. The live-capital gap is not inside strategy
rules; it is the missing planner that converts an approved sell `OrderIntent`
into route calldata, simulation evidence, gas-rank candidates, and a Kartal
direct-raw request.

Planned rule growth:

- Label token creators by public mempool vs private execution behavior.
- Sell immediately when private-labeled creators approve LP tokens.
- Add tax/honeypot exits and creator blocklists.
- Persist strategy-decision audit rows so the frontend can show exactly which named rule opened, held, or exited a position.

## Responsibilities

- Implement `eth_alpha_core::Strategy`.
- Return a strategy name from config, not a hardcoded implementation name, so
  multiple variants can run in one engine without sharing positions or PnL.
- Read `StrategyContext`, market snapshots, risk state, and current portfolio state.
- Return `StrategyDecision`.
- Keep strategy-local parameters and lightweight memory.

## Non-Responsibilities

- No transaction submission.
- No signing.
- No DB writes.
- No external cache reads.
- No ZMQ publishing.
- No direct mempool simulation.

## Initial Strategy Ports

The Python module had these useful starting points:

- `MarketTracker`: buys/tracks broad market performance; good for analytics and benchmarking.
- `BuyAll`: simple entry/exit state machine; good for validating engine behavior.
- `WalletTrackerStrategy`: wallet-specific limits and active-position tracking.
- `BuyScamStrategy`: useful as a risk/behavior experiment, but should be treated carefully.

## Python Strategy Audit

The legacy Python strategies lived in the removed Python portfolio manager's `strategy/` package.
They all implemented `BaseStrategy.analyze_token(token, position)` and returned a `TradeSignal`.
The Python engine then mutated `TokenPosition` state from that signal.

That shape is useful for migration, but it should not be copied directly:

| Python strategy | Entry rule | Exit rule | Rust port target |
| --- | --- | --- | --- |
| `MarketTracker` | buy when token lifecycle becomes `TRADING_ENABLED` | never sell; hold for analytics | benchmark strategy that submits small chain-sim buys and keeps positions open |
| `BuyAll` | buy every trading-enabled token | sell when ROI reaches `profit_target_x`, default `7.0` | simple lifecycle strategy for engine/strategy validation |
| `BuyScamStrategy` | buy when `latest_token_assessment.is_scam` is true | sell when ROI reaches `profit_target_x`, default `7.0` | controlled research strategy only; never enable for live execution without explicit risk policy |
| `WalletTrackerStrategy` | buy healthy trading-enabled tokens while below `max_positions` and not already active | sell on profit target, stop loss, or token scam flag | wallet-scoped strategy using engine portfolio state, not process-local booleans |

Python state handlers were:

```text
INIT
  -> SUBMIT_BUY
BUY_SUBMITTED
  -> CONFIRM_BUY on the next token update
BUY_CONFIRMED
  -> SUBMIT_SELL when strategy exit rule is true
SELL_SUBMITTED
  -> CONFIRM_SELL on the next token update
```

Rust should not model `CONFIRM_BUY` or `CONFIRM_SELL` as strategy decisions. A strategy only says what it wants. The engine and execution adapter produce the report that changes position state:

```text
Hold
SubmitOrder(OrderIntent)
CancelOrders { ... }

OrderIntent
  -> ExecutionAdapter
  -> ExecutionReport
  -> position/order transition
```

## Strategy Contract

A strategy can say what it wants. The engine decides whether it is allowed.

```text
StrategyDecision
  -> RiskCheck
  -> PortfolioLimits
  -> ExecutionAdapter
```

This avoids the Python problem where strategy, position manager, and signal publisher were tightly coupled.

## Block-Level Decision Semantics

The token runtime operates at block granularity. For a given processed block:

1. Token and pool state are updated from that block.
2. The strategy sees a market snapshot for that block.
3. The strategy returns `Hold` or an actionable `StrategyDecision`.
4. The engine turns an approved decision into an `OrderIntent`.
5. Backtest or live execution decides the fill and emits `ExecutionReport`.

Strategies must not assume they can observe intra-block ordering unless the market event explicitly provides it. A strategy decision made from block `N` is a decision after observing the block-level state for `N`, not a guaranteed transaction position inside that block. For future real execution, the strategy or real adapter must consume `eth_block_tx_rank` evidence before submitting through `tx_executor`.

## Backtest Fill Contract

Backtests should be pessimistic by default. When a strategy decides after a token update for a block, the simulated order should use the worst executable price the strategy could plausibly receive in the eligible block-level fill window:

- For buys, use the most adverse buy price/output available in that block-level model.
- For sells, use the most adverse sell price/output available in that block-level model.
- Include the configured gas, fee, slippage, latency, and failed-transaction assumptions in the backtest report.
- If the replay only has one pool snapshot for the block, mark the fill as block-snapshot based and do not claim exact intra-block execution.

This keeps historical results conservative. It also avoids the old Python behavior where submit and confirm were often inferred from consecutive token updates rather than from an execution report.

## Rust Migration Rules

- Strategy implementations own parameters and lightweight memory only.
- Portfolio exposure, max-position checks, and wallet balances come from `StrategyContext` and engine state.
- Token health, scam labels, LP risk, and mempool risk come from `MarketSnapshotRef` and `RiskEvent`.
- Position state transitions are owned by `eth_alpha_core::position` and applied from `ExecutionReport`.
- Live and backtest strategy behavior must use the same `Strategy` trait.
- Research strategies such as `BuyScamStrategy` must be gated so they cannot accidentally route to live execution.
