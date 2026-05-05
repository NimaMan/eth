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
