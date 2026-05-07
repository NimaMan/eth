use std::sync::Arc;

use eth_live_feed::LiveTokenRuntimeConfig;
use eyre::{eyre, Result};
use reth_chain_query::RethQueryProvider;

use crate::config::TokenServerConfig;
use crate::historical::RunManager;
use crate::live::LiveTracker;
use crate::processed_block_cache::TokenProcessedBlockCacheStore;

#[derive(Clone)]
pub struct ServerState {
    pub config: TokenServerConfig,
    pub runs: RunManager,
    pub live_tracker: LiveTracker,
    pub processed_block_cache: Option<Arc<TokenProcessedBlockCacheStore>>,
}

impl ServerState {
    pub fn new(config: TokenServerConfig) -> Result<Self> {
        let datadir = config
            .reth_datadir
            .to_str()
            .ok_or_else(|| eyre!("RETH_DATADIR is not valid UTF-8"))?;
        let provider = Arc::new(RethQueryProvider::new(datadir)?);
        let processed_block_cache = match config.processed_block_cache_dir.as_ref() {
            Some(path) => Some(Arc::new(TokenProcessedBlockCacheStore::open(path)?)),
            None => None,
        };
        let runs = RunManager::new(
            config.clone(),
            provider.clone(),
            processed_block_cache.clone(),
        );
        let live_tracker = LiveTracker::new(
            live_runtime_config(&config),
            provider,
            processed_block_cache.clone(),
        );

        Ok(Self {
            config,
            runs,
            live_tracker,
            processed_block_cache,
        })
    }
}

fn live_runtime_config(config: &TokenServerConfig) -> LiveTokenRuntimeConfig {
    LiveTokenRuntimeConfig {
        history_limit: config.history_limit,
        max_blocks: config.max_blocks,
        default_warmup_blocks: config.default_blocks,
        redis_url: config.redis_url.clone(),
        live_block_stream: config.live_block_stream.clone(),
        cache_retry_attempts: config.live_cache_retry_attempts,
        cache_retry_delay_ms: config.live_cache_retry_delay_ms,
        stream_block_ms: config.live_stream_block_ms,
        stream_count: config.live_stream_count,
    }
}
