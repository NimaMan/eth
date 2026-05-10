# Paper Execution Adapter

Perfect-fill adapter for rapid prototyping and integration tests.

## Behavior

- Every `OrderIntent` is confirmed instantly.
- `filled_amount` equals `intent.amount` exactly.
- `gas_used` is reported as `0`.
- No price impact, slippage, or failure modeling.

## Use Cases

- Integration tests where you need deterministic fills.
- Live trader `--mode=paper` baseline to verify the pipeline wires correctly
  before turning on fill modeling.

## When NOT to Use

- Do not use for PnL measurement. Perfect fills hide losses.
- Do not use for strategy validation. It will make bad strategies look profitable.

## Future Work

This adapter will remain as the simplest reference implementation of
`EngineExecutionAdapter`. All honest fill logic belongs in `modeled/`.
