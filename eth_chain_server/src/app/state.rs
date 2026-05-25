use std::sync::Arc;

use eth_live_feed::{LiveTokenEvent, LiveTokenReader, LiveTokenRuntimeConfig};
use eth_risk_atlas::RiskAtlasReader;
use eyre::{eyre, Result, WrapErr};
use reth_chain_query::{reth_index::RethIndexDB, RethQueryProvider};

use crate::app::config::ChainServerConfig;
use crate::live::{LiveChainRuntime, LiveChainRuntimeConfig, LiveTracker};
use crate::prices::ChainPriceService;
use crate::ranges::RangeIndexManager;
use crate::recent_blocks::{
    mined_fee_sample_from_processed_block, RecentLiveBlocks, RecentLiveFeeSamples,
    RecentLiveStateFrames,
};
use crate::stores::alpha_trading::AlphaTradingStore;
use crate::stores::mempool_signals::MempoolSignalStore;
use crate::token_analytics::network::TokenNetworkAnalysisManager;
use reth_chain_query::reth_index::AddressBlockParticipationWriter;
use tx_processor::{ProcessedBlockDiskCacheStore, ProcessedBlockReplayStoreWriter};

#[derive(Clone)]
pub struct ServerState {
    pub config: ChainServerConfig,
    pub provider: Arc<RethQueryProvider>,
    pub range_indexer: RangeIndexManager,
    pub live_tracker: LiveTracker,
    pub live_chain_runtime: LiveChainRuntime,
    pub token_network_analytics: TokenNetworkAnalysisManager,
    pub recent_live_blocks: RecentLiveBlocks,
    pub recent_live_fee_samples: RecentLiveFeeSamples,
    pub recent_live_state_frames: RecentLiveStateFrames,
    pub processed_block_disk_cache: Option<Arc<ProcessedBlockDiskCacheStore>>,
    pub processed_block_replay_store: Option<Arc<ProcessedBlockReplayStoreWriter>>,
    pub mempool_signals: MempoolSignalStore,
    pub alpha_trading: AlphaTradingStore,
    pub risk_atlas: RiskAtlasReader,
    pub price_service: ChainPriceService,
}

