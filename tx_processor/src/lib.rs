/// Ethereum transaction and block processing on top of local Reth data and
/// tx_simulator execution traces.
pub use tx_simulator::{
    BlockStateSession, BlockTxStateSession, ChainStateInfo, FullSimulationResult,
    SequentialSimulationOptions, SequentialSimulationResult, SequentialTransactionResult,
    SimulationResult, TxSimulator, UnsignedTransaction, UnsignedTxChainSimulation,
};

// Export transaction processing modules
pub mod address_block_participation;
pub mod block_processor;
pub mod live;
pub mod processed_tx_builder;
pub mod processed_tx_provider;
pub mod trade_simulation;
pub mod tx_processor;

// Re-export data models from tx_processor
pub use address_block_participation::address_participations_from_processed_block;
pub use block_processor::{
    sealed_header_from_processed_block_header, BlockBatchOptions, BlockProcessor,
    CachedProcessedBlock, PersistentProcessedBlockCacheMode, ProcessedBlock, ProcessedBlockSource,
    ProcessedBlockTransactions,
};
pub use live::{
    LiveBlockProcessor, LiveBlockProcessorConfig, LiveBlockStateFrame, LiveProcessedBlock,
    LiveStateDiffFrame,
};
pub use processed_tx_provider::{
    load_cached_processed_block, load_processed_block, load_processed_block_range,
    load_processed_block_range_with_options, processed_block_trace_config_hash,
    prune_processed_block_disk_cache, should_prune_processed_block_disk_cache,
    AddressProcessedTxProvider, CompactProcessedTransaction, LoadedProcessedBlock,
    LoadedProcessedBlockWithMetrics, ProcessedBlockAddressIndexWrite, ProcessedBlockCacheKey,
    ProcessedBlockCacheStore, ProcessedBlockDiskCacheBlockRange,
    ProcessedBlockDiskCacheChainCoverage, ProcessedBlockDiskCacheCoverage,
    ProcessedBlockDiskCacheKey, ProcessedBlockDiskCacheRangePlan, ProcessedBlockDiskCacheRead,
    ProcessedBlockDiskCacheReader, ProcessedBlockDiskCacheStore, ProcessedBlockDiskCacheWrite,
    ProcessedBlockDiskCacheWriter, ProcessedBlockLoadMetrics, ProcessedBlockProvider,
    ProcessedBlockRangeLoadOptions, ProcessedBlockReplayStoreWrite,
    ProcessedBlockReplayStoreWriter, ProcessedTxProvider, TokenProcessedTxProvider,
    DEFAULT_PROCESSED_BLOCK_DISK_CACHE_FILL_BATCH_BLOCKS,
    DEFAULT_PROCESSED_BLOCK_DISK_CACHE_FILL_CONCURRENCY, DEFAULT_PROCESSED_BLOCK_RANGE_READ_BATCH,
};
pub use tx_processor::data_models::{
    Erc20CallKind, InternalErc20Call, InternalErc20Transfer, ProcessedTransaction, TransactionFees,
};
// Export ERC20 token buy-approve-sell simulator through simulator module
pub use processed_tx_builder::{SignedTxBuilder, UnsignedTxBuilder};
pub use trade_simulation::{
    check_can_buy_sell_pool, check_can_buy_sell_pool_with_chain, simulate_buy_swap,
    simulate_buy_swap_with_params, simulate_buy_swap_with_params_and_chain, simulate_sell_swap,
    simulate_sell_swap_with_params, simulate_sell_swap_with_params_and_chain, BuySwapResult,
    LivePoolBuySellSimulator, OptionalSetupBuyApproveSellResult, PoolBuySellParameters,
    PoolBuySellSimulationResult, PoolBuySellSimulator, PoolType, SellSwapResult,
    UniswapV4PoolConfig,
};

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
