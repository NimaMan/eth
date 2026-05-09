use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::time::Duration;

use alloy_primitives::{Address, B256};
use serde::{Deserialize, Serialize};
use tx_processor::{
    LivePoolBuySellSimulator, PoolBuySellSimulator, ProcessedBlock, ProcessedTransaction,
};

use crate::chain_metadata::{
    TokenDiscoveryProvider, TokenMetadataLookup, TokenMetadataProvider,
    UniswapV2PoolMetadataProvider,
};
use crate::network::{
    graph::RawTokenNetworkGraph, ingest::extract_token_network_updates, model::TokenNetworkId,
};

use crate::tracking::replay_context::BlockReplayContext;
use crate::tracking::transaction_applier::V2TradingSimulation;
use crate::tracking::ProcessedTokenUpdateRouter;
use crate::tracking::{
    address_string, hash_string, normalize_address, LiveTokenRetentionPolicy,
    LiveTokenRetentionReport, TokenBlockUpdateReport, TokenRegistry, TokenTransactionUpdateError,
    TrackedTokenIndex, TrackedTokenIndexUpdate, TrackedTokenStatus,
};

pub const DEFAULT_TRACKED_TOKEN_INDEX_SIZE: usize = 2000;
const LIVE_METADATA_LOOKUP_TIMEOUT_MS: u64 = 2_500;
const LIVE_TOKEN_TRACKER_LOG_TARGET: &str = "live_token_tracker";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BlockTokenProcessor {
    #[serde(default)]
    pub is_live_mode: bool,
    pub registry: TokenRegistry,
    pub update_router: ProcessedTokenUpdateRouter,
    pub token_index: TrackedTokenIndex,
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

    pub async fn process_block(
        &mut self,
        block: &ProcessedBlock,
        pool_simulator: &PoolBuySellSimulator,
    ) -> TokenBlockUpdateReport {
        if self.is_live_mode {
            return self.live_mode_historical_simulator_report(block, "process_block");
        }

        self.process_block_with_trading_simulation(
            block,
            V2TradingSimulation::Historical(pool_simulator),
        )
        .await
    }

    pub(crate) async fn process_block_with_live_pool_simulator(
        &mut self,
        block: &ProcessedBlock,
        pool_simulator: &LivePoolBuySellSimulator,
    ) -> TokenBlockUpdateReport {
        self.process_block_with_trading_simulation(block, V2TradingSimulation::Live(pool_simulator))
            .await
    }

    #[cfg(test)]
    pub(crate) async fn process_block_with_test_simulator(
        &mut self,
        block: &ProcessedBlock,
    ) -> TokenBlockUpdateReport {
        self.process_block_with_trading_simulation(block, V2TradingSimulation::Noop)
            .await
    }

    async fn process_block_with_trading_simulation(
        &mut self,
        block: &ProcessedBlock,
        trading_simulation: V2TradingSimulation<'_>,
    ) -> TokenBlockUpdateReport {
        let block_number = block.header.number;
        if self.processed_blocks.contains_key(&block_number) {
            return TokenBlockUpdateReport {
                block_number,
                block_hash: hash_string(&block.header.hash),
                block_timestamp: block.header.timestamp,
                transaction_count: block.transactions.len(),
                processed_transaction_count: 0,
                failed_transaction_count: 0,
                already_processed: true,
                created_token_addresses: Vec::new(),
                updated_token_addresses: Vec::new(),
                token_updates: Vec::new(),
                transaction_errors: Vec::new(),
            };
        }

        self.updated_token_addresses.clear();
        self.last_block_failure_count = 0;

        let mut transactions: Vec<_> = block.transactions.iter().collect();
        transactions.sort_by_key(|tx| tx.processed.tx_index);

        let mut token_updates = Vec::new();
        let mut transaction_errors = Vec::new();
        let created_token_addresses = Vec::new();
        let mut updated_token_addresses = BTreeSet::new();
        let mut processed_transaction_count = 0;
        let mut replay_context = BlockReplayContext::default();

        for tx in transactions {
            if let Some(error) = &tx.processing_error {
                self.last_block_failure_count += 1;
                transaction_errors.push(TokenTransactionUpdateError {
                    tx_hash: hash_string(&tx.processed.hash),
                    tx_index: tx.processed.tx_index,
                    message: error.clone(),
                });
                continue;
            }
            if !should_apply_transaction_to_token_state(&tx.processed) {
                continue;
            }

            let prior_txs = replay_context.prior_txs_for_transaction(&self.registry, &tx.processed);
            match self
                .update_router
                .update_registry_from_processed_transaction_with_trading_simulation(
                    &mut self.registry,
                    &self.token_index,
                    &tx.processed,
                    trading_simulation,
                    &prior_txs,
                    Some(&block.header),
                )
                .await
            {
                Ok(reports) => {
                    processed_transaction_count += 1;
                    let mut transaction_token_addresses = BTreeSet::new();
                    for report in reports {
                        let token_address = report.token_address.clone();
                        transaction_token_addresses.insert(token_address.clone());
                        updated_token_addresses.insert(token_address.clone());
                        self.refresh_token_index(&token_address, block_number);
                        token_updates.push(report);
                    }
                    self.apply_network_updates_for_transaction(
                        &tx.processed,
                        transaction_token_addresses,
                    );
                }
                Err(error) => {
                    self.last_block_failure_count += 1;
                    transaction_errors.push(TokenTransactionUpdateError {
                        tx_hash: hash_string(&tx.processed.hash),
                        tx_index: tx.processed.tx_index,
                        message: error.to_string(),
                    });
                }
            }
            replay_context.observe_transaction(&self.registry, &tx.processed);
        }

        if self.start_block.is_none() {
            self.start_block = Some(block_number);
        }
        self.latest_processed_block = Some(block_number);
        self.processed_blocks.insert(block_number, true);

        self.updated_token_addresses = updated_token_addresses.into_iter().collect();

        TokenBlockUpdateReport {
            block_number,
            block_hash: hash_string(&block.header.hash),
            block_timestamp: block.header.timestamp,
            transaction_count: block.transactions.len(),
            processed_transaction_count,
            failed_transaction_count: self.last_block_failure_count,
            already_processed: false,
            created_token_addresses,
            updated_token_addresses: self.updated_token_addresses.clone(),
            token_updates,
            transaction_errors,
        }
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

        self.process_block_with_metadata_provider_and_trading_simulation(
            block,
            metadata_provider,
            V2TradingSimulation::Historical(pool_simulator),
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
        self.process_block_with_metadata_provider_and_trading_simulation(
            block,
            metadata_provider,
            V2TradingSimulation::Live(pool_simulator),
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
            V2TradingSimulation::Noop,
        )
        .await
    }

    async fn process_block_with_metadata_provider_and_trading_simulation<P>(
        &mut self,
        block: &ProcessedBlock,
        metadata_provider: &P,
        trading_simulation: V2TradingSimulation<'_>,
    ) -> TokenBlockUpdateReport
    where
        P: TokenMetadataProvider,
    {
        let block_number = block.header.number;
        if self.processed_blocks.contains_key(&block_number) {
            return TokenBlockUpdateReport {
                block_number,
                block_hash: hash_string(&block.header.hash),
                block_timestamp: block.header.timestamp,
                transaction_count: block.transactions.len(),
                processed_transaction_count: 0,
                failed_transaction_count: 0,
                already_processed: true,
                created_token_addresses: Vec::new(),
                updated_token_addresses: Vec::new(),
                token_updates: Vec::new(),
                transaction_errors: Vec::new(),
            };
        }

        self.updated_token_addresses.clear();
        self.last_block_failure_count = 0;

        let mut transactions: Vec<_> = block.transactions.iter().collect();
        transactions.sort_by_key(|tx| tx.processed.tx_index);

        let mut token_updates = Vec::new();
        let mut transaction_errors = Vec::new();
        let mut created_token_addresses = BTreeSet::new();
        let mut updated_token_addresses = BTreeSet::new();
        let mut processed_transaction_count = 0;
        let mut metadata_tx_index = HashMap::new();
        let mut replay_context = BlockReplayContext::default();

        for tx in transactions {
            if let Some(error) = &tx.processing_error {
                self.last_block_failure_count += 1;
                transaction_errors.push(TokenTransactionUpdateError {
                    tx_hash: hash_string(&tx.processed.hash),
                    tx_index: tx.processed.tx_index,
                    message: error.clone(),
                });
                continue;
            }
            if !should_apply_transaction_to_token_state(&tx.processed) {
                continue;
            }
            index_metadata_transaction(&mut metadata_tx_index, &tx.processed);

            let pending_tx_hashes = pending_metadata_tx_hashes(&metadata_tx_index, &tx.processed);
            match self
                .discover_created_tokens(&tx.processed, &pending_tx_hashes, metadata_provider)
                .await
            {
                Ok(created) => {
                    created_token_addresses.extend(created);
                }
                Err(error) => {
                    self.last_block_failure_count += 1;
                    transaction_errors.push(TokenTransactionUpdateError {
                        tx_hash: hash_string(&tx.processed.hash),
                        tx_index: tx.processed.tx_index,
                        message: error.to_string(),
                    });
                }
            }

            let prior_txs = replay_context.prior_txs_for_transaction(&self.registry, &tx.processed);
            match self
                .update_router
                .update_registry_from_processed_transaction_with_trading_simulation(
                    &mut self.registry,
                    &self.token_index,
                    &tx.processed,
                    trading_simulation,
                    &prior_txs,
                    Some(&block.header),
                )
                .await
            {
                Ok(reports) => {
                    processed_transaction_count += 1;
                    let mut transaction_token_addresses = BTreeSet::new();
                    for report in reports {
                        let token_address = report.token_address.clone();
                        transaction_token_addresses.insert(token_address.clone());
                        updated_token_addresses.insert(token_address.clone());
                        self.refresh_token_index(&token_address, block_number);
                        token_updates.push(report);
                    }
                    self.apply_network_updates_for_transaction(
                        &tx.processed,
                        transaction_token_addresses,
                    );
                }
                Err(error) => {
                    self.last_block_failure_count += 1;
                    transaction_errors.push(TokenTransactionUpdateError {
                        tx_hash: hash_string(&tx.processed.hash),
                        tx_index: tx.processed.tx_index,
                        message: error.to_string(),
                    });
                }
            }
            replay_context.observe_transaction(&self.registry, &tx.processed);
        }

        if self.start_block.is_none() {
            self.start_block = Some(block_number);
        }
        self.latest_processed_block = Some(block_number);
        self.processed_blocks.insert(block_number, true);

        self.updated_token_addresses = updated_token_addresses.into_iter().collect();

        TokenBlockUpdateReport {
            block_number,
            block_hash: hash_string(&block.header.hash),
            block_timestamp: block.header.timestamp,
            transaction_count: block.transactions.len(),
            processed_transaction_count,
            failed_transaction_count: self.last_block_failure_count,
            already_processed: false,
            created_token_addresses: created_token_addresses.into_iter().collect(),
            updated_token_addresses: self.updated_token_addresses.clone(),
            token_updates,
            transaction_errors,
        }
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
        if self.is_live_mode {
            return self.live_mode_historical_simulator_report(
                block,
                "process_block_with_discovery_provider",
            );
        }

        self.process_block_with_token_and_pool_discovery_providers(
            block,
            discovery_provider,
            discovery_provider,
            pool_simulator,
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
        self.process_block_with_token_and_pool_discovery_providers_and_trading_simulation(
            block,
            discovery_provider,
            discovery_provider,
            V2TradingSimulation::Live(pool_simulator),
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

        self.process_block_with_token_and_pool_discovery_providers_and_trading_simulation(
            block,
            metadata_provider,
            pool_metadata_provider,
            V2TradingSimulation::Historical(pool_simulator),
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
        self.process_block_with_token_and_pool_discovery_providers_and_trading_simulation(
            block,
            metadata_provider,
            pool_metadata_provider,
            V2TradingSimulation::Live(pool_simulator),
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
            V2TradingSimulation::Noop,
        )
        .await
    }

    async fn process_block_with_token_and_pool_discovery_providers_and_trading_simulation<T, V>(
        &mut self,
        block: &ProcessedBlock,
        metadata_provider: &T,
        pool_metadata_provider: &V,
        trading_simulation: V2TradingSimulation<'_>,
    ) -> TokenBlockUpdateReport
    where
        T: TokenMetadataProvider,
        V: UniswapV2PoolMetadataProvider,
    {
        let block_number = block.header.number;
        if self.processed_blocks.contains_key(&block_number) {
            return TokenBlockUpdateReport {
                block_number,
                block_hash: hash_string(&block.header.hash),
                block_timestamp: block.header.timestamp,
                transaction_count: block.transactions.len(),
                processed_transaction_count: 0,
                failed_transaction_count: 0,
                already_processed: true,
                created_token_addresses: Vec::new(),
                updated_token_addresses: Vec::new(),
                token_updates: Vec::new(),
                transaction_errors: Vec::new(),
            };
        }

        self.updated_token_addresses.clear();
        self.last_block_failure_count = 0;

        let mut transactions: Vec<_> = block.transactions.iter().collect();
        transactions.sort_by_key(|tx| tx.processed.tx_index);

        let mut token_updates = Vec::new();
        let mut transaction_errors = Vec::new();
        let mut created_token_addresses = BTreeSet::new();
        let mut updated_token_addresses = BTreeSet::new();
        let mut processed_transaction_count = 0;
        let mut metadata_tx_index = HashMap::new();
        let mut replay_context = BlockReplayContext::default();

        for tx in transactions {
            if let Some(error) = &tx.processing_error {
                self.last_block_failure_count += 1;
                transaction_errors.push(TokenTransactionUpdateError {
                    tx_hash: hash_string(&tx.processed.hash),
                    tx_index: tx.processed.tx_index,
                    message: error.clone(),
                });
                continue;
            }
            if !should_apply_transaction_to_token_state(&tx.processed) {
                continue;
            }
            index_metadata_transaction(&mut metadata_tx_index, &tx.processed);

            let pending_tx_hashes = pending_metadata_tx_hashes(&metadata_tx_index, &tx.processed);
            match self
                .discover_created_tokens(&tx.processed, &pending_tx_hashes, metadata_provider)
                .await
            {
                Ok(created) => {
                    created_token_addresses.extend(created);
                }
                Err(error) => {
                    self.last_block_failure_count += 1;
                    transaction_errors.push(TokenTransactionUpdateError {
                        tx_hash: hash_string(&tx.processed.hash),
                        tx_index: tx.processed.tx_index,
                        message: error.to_string(),
                    });
                }
            }

            let prior_txs = replay_context.prior_txs_for_transaction(&self.registry, &tx.processed);
            match self
                .update_router
                .update_registry_from_processed_transaction_with_discovery_and_trading_simulation(
                    &mut self.registry,
                    &self.token_index,
                    &tx.processed,
                    pool_metadata_provider,
                    trading_simulation,
                    &prior_txs,
                    Some(&block.header),
                )
                .await
            {
                Ok(reports) => {
                    processed_transaction_count += 1;
                    let mut transaction_token_addresses = BTreeSet::new();
                    for report in reports {
                        let token_address = report.token_address.clone();
                        transaction_token_addresses.insert(token_address.clone());
                        updated_token_addresses.insert(token_address.clone());
                        self.refresh_token_index(&token_address, block_number);
                        token_updates.push(report);
                    }
                    self.apply_network_updates_for_transaction(
                        &tx.processed,
                        transaction_token_addresses,
                    );
                }
                Err(error) => {
                    self.last_block_failure_count += 1;
                    transaction_errors.push(TokenTransactionUpdateError {
                        tx_hash: hash_string(&tx.processed.hash),
                        tx_index: tx.processed.tx_index,
                        message: error.to_string(),
                    });
                }
            }
            replay_context.observe_transaction(&self.registry, &tx.processed);
        }

        if self.start_block.is_none() {
            self.start_block = Some(block_number);
        }
        self.latest_processed_block = Some(block_number);
        self.processed_blocks.insert(block_number, true);

        self.updated_token_addresses = updated_token_addresses.into_iter().collect();

        TokenBlockUpdateReport {
            block_number,
            block_hash: hash_string(&block.header.hash),
            block_timestamp: block.header.timestamp,
            transaction_count: block.transactions.len(),
            processed_transaction_count,
            failed_transaction_count: self.last_block_failure_count,
            already_processed: false,
            created_token_addresses: created_token_addresses.into_iter().collect(),
            updated_token_addresses: self.updated_token_addresses.clone(),
            token_updates,
            transaction_errors,
        }
    }

    async fn discover_created_tokens<P>(
        &mut self,
        tx: &ProcessedTransaction,
        pending_tx_hashes: &[B256],
        metadata_provider: &P,
    ) -> eyre::Result<Vec<String>>
    where
        P: TokenMetadataProvider,
    {
        let mut created = Vec::new();

        for token_address in created_token_addresses(tx) {
            let token_address_string = address_string(&token_address);
            if self.registry.token(&token_address_string).is_some() {
                continue;
            }

            let lookup = TokenMetadataLookup {
                token_address,
                block_number: tx.block_number,
                block_timestamp: tx.block_timestamp,
                metadata_block_number: tx.block_number.saturating_sub(1),
                transaction_hash: tx.hash,
                tx_index: tx.tx_index,
                creator_address: tx.from_address,
                creator_nonce: tx.nonce,
                pending_tx_hashes: pending_tx_hashes.to_vec(),
            };

            let metadata = if self.is_live_mode {
                match tokio::time::timeout(
                    Duration::from_millis(LIVE_METADATA_LOOKUP_TIMEOUT_MS),
                    metadata_provider.token_metadata(&lookup),
                )
                .await
                {
                    Ok(metadata) => metadata?,
                    Err(_) => {
                        tracing::warn!(
                            target: LIVE_TOKEN_TRACKER_LOG_TARGET,
                            block_number = tx.block_number,
                            tx_index = tx.tx_index,
                            tx_hash = %hash_string(&tx.hash),
                            token_address = %token_address_string,
                            timeout_ms = LIVE_METADATA_LOOKUP_TIMEOUT_MS,
                            action = "token_metadata_lookup",
                            result = "timeout",
                            "live token metadata lookup timed out"
                        );
                        None
                    }
                }
            } else {
                metadata_provider.token_metadata(&lookup).await?
            };

            let Some(metadata) = metadata else {
                continue;
            };

            let token_address = normalize_address(&metadata.address);
            if self.registry.token(&token_address).is_some() {
                continue;
            }

            self.registry
                .add_token_with_live_mode(metadata, self.is_live_mode);
            {
                let Some(token) = self.registry.token_mut(&token_address) else {
                    continue;
                };
                token.handle_contract_creation(
                    tx.block_number,
                    tx.block_timestamp,
                    hash_string(&tx.hash),
                    address_string(&tx.from_address),
                    tx.nonce,
                );
            }
            self.index_registry_token(
                &token_address,
                TrackedTokenStatus::Creation,
                tx.block_number,
            );
            created.push(token_address);
        }

        Ok(created)
    }

    fn refresh_token_index(&mut self, token_address: &str, current_block: u64) {
        let Some(status) = self.registry.token(token_address).map(|token| {
            if token.is_scam() {
                TrackedTokenStatus::InactiveScam
            } else if token.trading_enabled() {
                TrackedTokenStatus::Active
            } else {
                TrackedTokenStatus::Creation
            }
        }) else {
            self.network_graphs
                .remove(&normalize_address(token_address));
            return;
        };
        self.index_registry_token(token_address, status, current_block);
    }

    pub(super) fn index_registry_token(
        &mut self,
        token_address: &str,
        status: TrackedTokenStatus,
        current_block: u64,
    ) -> TrackedTokenIndexUpdate {
        let update = self.token_index.index_registry_token(
            &mut self.registry,
            token_address,
            status,
            current_block,
        );
        self.cleanup_network_graphs_for_index_update(&update);
        update
    }

    pub(super) fn apply_network_updates_for_transaction(
        &mut self,
        tx: &ProcessedTransaction,
        token_addresses: BTreeSet<String>,
    ) {
        for token_address in token_addresses {
            let Some((token_address, decimals)) = self
                .registry
                .token(&token_address)
                .map(|token| (normalize_address(&token.contract_address), token.decimals))
            else {
                self.network_graphs
                    .remove(&normalize_address(&token_address));
                continue;
            };

            let batch = extract_token_network_updates(tx, &token_address, decimals);
            if batch.is_empty() {
                continue;
            }
            self.network_graph_mut(&token_address).apply_batch(batch);
        }
    }

    fn network_graph_mut(&mut self, token_address: &str) -> &mut RawTokenNetworkGraph {
        let address = normalize_address(token_address);
        self.network_graphs
            .entry(address.clone())
            .or_insert_with(|| {
                RawTokenNetworkGraph::new(TokenNetworkId::new(None, address.as_str()))
            })
    }

    fn cleanup_network_graphs_for_index_update(&mut self, update: &TrackedTokenIndexUpdate) {
        if update.removed_by_retention {
            self.network_graphs
                .remove(&normalize_address(&update.token_address));
        }
        if let Some(evicted_token_address) = &update.evicted_token_address {
            self.network_graphs
                .remove(&normalize_address(evicted_token_address));
        }
    }

    pub fn apply_retention_policy(
        &mut self,
        policy: &LiveTokenRetentionPolicy,
        current_block: u64,
    ) -> LiveTokenRetentionReport {
        let report =
            self.token_index
                .apply_retention_policy(&mut self.registry, policy, current_block);
        self.cleanup_network_graphs_to_registry();
        report
    }

    pub fn apply_index_retention_policy(
        &mut self,
        current_block: u64,
    ) -> Option<LiveTokenRetentionReport> {
        let report = self
            .token_index
            .apply_live_retention_policy(&mut self.registry, current_block)?;
        self.cleanup_network_graphs_to_registry();
        Some(report)
    }

    fn cleanup_network_graphs_to_registry(&mut self) {
        self.network_graphs
            .retain(|token_address, _| self.registry.token(token_address).is_some());
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

fn created_token_addresses(tx: &ProcessedTransaction) -> Vec<Address> {
    if !should_apply_transaction_to_token_state(tx) {
        return Vec::new();
    }

    let mut addresses = Vec::new();
    if let Some(address) = tx.contract_address {
        addresses.push(address);
    }
    addresses.extend(
        tx.contract_creation_events
            .iter()
            .map(|event| event.contract_address),
    );
    addresses.sort();
    addresses.dedup();
    addresses
}

fn should_apply_transaction_to_token_state(tx: &ProcessedTransaction) -> bool {
    tx.status
}

fn index_metadata_transaction(index: &mut HashMap<Address, Vec<B256>>, tx: &ProcessedTransaction) {
    let mut addresses = BTreeSet::new();
    addresses.insert(tx.from_address);
    addresses.extend(tx.unique_addresses.iter().copied());

    for address in addresses {
        index.entry(address).or_default().push(tx.hash);
    }
}

fn pending_metadata_tx_hashes(
    index: &HashMap<Address, Vec<B256>>,
    tx: &ProcessedTransaction,
) -> Vec<B256> {
    index
        .get(&tx.from_address)
        .cloned()
        .unwrap_or_else(|| vec![tx.hash])
}
