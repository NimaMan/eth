//! EVM-backed execution adapter.
//!
//! This adapter bridges `OrderIntent` to `tx_simulator` so that every fill is
//! validated by running the actual swap calldata against a local Reth MDBX
//! database before submission.
//!
//! **Planned**: not yet wired. When built it will:
//! 1. Resolve `OrderIntent` → `AmmSwapRoute` using pool protocol.
//! 2. Build swap calldata via `tx_simulator::tx_builders`.
//! 3. Simulate via `tx_simulator::TxSimulator` against the current block.
//! 4. Return `ExecutionReport` with actual `filled_amount`, `gas_used`, and
//!    revert status.
//!
//! Use this for pre-flight validation before real execution, or for the most
//! honest paper fills when pool-snapshot math is insufficient.

// TODO: implement SimulatedExecutionAdapter
