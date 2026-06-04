# Runtime

Runtime owns the `AlphaEngine` event loop and order execution flow.

- `mod.rs`: applies market/risk/execution events and runs strategies.
- `event_flow.rs`: small helpers for event block numbers and submitted reports.
- `execution_flow.rs`: applies strategy decisions, risk policy, execution reports,
  pending reports, and position state updates.

## Operator buy-halt gate ("exit-only")

`execution_flow.rs` is the single chokepoint where every `SubmitOrder` intent is
turned into an order. Before risk/execution, `buy_entry_halted()` drops a `Buy`
intent when its strategy is in `AlphaEngine.halt_buys_strategies` (set each poll
from `strategy_buy_controls`). Because sells, strategy exits and manual closes
are always `OrderSide::Sell`, they are never gated — a paused strategy keeps
exiting and can still be manually closed. The skip is logged with reason
`entry.paused_by_operator`.
