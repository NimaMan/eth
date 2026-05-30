//! Processed transaction access.
//!
//! This module owns processed data providers at transaction and block granularity:
//! direct transaction processing, address/token filtered transaction views, and
//! processed block/range loading for callers that need batches of transactions.

pub mod address;
pub mod block;
pub mod cache;
pub mod core;
pub mod provider;
pub mod token;

pub use address::AddressProcessedTxProvider;
pub use block::{
    bench_deserialize_block, bench_serialize_block, load_cached_processed_block,
    load_processed_block, load_processed_block_range, load_processed_block_range_with_options,
    prune_processed_block_disk_cache, should_prune_processed_block_disk_cache, CacheFieldSet,
    CacheSerCodec, CompactProcessedTransaction, LoadedProcessedBlock,
    LoadedProcessedBlockWithMetrics, ProcessedBlockAddressIndexWrite,
    ProcessedBlockDiskCacheBlockRange, ProcessedBlockDiskCacheChainCoverage,
    ProcessedBlockDiskCacheCoverage, ProcessedBlockDiskCacheKey, ProcessedBlockDiskCacheRangePlan,
    ProcessedBlockDiskCacheRead, ProcessedBlockDiskCacheReader, ProcessedBlockDiskCacheStore,
    ProcessedBlockDiskCacheWrite, ProcessedBlockDiskCacheWriter, ProcessedBlockLoadMetrics,
    ProcessedBlockProvider, ProcessedBlockRangeLoadOptions, ProcessedBlockReplayStoreWrite,
    ProcessedBlockReplayStoreWriter, DEFAULT_PROCESSED_BLOCK_DISK_CACHE_FILL_BATCH_BLOCKS,
    DEFAULT_PROCESSED_BLOCK_DISK_CACHE_FILL_CONCURRENCY, DEFAULT_PROCESSED_BLOCK_RANGE_READ_BATCH,
};
pub use cache::{
    processed_block_trace_config_hash, ProcessedBlockCacheKey, ProcessedBlockCacheStore,
};
pub use provider::ProcessedTxProvider;
pub use token::TokenProcessedTxProvider;
