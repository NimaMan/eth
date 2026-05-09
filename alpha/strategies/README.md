# Strategies

Crate: `eth_strategies`

This crate contains built-in strategies. Strategies are decision logic only.

## Current Implementations

- `MarketTrackerStrategy`: submits one paper buy per tradable pool or `TradingEnabled` risk event, then suppresses repeat buys for that pool. It blocks itself when a matching critical risk is active.
- `SnipeAllStrategy`: first live paper policy. It buys every newly observed eligible live pool once and exits a matching open position when a liquidity-removal risk event arrives.

## Snipe All V1

`SnipeAllStrategy` is intentionally simple and explicit. Mempool signals are assumed to be a normal part of every serious strategy, so the strategy name describes the entry posture rather than the signal source.

Entry:

- Buy each eligible live pool once.
- Skip pools that cannot buy, cannot sell, are flagged as scam, use an unsupported quote currency, or are below the denomination-specific liquidity floor.
- Supported quote currencies are `ETH`, `WETH`, `USDC`, and `USDT`. ETH/WETH pools use `min_denom_reserve`; USDC/USDT pools use `min_stable_denom_reserve`.
- The shared rule contract lives in `alpha/token_eligibility`; strategy-specific config only overrides that contract's thresholds.
- `DAI` remains unsupported for Snipe All unless we explicitly add it later.
- Skip historical warmup state in the live trader; the runtime primes watermarks and only sends new live changes once the token tracker reports `live`.

Exit:

- Sell a matching open position on `RiskKind::LiquidityRemoval`.
- `LpApproval`, creator-flow labels, and tax/honeypot rules are scaffolded as named rule modules but currently hold.

Planned rule growth:

- Label token creators by public mempool vs private execution behavior.
- Sell immediately when private-labeled creators approve LP tokens.
- Add tax/honeypot exits and creator blocklists.
- Persist strategy-decision audit rows so the frontend can show exactly which named rule opened, held, or exited a position.

## Responsibilities

- Implement `eth_alpha_core::Strategy`.
- Read `StrategyContext`, market snapshots, risk state, and current portfolio state.
- Return `StrategyDecision`.
- Keep strategy-local parameters and lightweight memory.

## Non-Responsibilities

- No transaction submission.
- No signing.
- No DB writes.
- No Redis reads.
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
| `MarketTracker` | buy when token lifecycle becomes `TRADING_ENABLED` | never sell; hold for analytics | benchmark strategy that submits small paper buys and keeps positions open |
| `BuyAll` | buy every trading-enabled token | sell when ROI reaches `profit_target_x`, default `7.0` | simple lifecycle strategy for engine/backtest validation |
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

Strategies must not assume they can observe intra-block ordering unless the market event explicitly provides it. A strategy decision made from block `N` is a decision after observing the block-level state for `N`, not a guaranteed transaction position inside that block.

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
