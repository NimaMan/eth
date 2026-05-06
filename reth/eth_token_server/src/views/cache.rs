use serde::Serialize;

use crate::processed_block_cache::{
    TokenProcessedBlockCacheCoverage, TokenProcessedBlockCacheStore,
};
use crate::runs::progress::now_unix_secs;

#[derive(Clone, Debug, Serialize)]
pub struct CacheCoverageResponse {
    pub enabled: bool,
    pub generated_at_unix_secs: u64,
    pub retained_block_limit: Option<u64>,
    pub coverage: Option<TokenProcessedBlockCacheCoverage>,
}

pub fn coverage(
    cache_store: Option<&TokenProcessedBlockCacheStore>,
    retained_block_limit: Option<u64>,
) -> eyre::Result<CacheCoverageResponse> {
    let coverage = match cache_store {
        Some(cache_store) => Some(cache_store.coverage()?),
        None => None,
    };

    Ok(CacheCoverageResponse {
        enabled: coverage.is_some(),
        generated_at_unix_secs: now_unix_secs(),
        retained_block_limit,
        coverage,
    })
}
