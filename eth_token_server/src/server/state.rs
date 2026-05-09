use std::sync::Arc;

use eth_live_feed::LiveTokenRuntimeConfig;
use eyre::{eyre, Result};
use reth_chain_query::{reth_index::RethIndexDB, RethQueryProvider};

use crate::alpha_trading::AlphaTradingStore;
use crate::config::TokenServerConfig;
use crate::live::LiveTracker;
use crate::mempool_signals::MempoolSignalStore;
use crate::range_indexer::RangeIndexManager;
use tx_processor::ProcessedBlockDiskCacheStore;

#[derive(Clone)]
pub struct ServerState {
    pub config: TokenServerConfig,
    pub provider: Arc<RethQueryProvider>,
    pub range_indexer: RangeIndexManager,
    pub live_tracker: LiveTracker,
    pub processed_block_disk_cache: Option<Arc<ProcessedBlockDiskCacheStore>>,
    pub mempool_signals: MempoolSignalStore,
    pub alpha_trading: AlphaTradingStore,
}

impl ServerState {
    pub fn new(config: TokenServerConfig) -> Result<Self> {
        let datadir = config
            .reth_datadir
            .to_str()
            .ok_or_else(|| eyre!("RETH_DATADIR is not valid UTF-8"))?;
        let provider = attach_reth_index(
            RethQueryProvider::new(datadir)?,
            config.reth_index_dir.as_deref(),
        );
        let provider = Arc::new(provider);
        let processed_block_disk_cache = match config.processed_block_disk_cache_dir.as_ref() {
            Some(path) => Some(Arc::new(ProcessedBlockDiskCacheStore::open(path)?)),
            None => None,
        };
        let range_indexer = RangeIndexManager::new(
            config.clone(),
            provider.clone(),
            processed_block_disk_cache.clone(),
        );
        let live_tracker = LiveTracker::new(
            live_runtime_config(&config),
            provider.clone(),
            processed_block_disk_cache.clone(),
        );
        let mempool_signals =
            MempoolSignalStore::new(&config.mempool_database_url, config.mempool_signal_limit)?;
        let alpha_trading = AlphaTradingStore::new(&config.alpha_database_url)?;

        Ok(Self {
            config,
            provider,
            range_indexer,
            live_tracker,
            processed_block_disk_cache,
            mempool_signals,
            alpha_trading,
        })
    }
}

fn attach_reth_index(
    provider: RethQueryProvider,
    reth_index_dir: Option<&std::path::Path>,
) -> RethQueryProvider {
    let Some(reth_index_dir) = reth_index_dir else {
        return provider;
    };

    match RethIndexDB::open_read_only(reth_index_dir) {
        Ok(db) => provider.with_reth_index_db(Arc::new(db)),
        Err(error) => {
            tracing::warn!(
                reth_index_dir = %reth_index_dir.display(),
                error = %error,
                "reth_index unavailable; token activity block lookup disabled"
            );
            provider
        }
    }
}

fn live_runtime_config(config: &TokenServerConfig) -> LiveTokenRuntimeConfig {
    LiveTokenRuntimeConfig {
        history_limit: config.history_limit,
        default_warmup_blocks: config.live_warmup_blocks,
        redis_url: config.redis_url.clone(),
        live_block_stream: config.live_block_stream.clone(),
        processed_block_disk_cache_retry_attempts: config
            .live_processed_block_disk_cache_retry_attempts,
        processed_block_disk_cache_retry_delay_ms: config
            .live_processed_block_disk_cache_retry_delay_ms,
        stream_block_ms: config.live_stream_block_ms,
        stream_count: config.live_stream_count,
        block_apply_timeout_ms: config.live_block_apply_timeout_ms,
    }
}
