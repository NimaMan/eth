use std::sync::Arc;

use eth_live_feed::{LiveTokenEvent, LiveTokenReader, LiveTokenRuntimeConfig};
use eyre::{eyre, Result};
use reth_chain_query::{reth_index::RethIndexDB, RethQueryProvider};

use crate::app::config::ChainServerConfig;
use crate::live::{LiveChainRuntime, LiveChainRuntimeConfig, LiveTracker};
use crate::ranges::RangeIndexManager;
use crate::recent_blocks::RecentLiveBlocks;
use crate::stores::alpha_trading::AlphaTradingStore;
use crate::stores::mempool_signals::MempoolSignalStore;
use reth_chain_query::reth_index::AddressBlockParticipationWriter;
use tx_processor::{ProcessedBlockDiskCacheStore, ProcessedBlockReplayStoreWriter};

#[derive(Clone)]
pub struct ServerState {
    pub config: ChainServerConfig,
    pub provider: Arc<RethQueryProvider>,
    pub range_indexer: RangeIndexManager,
    pub live_tracker: LiveTracker,
    pub live_chain_runtime: LiveChainRuntime,
    pub recent_live_blocks: RecentLiveBlocks,
    pub processed_block_disk_cache: Option<Arc<ProcessedBlockDiskCacheStore>>,
    pub processed_block_replay_store: Option<Arc<ProcessedBlockReplayStoreWriter>>,
    pub mempool_signals: MempoolSignalStore,
    pub alpha_trading: AlphaTradingStore,
}

impl ServerState {
    pub fn new(config: ChainServerConfig) -> Result<Self> {
        let datadir = config
            .reth_datadir
            .to_str()
            .ok_or_else(|| eyre!("RETH_DATADIR is not valid UTF-8"))?;
        let provider = attach_reth_index(
            RethQueryProvider::new(datadir)?,
            config.reth_index_dir.as_deref(),
        );
        let provider = Arc::new(provider);
        let (processed_block_disk_cache, processed_block_replay_store) = match config
            .processed_block_disk_cache_dir
            .as_ref()
        {
            Some(path) => {
                let store = ProcessedBlockDiskCacheStore::open(path)?;
                let address_index = match config.reth_index_dir.as_ref() {
                    Some(index_dir) => match RethIndexDB::open(index_dir) {
                        Ok(db) => Some(AddressBlockParticipationWriter::new(Arc::new(db))),
                        Err(error) => {
                            tracing::warn!(
                                reth_index_dir = %index_dir.display(),
                                error = %error,
                                "processed block replay store address index writer unavailable; continuing with disk cache only"
                            );
                            None
                        }
                    },
                    None => None,
                };
                let writer = ProcessedBlockReplayStoreWriter::new(
                    store.clone(),
                    provider.chain_id(),
                    address_index,
                );
                (Some(Arc::new(store)), Some(Arc::new(writer)))
            }
            None => (None, None),
        };
        let range_indexer = RangeIndexManager::new(
            config.clone(),
            provider.clone(),
            processed_block_replay_store.clone(),
        );
        let live_tracker = LiveTracker::new(
            live_runtime_config(&config),
            provider.clone(),
            processed_block_replay_store.clone(),
        );
        let live_chain_runtime = LiveChainRuntime::new(
            live_chain_runtime_config(&config),
            provider.clone(),
            live_tracker.clone(),
            processed_block_replay_store.clone(),
        );
        let recent_live_blocks = RecentLiveBlocks::new(config.history_limit.max(128));
        let mempool_signals =
            MempoolSignalStore::new(&config.mempool_database_url, config.mempool_signal_limit)?;
        let alpha_trading = AlphaTradingStore::new(&config.alpha_database_url)?;

        Ok(Self {
            config,
            provider,
            range_indexer,
            live_tracker,
            live_chain_runtime,
            recent_live_blocks,
            processed_block_disk_cache,
            processed_block_replay_store,
            mempool_signals,
            alpha_trading,
        })
    }

    pub fn spawn_recent_live_block_recorder(&self) {
        let mut events = self.live_tracker.subscribe();
        let recent_live_blocks = self.recent_live_blocks.clone();
        tokio::spawn(async move {
            loop {
                match events.recv().await {
                    Ok(LiveTokenEvent::BlockApplied {
                        block_number,
                        block_hash,
                        ..
                    }) => {
                        recent_live_blocks.record(block_number, block_hash);
                    }
                    Ok(_) => {}
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        });
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

fn live_runtime_config(config: &ChainServerConfig) -> LiveTokenRuntimeConfig {
    LiveTokenRuntimeConfig {
        history_limit: config.history_limit,
        default_warmup_blocks: config.live_warmup_blocks,
        block_apply_timeout_ms: config.live_block_apply_timeout_ms,
    }
}

fn live_chain_runtime_config(config: &ChainServerConfig) -> LiveChainRuntimeConfig {
    LiveChainRuntimeConfig {
        execution_rpc: config.execution_rpc.clone(),
        execution_ws: config.execution_ws.clone(),
    }
}
