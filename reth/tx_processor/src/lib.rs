/// Clean TX Processor - Rust alternative to Python eth_block_processor
/// 
/// This is a simplified, clean transaction processor that replaces the complex
/// Python eth_block_processor.txn module with direct Reth database access.
/// 
/// Key improvements over Python version:
/// - 10-40x faster (direct DB vs RPC)
/// - Much simpler codebase
/// - No complex RPC handling
/// - Consistent performance

// Re-export the NEW TX Simulator types (NO CallRequest - uses UnsignedTransaction)
pub use tx_simulator::{
    TxSimulator,
    UnsignedTransaction,
    SimulationResult,
    FullSimulationResult,
    SequentialSimulationResult,
    SequentialTransactionResult,
    SequentialSimulationOptions,
    UnsignedTxChainSimulation,
    ChainStateInfo,
};

// Export transaction processing modules
pub mod tx_processor;
pub mod processed_tx_provider;
pub mod simulator;

// Re-export data models from tx_processor
pub use tx_processor::data_models::{ProcessedTransaction, TransactionFees};
pub use processed_tx_provider::ProcessedTxProvider;
pub mod config;

// Export ERC20 token buy-approve-sell simulator through simulator module
pub use simulator::{
    check_can_buy_sell_pool,
    PoolViabilityConfig,
    PoolViabilityResult,
    PoolType,
    OptionalSetupBuyApproveSellResult,
    simulate_buy_swap,
    BuySwapResult,
    simulate_sell_swap,
    SellSwapResult,
};

// Convenience facade: simulate and return ProcessedTransaction directly
use alloy_primitives::B256;
use eyre::Result;

/// Simulate an unsigned transaction at a block (or latest) and return a fully processed transaction.
pub async fn process_unsigned_tx(
    simulator: &TxSimulator,
    unsigned_tx: UnsignedTransaction,
    block_number: Option<u64>,
) -> Result<ProcessedTransaction> {
    let provider = ProcessedTxProvider::with_provider_factory(simulator.provider_factory().clone())?;
    provider.process_transaction_from_unsigned_tx(unsigned_tx, block_number).await
}

/// Load a transaction by hash, simulate it with correct pre-state, and return a processed transaction.
pub async fn process_tx_by_hash(
    simulator: &TxSimulator,
    tx_hash: B256,
) -> Result<ProcessedTransaction> {
    let provider = ProcessedTxProvider::with_provider_factory(simulator.provider_factory().clone())?;
    provider.process_transaction_by_hash(tx_hash).await
}
