# strategy

Strategy API and decision model.

## Owns

- `Strategy`
- `StrategyContext`
- `StrategyDecision`

## Does Not Own

- Built-in strategy implementations.
- Order submission.
- Persistence.
- Live service orchestration.

## Python Lesson

Python strategies returned `TradeSignal`s that were close to execution messages. Rust strategies should return decisions. The engine converts approved decisions into `OrderIntent`s.

The legacy Python strategy methods also handled pseudo-confirmation states such as `BUY_SUBMITTED -> CONFIRM_BUY` and `SELL_SUBMITTED -> CONFIRM_SELL`. Rust strategies must not confirm their own orders. Confirmation is an execution concern:

```text
StrategyDecision
  -> OrderIntent
  -> ExecutionReport
  -> PositionState transition
```

At block level, a strategy receives the market view for a processed block and returns intent from that view. Backtest fill policy, including worst-case block-level price selection, belongs to the backtest execution adapter rather than the strategy trait.
