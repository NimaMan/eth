use std::collections::BTreeMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tx_processor::{
    BlockStateSession, LivePoolBuySellSimulator, PoolBuySellSimulator, ProcessedBlock,
};

use crate::chain_metadata::{
    TokenDiscoveryProvider, TokenMetadataProvider, UniswapV2PoolMetadataProvider,
};
use crate::network::graph::RawTokenNetworkGraph;
use crate::tracking::token_update_router::PoolTradingSimulationMode;
use crate::tracking::{
    hash_string, LiveTokenRetentionPolicy, ProcessedTokenUpdateRouter, TokenBlockUpdateReport,
    TokenRegistry, TokenTransactionUpdateError, TrackedTokenIndex,
};

pub const DEFAULT_TRACKED_TOKEN_INDEX_SIZE: usize = 2000;

fn default_network_graphs_enabled() -> bool {
    true
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BlockTokenProcessor {
    #[serde(default)]
    pub is_live_mode: bool,
    pub registry: TokenRegistry,
    pub update_router: ProcessedTokenUpdateRouter,
    pub token_index: TrackedTokenIndex,
    #[serde(default = "default_network_graphs_enabled")]
    pub network_graphs_enabled: bool,
    #[serde(default)]
    pub network_graphs: BTreeMap<String, RawTokenNetworkGraph>,
    pub processed_blocks: BTreeMap<u64, bool>,
    pub latest_processed_block: Option<u64>,
    pub start_block: Option<u64>,
    pub updated_token_addresses: Vec<String>,
    pub last_block_failure_count: usize,
}

impl BlockTokenProcessor {
    pub fn new(history_limit: usize) -> Self {
        Self::new_with_token_index_limit(history_limit, Some(DEFAULT_TRACKED_TOKEN_INDEX_SIZE))
    }

    pub fn new_unbounded_token_index(history_limit: usize) -> Self {
        Self::new_with_token_index_limit(history_limit, None)
    }

    pub fn new_with_token_index_limit(
        history_limit: usize,
        token_index_limit: Option<usize>,
    ) -> Self {
        Self {
            is_live_mode: false,
            registry: TokenRegistry::new(),
            update_router: ProcessedTokenUpdateRouter::new(history_limit),
            token_index: token_index_with_limit(token_index_limit),
            network_graphs_enabled: true,
            network_graphs: BTreeMap::new(),
            processed_blocks: BTreeMap::new(),
            latest_processed_block: None,
            start_block: None,
            updated_token_addresses: Vec::new(),
            last_block_failure_count: 0,
        }
    }

    pub fn with_registry(registry: TokenRegistry, history_limit: usize) -> Self {
        let update_router = ProcessedTokenUpdateRouter::new(history_limit);
        Self::with_registry_and_update_router(registry, update_router)
    }

    pub fn with_registry_and_update_router(
        registry: TokenRegistry,
        update_router: ProcessedTokenUpdateRouter,
    ) -> Self {
        let token_index =
            TrackedTokenIndex::from_registry(&registry, DEFAULT_TRACKED_TOKEN_INDEX_SIZE);
        Self {
            is_live_mode: false,
            registry,
            update_router,
            token_index,
            network_graphs_enabled: true,
            network_graphs: BTreeMap::new(),
            processed_blocks: BTreeMap::new(),
            latest_processed_block: None,
            start_block: None,
            updated_token_addresses: Vec::new(),
            last_block_failure_count: 0,
        }
    }

    pub fn set_live_mode(&mut self, is_live_mode: bool) {
        self.is_live_mode = is_live_mode;
        self.registry.set_live_mode(is_live_mode);
        if is_live_mode {
            if self.token_index.live_retention_policy().is_none() {
                self.token_index
                    .set_live_retention_policy(Some(LiveTokenRetentionPolicy::default()));
            }
        } else {
            self.token_index.set_live_retention_policy(None);
        }
    }

    pub fn disable_network_graphs(&mut self) {
        self.network_graphs_enabled = false;
        self.network_graphs.clear();
    }

    pub async fn process_block(
        &mut self,
        block: &ProcessedBlock,
        pool_simulator: &PoolBuySellSimulator,
    ) -> TokenBlockUpdateReport {
        if self.is_live_mode {
            return self.live_mode_historical_simulator_report(block, "process_block");
        }

        let block_session = Mutex::new(None);
        let trading_simulation = PoolTradingSimulationMode::HistoricalBlockSession {
            pool_simulator,
            block_session: &block_session,
            profile_run_id: None,
        };

        self.process_block_with_trading_simulation(block, trading_simulation)
            .await
    }

    pub(crate) async fn process_block_with_live_pool_simulator(
        &mut self,
        block: &ProcessedBlock,
        pool_simulator: &LivePoolBuySellSimulator,
    ) -> TokenBlockUpdateReport {
        let block_sessions: Mutex<BTreeMap<u64, BlockStateSession>> = Mutex::new(BTreeMap::new());
        self.process_block_with_trading_simulation(
            block,
            PoolTradingSimulationMode::LiveBlockSession {
                pool_simulator,
                block_sessions: &block_sessions,
                direct_state_only: false,
                profile_run_id: Some("live"),
            },
        )
        .await
    }

    #[cfg(test)]
    pub(crate) async fn process_block_with_test_simulator(
        &mut self,
        block: &ProcessedBlock,
    ) -> TokenBlockUpdateReport {
        self.process_block_with_trading_simulation(block, PoolTradingSimulationMode::Noop)
            .await
    }

    pub async fn process_block_with_metadata_provider<P>(
        &mut self,
        block: &ProcessedBlock,
        metadata_provider: &P,
        pool_simulator: &PoolBuySellSimulator,
    ) -> TokenBlockUpdateReport
    where
        P: TokenMetadataProvider,
    {
        if self.is_live_mode {
            return self.live_mode_historical_simulator_report(
                block,
                "process_block_with_metadata_provider",
            );
        }

        let block_session = Mutex::new(None);
        let trading_simulation = PoolTradingSimulationMode::HistoricalBlockSession {
            pool_simulator,
            block_session: &block_session,
            profile_run_id: None,
        };

        self.process_block_with_metadata_provider_and_trading_simulation(
            block,
            metadata_provider,
            trading_simulation,
        )
        .await
    }

    pub(crate) async fn process_block_with_metadata_provider_and_live_pool_simulator<P>(
        &mut self,
        block: &ProcessedBlock,
        metadata_provider: &P,
        pool_simulator: &LivePoolBuySellSimulator,
    ) -> TokenBlockUpdateReport
    where
        P: TokenMetadataProvider,
    {
        let block_sessions: Mutex<BTreeMap<u64, BlockStateSession>> = Mutex::new(BTreeMap::new());
        self.process_block_with_metadata_provider_and_trading_simulation(
            block,
            metadata_provider,
            PoolTradingSimulationMode::LiveBlockSession {
                pool_simulator,
                block_sessions: &block_sessions,
                direct_state_only: false,
                profile_run_id: Some("live"),
            },
        )
        .await
    }

    #[cfg(test)]
    pub(crate) async fn process_block_with_metadata_provider_test_simulator<P>(
        &mut self,
        block: &ProcessedBlock,
        metadata_provider: &P,
    ) -> TokenBlockUpdateReport
    where
        P: TokenMetadataProvider,
    {
        self.process_block_with_metadata_provider_and_trading_simulation(
            block,
            metadata_provider,
            PoolTradingSimulationMode::Noop,
        )
        .await
    }
    pub async fn process_block_with_discovery_provider<P>(
        &mut self,
        block: &ProcessedBlock,
        discovery_provider: &P,
        pool_simulator: &PoolBuySellSimulator,
    ) -> TokenBlockUpdateReport
    where
        P: TokenDiscoveryProvider,
    {
        self.process_block_with_discovery_provider_and_profile_run_id(
            block,
            discovery_provider,
            pool_simulator,
            None,
        )
        .await
    }

    pub async fn process_block_with_discovery_provider_and_profile_run_id<P>(
        &mut self,
        block: &ProcessedBlock,
        discovery_provider: &P,
        pool_simulator: &PoolBuySellSimulator,
        profile_run_id: Option<&str>,
    ) -> TokenBlockUpdateReport
    where
        P: TokenDiscoveryProvider,
    {
        if self.is_live_mode {
            return self.live_mode_historical_simulator_report(
                block,
                "process_block_with_discovery_provider",
            );
        }

        let block_session = Mutex::new(None);
        let trading_simulation = PoolTradingSimulationMode::HistoricalBlockSession {
            pool_simulator,
            block_session: &block_session,
            profile_run_id,
        };

        self.process_block_with_token_and_pool_discovery_providers_and_trading_simulation(
            block,
            discovery_provider,
            discovery_provider,
            trading_simulation,
        )
        .await
    }

    pub(crate) async fn process_block_with_discovery_provider_and_live_pool_simulator<P>(
        &mut self,
        block: &ProcessedBlock,
        discovery_provider: &P,
        pool_simulator: &LivePoolBuySellSimulator,
    ) -> TokenBlockUpdateReport
    where
        P: TokenDiscoveryProvider,
    {
        let block_sessions: Mutex<BTreeMap<u64, BlockStateSession>> = Mutex::new(BTreeMap::new());
        self.process_block_with_discovery_provider_and_live_pool_simulator_sessions(
            block,
            discovery_provider,
            pool_simulator,
            &block_sessions,
            false,
        )
        .await
    }

    pub(crate) async fn process_block_with_discovery_provider_and_live_pool_simulator_sessions<P>(
        &mut self,
        block: &ProcessedBlock,
        discovery_provider: &P,
        pool_simulator: &LivePoolBuySellSimulator,
        block_sessions: &Mutex<BTreeMap<u64, BlockStateSession>>,
        direct_state_only: bool,
    ) -> TokenBlockUpdateReport
    where
        P: TokenDiscoveryProvider,
    {
        self.process_block_with_token_and_pool_discovery_providers_and_trading_simulation(
            block,
            discovery_provider,
            discovery_provider,
            PoolTradingSimulationMode::LiveBlockSession {
                pool_simulator,
                block_sessions,
                direct_state_only,
                profile_run_id: Some("live"),
            },
        )
        .await
    }

    pub async fn process_block_with_token_and_pool_discovery_providers<T, V>(
        &mut self,
        block: &ProcessedBlock,
        metadata_provider: &T,
        pool_metadata_provider: &V,
        pool_simulator: &PoolBuySellSimulator,
    ) -> TokenBlockUpdateReport
    where
        T: TokenMetadataProvider,
        V: UniswapV2PoolMetadataProvider,
    {
        if self.is_live_mode {
            return self.live_mode_historical_simulator_report(
                block,
                "process_block_with_token_and_pool_discovery_providers",
            );
        }

        let block_session = Mutex::new(None);
        let trading_simulation = PoolTradingSimulationMode::HistoricalBlockSession {
            pool_simulator,
            block_session: &block_session,
            profile_run_id: None,
        };

        self.process_block_with_token_and_pool_discovery_providers_and_trading_simulation(
            block,
            metadata_provider,
            pool_metadata_provider,
            trading_simulation,
        )
        .await
    }

    pub(crate) async fn process_block_with_token_and_pool_discovery_providers_and_live_pool_simulator<
        T,
        V,
    >(
        &mut self,
        block: &ProcessedBlock,
        metadata_provider: &T,
        pool_metadata_provider: &V,
        pool_simulator: &LivePoolBuySellSimulator,
    ) -> TokenBlockUpdateReport
    where
        T: TokenMetadataProvider,
        V: UniswapV2PoolMetadataProvider,
    {
        let block_sessions: Mutex<BTreeMap<u64, BlockStateSession>> = Mutex::new(BTreeMap::new());
        self.process_block_with_token_and_pool_discovery_providers_and_trading_simulation(
            block,
            metadata_provider,
            pool_metadata_provider,
            PoolTradingSimulationMode::LiveBlockSession {
                pool_simulator,
                block_sessions: &block_sessions,
                direct_state_only: false,
                profile_run_id: Some("live"),
            },
        )
        .await
    }

    #[cfg(test)]
    pub(crate) async fn process_block_with_token_and_pool_discovery_providers_test_simulator<T, V>(
        &mut self,
        block: &ProcessedBlock,
        metadata_provider: &T,
        pool_metadata_provider: &V,
    ) -> TokenBlockUpdateReport
    where
        T: TokenMetadataProvider,
        V: UniswapV2PoolMetadataProvider,
    {
        self.process_block_with_token_and_pool_discovery_providers_and_trading_simulation(
            block,
            metadata_provider,
            pool_metadata_provider,
            PoolTradingSimulationMode::Noop,
        )
        .await
    }

    pub(super) fn live_mode_historical_simulator_report(
        &mut self,
        block: &ProcessedBlock,
        entrypoint: &'static str,
    ) -> TokenBlockUpdateReport {
        self.updated_token_addresses.clear();
        self.last_block_failure_count = 1;

        TokenBlockUpdateReport {
            block_number: block.header.number,
            block_hash: hash_string(&block.header.hash),
            block_timestamp: block.header.timestamp,
            transaction_count: block.transactions.len(),
            processed_transaction_count: 0,
            failed_transaction_count: self.last_block_failure_count,
            already_processed: false,
            created_token_addresses: Vec::new(),
            updated_token_addresses: Vec::new(),
            token_updates: Vec::new(),
            transaction_errors: vec![TokenTransactionUpdateError {
                tx_hash: hash_string(&block.header.hash),
                tx_index: 0,
                message: format!(
                    "{entrypoint} cannot run in live mode with PoolBuySellSimulator; use LiveBlockTokenProcessor with LivePoolBuySellSimulator"
                ),
            }],
        }
    }
}

fn token_index_with_limit(limit: Option<usize>) -> TrackedTokenIndex {
    match limit {
        Some(limit) => TrackedTokenIndex::new(limit),
        None => TrackedTokenIndex::unbounded(),
    }
}
