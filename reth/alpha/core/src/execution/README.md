# execution

Execution reports and execution adapter contract.

## Owns

- `ExecutionReport`
- `ExecutionStatus`
- `ExecutionAdapter` trait

## Does Not Own

- Concrete `tx_executor` implementation.
- Simulation implementation.
- Wallet private keys.

## Python Lesson

Live and backtest should both report execution through the same path. Backtest may synthesize reports; live reports come from the executor and confirmed chain data.
