use std::collections::{BTreeMap, BTreeSet, HashMap};

use alloy_primitives::{Address, B256};
use serde::{Deserialize, Serialize};
use tx_processor::{
    LivePoolBuySellSimulator, PoolBuySellSimulator, ProcessedBlock, ProcessedTransaction,
};

use super::update_router::V2TradingSimulation;
use super::ProcessedTokenUpdateRouter;
use super::{
    address_string, hash_string, normalize_address, TokenDiscoveryProvider, TokenMetadataLookup,
    TokenMetadataProvider, TokenRegistry, TokenStateUpdateReport, TrackedTokenIndex,
    TrackedTokenStatus, UniswapV2PoolMetadataProvider,
};

pub const DEFAULT_TRACKED_TOKEN_INDEX_SIZE: usize = 2000;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TokenTransactionUpdateError {
    pub tx_hash: String,
    pub tx_index: u64,
    pub message: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TokenBlockUpdateReport {
    pub block_number: u64,
    pub block_hash: String,
    pub block_timestamp: u64,
    pub transaction_count: usize,
    pub processed_transaction_count: usize,
    pub failed_transaction_count: usize,
    pub already_processed: bool,
    pub created_token_addresses: Vec<String>,
    pub updated_token_addresses: Vec<String>,
    pub token_updates: Vec<TokenStateUpdateReport>,
    pub transaction_errors: Vec<TokenTransactionUpdateError>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BlockTokenProcessor {
    #[serde(default)]
    pub is_live_mode: bool,
    pub registry: TokenRegistry,
    pub update_router: ProcessedTokenUpdateRouter,
    pub token_index: TrackedTokenIndex,
    pub processed_blocks: BTreeMap<u64, bool>,
    pub latest_processed_block: Option<u64>,
    pub start_block: Option<u64>,
    pub updated_token_addresses: Vec<String>,
    pub last_block_failure_count: usize,
}

impl BlockTokenProcessor {
    pub fn new(history_limit: usize) -> Self {
        Self {
            is_live_mode: false,
            registry: TokenRegistry::new(),
            update_router: ProcessedTokenUpdateRouter::new(history_limit),
            token_index: TrackedTokenIndex::new(DEFAULT_TRACKED_TOKEN_INDEX_SIZE),
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
    }

    pub async fn process_block(
        &mut self,
        block: &ProcessedBlock,
        pool_simulator: &PoolBuySellSimulator,
    ) -> TokenBlockUpdateReport {
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
        let mut prior_control_txs_by_sender = HashMap::new();

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

            let prior_txs = simulation_prior_txs_for_sender(
                &mut prior_control_txs_by_sender,
                &self.registry,
                &tx.processed,
            );
            match self
                .update_router
                .update_registry_from_processed_transaction_with_trading_simulation(
                    &mut self.registry,
                    &self.token_index,
                    &tx.processed,
                    trading_simulation,
                    &prior_txs,
                )
                .await
            {
                Ok(reports) => {
                    processed_transaction_count += 1;
                    for report in reports {
                        updated_token_addresses.insert(report.token_address.clone());
                        self.refresh_token_index(&report.token_address);
                        token_updates.push(report);
                    }
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
        let mut prior_control_txs_by_sender = HashMap::new();

        for tx in transactions {
            index_metadata_transaction(&mut metadata_tx_index, &tx.processed);
            if let Some(error) = &tx.processing_error {
                self.last_block_failure_count += 1;
                transaction_errors.push(TokenTransactionUpdateError {
                    tx_hash: hash_string(&tx.processed.hash),
                    tx_index: tx.processed.tx_index,
                    message: error.clone(),
                });
                continue;
            }

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

            let prior_txs = simulation_prior_txs_for_sender(
                &mut prior_control_txs_by_sender,
                &self.registry,
                &tx.processed,
            );
            match self
                .update_router
                .update_registry_from_processed_transaction_with_trading_simulation(
                    &mut self.registry,
                    &self.token_index,
                    &tx.processed,
                    trading_simulation,
                    &prior_txs,
                )
                .await
            {
                Ok(reports) => {
                    processed_transaction_count += 1;
                    for report in reports {
                        updated_token_addresses.insert(report.token_address.clone());
                        self.refresh_token_index(&report.token_address);
                        token_updates.push(report);
                    }
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
        let mut prior_control_txs_by_sender = HashMap::new();

        for tx in transactions {
            index_metadata_transaction(&mut metadata_tx_index, &tx.processed);
            if let Some(error) = &tx.processing_error {
                self.last_block_failure_count += 1;
                transaction_errors.push(TokenTransactionUpdateError {
                    tx_hash: hash_string(&tx.processed.hash),
                    tx_index: tx.processed.tx_index,
                    message: error.clone(),
                });
                continue;
            }

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

            let prior_txs = simulation_prior_txs_for_sender(
                &mut prior_control_txs_by_sender,
                &self.registry,
                &tx.processed,
            );
            match self
                .update_router
                .update_registry_from_processed_transaction_with_discovery_and_trading_simulation(
                    &mut self.registry,
                    &self.token_index,
                    &tx.processed,
                    pool_metadata_provider,
                    trading_simulation,
                    &prior_txs,
                )
                .await
            {
                Ok(reports) => {
                    processed_transaction_count += 1;
                    for report in reports {
                        updated_token_addresses.insert(report.token_address.clone());
                        self.refresh_token_index(&report.token_address);
                        token_updates.push(report);
                    }
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

            let Some(metadata) = metadata_provider.token_metadata(&lookup).await? else {
                continue;
            };

            let token_address = normalize_address(&metadata.address);
            if self.registry.token(&token_address).is_some() {
                continue;
            }

            self.registry
                .add_token_with_live_mode(metadata, self.is_live_mode);
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
            self.token_index
                .index_token(token, TrackedTokenStatus::Creation);
            created.push(token_address);
        }

        Ok(created)
    }

    fn refresh_token_index(&mut self, token_address: &str) {
        let Some(token) = self.registry.token(token_address) else {
            return;
        };
        let status = if token.is_scam() {
            TrackedTokenStatus::InactiveScam
        } else if token.trading_enabled() {
            TrackedTokenStatus::Active
        } else {
            TrackedTokenStatus::Creation
        };
        self.token_index.index_token(token, status);
    }
}

fn simulation_prior_txs_for_sender(
    prior_control_txs_by_sender: &mut HashMap<Address, Vec<ProcessedTransaction>>,
    registry: &TokenRegistry,
    tx: &ProcessedTransaction,
) -> Vec<ProcessedTransaction> {
    let mut prior_txs = prior_control_txs_by_sender
        .get(&tx.from_address)
        .cloned()
        .unwrap_or_default();

    if transaction_touches_tracked_control_address(registry, tx) {
        prior_txs.push(tx.clone());
        prior_control_txs_by_sender
            .entry(tx.from_address)
            .or_default()
            .push(tx.clone());
    }

    prior_txs.sort_by_key(|tx| tx.tx_index);
    prior_txs.dedup_by_key(|tx| tx.hash);
    prior_txs
}

fn transaction_touches_tracked_control_address(
    registry: &TokenRegistry,
    tx: &ProcessedTransaction,
) -> bool {
    let tx_addresses = tx_address_strings(tx);
    registry.tokens.values().any(|token| {
        token
            .token_control_addresses
            .iter()
            .any(|address| tx_addresses.contains(&normalize_address(address)))
    })
}

fn tx_address_strings(tx: &ProcessedTransaction) -> BTreeSet<String> {
    let mut addresses = BTreeSet::new();
    addresses.extend(tx.unique_addresses.iter().map(address_string));
    addresses.extend(tx.erc20_contracts.iter().map(address_string));
    addresses.insert(address_string(&tx.from_address));
    if let Some(address) = tx.to_address {
        addresses.insert(address_string(&address));
    }
    if let Some(address) = tx.contract_address {
        addresses.insert(address_string(&address));
    }
    addresses
}

fn created_token_addresses(tx: &ProcessedTransaction) -> Vec<Address> {
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
