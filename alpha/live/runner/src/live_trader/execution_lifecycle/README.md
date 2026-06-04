# Execution Lifecycle

This folder owns settlement of executions that have already been submitted.

Real live trading and chain-sim live backtesting share the same lifecycle shape:

1. Strategy observes block `N`.
2. Strategy creates an `OrderIntent`.
3. Execution adapter records a submitted `ExecutionReport`.
4. The position moves to `buy_submitted` or `sell_submitted`.
5. A later block provides execution evidence.
6. Alpha applies a final confirmed, failed, cancelled, or deferred report.

The backend-specific difference is the source of step 5:

- real live trading waits for mined transaction receipts and vault events;
- chain-sim live backtesting asks chain-server to simulate our transaction as
  the last transaction at the expected execution block using the server-owned
  `LiveTxSimulator`.

For chain-sim live backtesting, `LiveChainSimExecutionAdapter::execute()` only
records submission. It does not wait for the next block and it does not simulate
inside the strategy callback. The submitted execution report is the database
source of truth; there is no in-memory queue that owns settlement.

`ChainSimSettlement` runs each live tick. It loads submitted chain-sim execution
reports from `alpha_trading.execution_reports`, joins back to the recorded
`OrderIntent`, and checks:

- the position is still in a submitted state;
- no final report exists for the order id;
- the expected execution block is at or behind the current live block;
- chain-server can serve exact live simulation state for that execution block;
- when Alpha recorded the submitted block hash, chain-server verifies that the
  execution block parent still matches it.

Only then does it apply the final execution event returned by chain-server. If
the exact live state block is unavailable or the parent hash indicates a reorg,
the position remains submitted and the trader logs an infrastructure wait. It
must not turn that case into `buy_failed`, because that is not an on-chain
failure.
