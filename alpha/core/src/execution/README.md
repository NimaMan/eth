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

## Shared Lifecycle Contract

All alpha runtimes use the same order/position lifecycle:

```text
BuyIntentCreated
  -> BuySubmitted
  -> BuyConfirmed | BuyFailed | BuyCancelled
  -> SellIntentCreated
  -> SellSubmitted
  -> SellConfirmed | SellFailed | SellCancelled
```

Strategies create decisions. The engine converts actionable decisions into
`OrderIntent`s. Position state changes only from `ExecutionReport`s.

`Submitted` means the execution adapter accepted the order for its runtime:

- real live: Kartal accepted a transaction request and returned a broadcast tx
  hash;
- live backtest or historical backtest: the simulator accepted the order and a
  synthetic submitted report is recorded at the decision block.

`Confirmed` means the runtime has positive execution evidence:

- real live: a canonical receipt/reconciliation worker proves the tx succeeded
  and the vault/position evidence matches the order;
- live backtest or historical backtest: EVM simulation at the configured target
  execution block succeeds, usually block `N+1` for a decision made from block
  `N`.

`Failed` means the runtime has negative execution evidence, such as a reverted
receipt or failed EVM simulation. `Cancelled` means the order did not remain
eligible for execution and no confirmed fill should be assumed.

## Python Lesson

Live and backtest should both report execution through the same path. Backtest may synthesize reports; live reports come from the executor and confirmed chain data.