impl ServerState {
    pub fn new(config: ChainServerConfig) -> Result<Self> {
        let datadir = config
            .reth_datadir
            .to_str()
            .ok_or_else(|| eyre!("RETH_DATADIR is not valid UTF-8"))?;
        let reth_index = open_reth_index_handles(
            config.reth_index_dir.as_deref(),
            config.processed_block_disk_cache_dir.is_some(),
        );
        let provider = attach_reth_index(RethQueryProvider::new(datadir)?, reth_index.reader());
        let provider = Arc::new(provider);
        let (processed_block_disk_cache, processed_block_replay_store) =
            match config.processed_block_disk_cache_dir.as_ref() {
                Some(path) => {
                    let store = ProcessedBlockDiskCacheStore::open(path)?;
                    let writer = ProcessedBlockReplayStoreWriter::new(
                        store.clone(),
                        provider.chain_id(),
                        reth_index.address_writer(),
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
        let recent_live_fee_samples = RecentLiveFeeSamples::new(config.history_limit.max(128));
        let recent_live_state_frames = RecentLiveStateFrames::new(16);
        let live_chain_runtime = LiveChainRuntime::new(
            live_chain_runtime_config(&config),
            provider.clone(),
            live_tracker.clone(),
            processed_block_replay_store.clone(),
            recent_live_fee_samples.clone(),
            recent_live_state_frames.clone(),
        );
        let token_network_analytics = TokenNetworkAnalysisManager::new(
            provider.clone(),
            processed_block_replay_store.clone(),
        );
        let recent_live_blocks = RecentLiveBlocks::new(config.history_limit.max(128));
        let mempool_signals =
            MempoolSignalStore::new(&config.mempool_database_url, config.mempool_signal_limit)?
                .with_arrival_provider(provider.clone());
        let alpha_trading = AlphaTradingStore::new(&config.alpha_database_url)?;
        let risk_atlas = RiskAtlasReader::connect_lazy(&config.risk_atlas_database_url)?;
        let price_service = ChainPriceService::new(provider.provider_factory().clone())?;

        Ok(Self {
            config,
            provider,
            range_indexer,
            live_tracker,
            live_chain_runtime,
            token_network_analytics,
            recent_live_blocks,
            recent_live_fee_samples,
            recent_live_state_frames,
            processed_block_disk_cache,
            processed_block_replay_store,
            mempool_signals,
            alpha_trading,
            risk_atlas,
            price_service,
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

    pub fn spawn_recent_live_fee_sample_recorder(&self) {
        let Some(replay_store) = self.processed_block_replay_store.clone() else {
            tracing::warn!(
                "processed block disk cache disabled; gas-rank live fee samples cannot backfill from warmup"
            );
            return;
        };
        let mut events = self.live_tracker.subscribe();
        let recent_live_fee_samples = self.recent_live_fee_samples.clone();
        let chain_id = self.provider.chain_id();
        let gas_rank_seed_blocks = 100usize;

        tokio::spawn(async move {
            loop {
                match events.recv().await {
                    Ok(LiveTokenEvent::BlockApplied { block_number, .. }) => {
                        let replay_store = replay_store.clone();
                        let recent_live_fee_samples = recent_live_fee_samples.clone();
                        match tokio::task::spawn_blocking(move || {
                            record_fee_sample_from_cache(
                                replay_store.as_ref(),
                                &recent_live_fee_samples,
                                chain_id,
                                block_number,
                            )
                        })
                        .await
                        {
                            Ok(Ok(true)) => {}
                            Ok(Ok(false)) => {
                                tracing::debug!(
                                    block_number,
                                    "processed block cache miss while recording gas-rank sample"
                                );
                            }
                            Ok(Err(error)) => {
                                tracing::warn!(
                                    block_number,
                                    error = %error,
                                    "failed to record gas-rank sample from processed block cache"
                                );
                            }
                            Err(error) => {
                                tracing::warn!(
                                    block_number,
                                    error = %error,
                                    "gas-rank sample cache read task failed"
                                );
                            }
                        }
                    }
                    Ok(LiveTokenEvent::RuntimeLive {
                        current_block: Some(current_block),
                        ..
                    }) => {
                        let replay_store = replay_store.clone();
                        let recent_live_fee_samples = recent_live_fee_samples.clone();
                        match tokio::task::spawn_blocking(move || {
                            seed_fee_samples_from_cache(
                                replay_store.as_ref(),
                                &recent_live_fee_samples,
                                chain_id,
                                current_block,
                                gas_rank_seed_blocks,
                            )
                        })
                        .await
                        {
                            Ok(Ok(recorded)) => {
                                tracing::info!(
                                    current_block,
                                    recorded,
                                    "seeded gas-rank fee samples from processed block cache"
                                );
                            }
                            Ok(Err(error)) => {
                                tracing::warn!(
                                    current_block,
                                    error = %error,
                                    "failed to seed gas-rank fee samples from processed block cache"
                                );
                            }
                            Err(error) => {
                                tracing::warn!(
                                    current_block,
                                    error = %error,
                                    "gas-rank sample seed task failed"
                                );
                            }
                        }
                    }
                    Ok(_) => {}
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                        tracing::warn!(
                            skipped,
                            "gas-rank fee sample recorder lagged behind live tracker events"
                        );
                        continue;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        });
    }
}

fn seed_fee_samples_from_cache(
    replay_store: &ProcessedBlockReplayStoreWriter,
    recent_live_fee_samples: &RecentLiveFeeSamples,
    chain_id: u64,
    current_block: u64,
    history_limit: usize,
) -> Result<usize> {
    let mut recorded = 0usize;
    let start_block = current_block.saturating_sub(history_limit.saturating_sub(1) as u64);
    for block_number in start_block..=current_block {
        if record_fee_sample_from_cache(
            replay_store,
            recent_live_fee_samples,
            chain_id,
            block_number,
        )? {
            recorded += 1;
        }
    }
    Ok(recorded)
}

fn record_fee_sample_from_cache(
    replay_store: &ProcessedBlockReplayStoreWriter,
    recent_live_fee_samples: &RecentLiveFeeSamples,
    chain_id: u64,
    block_number: u64,
) -> Result<bool> {
    let Some(key) = replay_store
        .disk_cache_store()
        .cached_key_for_block_number(chain_id, block_number)
        .wrap_err_with(|| format!("failed to resolve processed block cache key {block_number}"))?
    else {
        return Ok(false);
    };
    let Some(block) = replay_store
        .disk_cache_store()
        .get(&key)
        .wrap_err_with(|| format!("failed to read processed block cache {block_number}"))?
    else {
        return Ok(false);
    };
    recent_live_fee_samples.record(mined_fee_sample_from_processed_block(&block));
    Ok(true)
}

struct RethIndexHandles {
    reader: Option<Arc<RethIndexDB>>,
    address_writer: Option<AddressBlockParticipationWriter>,
}

impl RethIndexHandles {
    fn none() -> Self {
        Self {
            reader: None,
            address_writer: None,
        }
    }

    fn reader(&self) -> Option<Arc<RethIndexDB>> {
        self.reader.clone()
    }

    fn address_writer(&self) -> Option<AddressBlockParticipationWriter> {
        self.address_writer.clone()
    }
}

fn open_reth_index_handles(
    reth_index_dir: Option<&std::path::Path>,
    needs_address_writer: bool,
) -> RethIndexHandles {
    let Some(reth_index_dir) = reth_index_dir else {
        return RethIndexHandles::none();
    };

    if needs_address_writer {
        match RethIndexDB::open(reth_index_dir) {
            Ok(db) => {
                let db = Arc::new(db);
                return RethIndexHandles {
                    reader: Some(db.clone()),
                    address_writer: Some(AddressBlockParticipationWriter::new(db)),
                };
            }
            Err(error) => {
                tracing::warn!(
                    reth_index_dir = %reth_index_dir.display(),
                    error = %error,
                    "processed block replay store address index writer unavailable; trying read-only reth_index for lookups"
                );
            }
        }
    }

    match RethIndexDB::open_read_only(reth_index_dir) {
        Ok(db) => RethIndexHandles {
            reader: Some(Arc::new(db)),
            address_writer: None,
        },
        Err(error) => {
            tracing::warn!(
                reth_index_dir = %reth_index_dir.display(),
                error = %error,
                "reth_index unavailable; token activity block lookup disabled"
            );
            RethIndexHandles::none()
        }
    }
}

fn attach_reth_index(
    provider: RethQueryProvider,
    reth_index_db: Option<Arc<RethIndexDB>>,
) -> RethQueryProvider {
    match reth_index_db {
        Some(db) => provider.with_reth_index_db(db),
        None => provider,
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
