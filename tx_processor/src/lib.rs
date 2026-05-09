/// Clean TX Processor - Rust alternative to Python eth_block_processor
///
/// This is a simplified, clean transaction processor that replaces the complex
/// Python eth_block_processor.tx module with direct Reth database access.
///
/// Key improvements over Python version:
/// - 10-40x faster (direct DB vs RPC)
/// - Much simpler codebase
/// - No complex RPC handling
/// - Consistent performance
// Re-export the NEW TX Simulator types (NO CallRequest - uses UnsignedTransaction)
pub use tx_simulator::{
    ChainStateInfo, FullSimulationResult, SequentialSimulationOptions, SequentialSimulationResult,
    SequentialTransactionResult, SimulationResult, TxSimulator, UnsignedTransaction,
    UnsignedTxChainSimulation,
};

// Export transaction processing modules
pub mod block_processor;
pub mod live;
pub mod processed_block_provider;
pub mod processed_tx_provider;
pub mod simulator;
pub mod tx_builder;
pub mod tx_processor;

// Re-export data models from tx_processor
pub use block_processor::{
    BlockBatchOptions, BlockProcessor, CachedProcessedBlock, PersistentProcessedBlockCacheMode,
    ProcessedBlock, ProcessedBlockSource, ProcessedBlockTransactions,
    PROCESSED_BLOCK_SCHEMA_VERSION,
};
pub use live::{
    LiveAddressBlockParticipationIndexWorker, LiveBlockProcessor, LiveBlockProcessorConfig,
    LiveBlockService, LiveProcessedBlock,
};
pub use processed_block_provider::{
    load_cached_processed_block_with_retry, load_processed_block, load_processed_block_range,
    prune_processed_block_disk_cache, should_prune_processed_block_disk_cache,
    CompactProcessedTransaction, LiveProcessedBlockProvider, LoadedProcessedBlock,
    LoadedProcessedBlockWithMetrics, ProcessedBlockDiskCacheBlockRange,
    ProcessedBlockDiskCacheChainCoverage, ProcessedBlockDiskCacheCoverage,
    ProcessedBlockDiskCacheKey, ProcessedBlockDiskCacheRangePlan, ProcessedBlockDiskCacheRead,
    ProcessedBlockDiskCacheReader, ProcessedBlockDiskCacheStore, ProcessedBlockDiskCacheWrite,
    ProcessedBlockDiskCacheWriter, ProcessedBlockLoadMetrics, ProcessedBlockProviderRetry,
    COMPACT_PROCESSED_TRANSACTION_SCHEMA_VERSION, DEFAULT_PROCESSED_BLOCK_RANGE_READ_BATCH,
};
pub use processed_tx_provider::{
    processed_block_trace_config_hash, AddressProcessedTxProvider, ProcessedBlockCacheKey,
    ProcessedBlockCacheStore, ProcessedTxProvider, TokenProcessedTxProvider,
};
pub use tx_processor::data_models::{ProcessedTransaction, TransactionFees};
// Export ERC20 token buy-approve-sell simulator through simulator module
pub use simulator::{
    check_can_buy_sell_pool, simulate_buy_swap, simulate_sell_swap, BuySwapResult,
    LivePoolBuySellSimulator, OptionalSetupBuyApproveSellResult, PoolBuySellParameters,
    PoolBuySellSimulationResult, PoolBuySellSimulator, PoolType, SellSwapResult,
};
pub use tx_builder::{SignedTxBuilder, UnsignedTxBuilder};

// Convenience facade: simulate and return ProcessedTransaction directly
use alloy_primitives::B256;
use eyre::Result;
use reth_primitives_traits::SealedHeader;

/// Simulate an unsigned transaction at a block (or latest) and return a fully processed transaction.
pub async fn process_unsigned_tx(
    simulator: &TxSimulator,
    unsigned_tx: UnsignedTransaction,
    block_number: Option<u64>,
) -> Result<ProcessedTransaction> {
    let provider =
        ProcessedTxProvider::with_provider_factory(simulator.provider_factory().clone())?;
    provider
        .process_transaction_from_unsigned_tx(unsigned_tx, block_number)
        .await
}

/// Simulate an unsigned transaction using a provided block header snapshot
pub async fn process_unsigned_tx_with_header(
    simulator: &TxSimulator,
    unsigned_tx: UnsignedTransaction,
    block_header: SealedHeader,
) -> Result<ProcessedTransaction> {
    let provider =
        ProcessedTxProvider::with_provider_factory(simulator.provider_factory().clone())?;
    provider
        .process_transaction_from_unsigned_tx_with_header(unsigned_tx, block_header)
        .await
}

/// Load a transaction by hash, simulate it with correct pre-state, and return a processed transaction.
pub async fn process_tx_by_hash(
    simulator: &TxSimulator,
    tx_hash: B256,
) -> Result<ProcessedTransaction> {
    let provider =
        ProcessedTxProvider::with_provider_factory(simulator.provider_factory().clone())?;
    provider.process_transaction_by_hash(tx_hash).await
}
