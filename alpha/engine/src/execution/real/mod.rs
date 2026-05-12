//! Real on-chain execution adapter.
//!
//! This adapter bridges `OrderIntent` to `tx_executor` so that approved orders
//! are signed and broadcast to the Ethereum mempool.
//!
//! **Planned**: not yet wired. When built it will:
//! 1. Resolve `OrderIntent` → `AmmSwapRoute`.
//! 2. Build swap calldata via `tx_simulator::tx_builders`.
//! 3. Optionally pre-simulate via `tx_simulator` (revert guard).
//! 4. Query `eth_block_tx_rank` for rough mined-block rank evidence.
//! 5. Persist that rank evidence with the order decision.
//! 6. Submit the final `DirectRawTransactionRequest` to `tx_executor::EthTxExecutor`.
//! 7. Poll for receipt and map real `tx_hash` back to `TokenPoolId`.
//!
//! This adapter is the **only** component in `alpha/engine` that talks to
//! `tx_executor`. Rank checks happen before that boundary in `eth_block_tx_rank`.
//! It must be explicitly enabled via a config flag and capital limit so
//! no-capital operators cannot accidentally submit real transactions.

// TODO: implement TxExecutorAdapter
