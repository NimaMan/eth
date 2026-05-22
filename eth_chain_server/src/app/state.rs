use std::sync::Arc;

use eth_live_feed::{LiveTokenEvent, LiveTokenReader, LiveTokenRuntimeConfig};
use eyre::{eyre, Result};
use reth_chain_query::{reth_index::RethIndexDB, RethQueryProvider};
use token_lab_scam_risk_atlas::RiskAtlasReader;

use crate::app::config::ChainServerConfig;
use crate::live::{LiveChainRuntime, LiveChainRuntimeConfig, LiveTracker};
use crate::prices::ChainPriceService;
use crate::ranges::RangeIndexManager;
use crate::recent_blocks::RecentLiveBlocks;
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
        let live_chain_runtime = LiveChainRuntime::new(
            live_chain_runtime_config(&config),
            provider.clone(),
            live_tracker.clone(),
            processed_block_replay_store.clone(),
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
        let risk_atlas = RiskAtlasReader::connect_lazy(&config.alpha_database_url)?;
        let price_service = ChainPriceService::new(provider.provider_factory().clone())?;

        Ok(Self {
            config,
            provider,
            range_indexer,
            live_tracker,
            live_chain_runtime,
            token_network_analytics,
            recent_live_blocks,
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
