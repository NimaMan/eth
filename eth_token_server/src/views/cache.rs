use serde::Serialize;

use crate::range_indexer::progress::now_unix_secs;
use tx_processor::{ProcessedBlockDiskCacheCoverage, ProcessedBlockDiskCacheStore};

#[derive(Clone, Debug, Serialize)]
pub struct ProcessedBlockDiskCacheCoverageResponse {
    pub enabled: bool,
    pub generated_at_unix_secs: u64,
    pub retained_block_limit: Option<u64>,
    pub coverage: Option<ProcessedBlockDiskCacheCoverage>,
}

pub fn coverage(
    cache_store: Option<&ProcessedBlockDiskCacheStore>,
    retained_block_limit: Option<u64>,
) -> eyre::Result<ProcessedBlockDiskCacheCoverageResponse> {
    let coverage = match cache_store {
        Some(cache_store) => Some(cache_store.coverage()?),
        None => None,
    };

    Ok(ProcessedBlockDiskCacheCoverageResponse {
        enabled: coverage.is_some(),
        generated_at_unix_secs: now_unix_secs(),
        retained_block_limit,
        coverage,
    })
}
