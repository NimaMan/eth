# order

Order intent and order lifecycle.

## Owns

- `OrderIntent`
- `OrderSide`
- `OrderStatus`
- route hints

## Does Not Own

- Transaction calldata construction.
- Signing.
- Nonce management.
- Broadcast behavior.

## Python Lesson

Python `TradeSignal` mixed strategy decision and execution request. Rust separates `StrategyDecision` from `OrderIntent`, then execution adapters translate approved intents into concrete executor requests.
