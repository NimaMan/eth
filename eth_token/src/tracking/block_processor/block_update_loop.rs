use std::collections::{BTreeSet, HashMap};
use std::time::Instant;

use tx_processor::ProcessedBlock;

use crate::chain_metadata::{TokenMetadataProvider, UniswapV2PoolMetadataProvider};
use crate::tracking::token_update_router::PoolTradingSimulationMode;
use crate::tracking::{hash_string, TokenBlockUpdateReport, TokenTransactionUpdateError};

use super::block_update_profile::{
    elapsed_millis, log_block_token_processor_profile, BlockTokenProcessorProfile,
};
use super::processor::BlockTokenProcessor;
use super::token_creation_update::{
    index_metadata_transaction, pending_metadata_tx_hashes, should_apply_transaction_to_token_state,
};

impl BlockTokenProcessor {
    pub(in crate::tracking::block_processor) async fn process_block_with_trading_simulation(
        &mut self,
        block: &ProcessedBlock,
        trading_simulation: PoolTradingSimulationMode<'_>,
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

            match self
                .update_router
                .update_registry_from_processed_transaction_with_trading_simulation(
                    &mut self.registry,
                    &self.token_index,
                    &tx.processed,
                    trading_simulation,
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
    pub(in crate::tracking::block_processor) async fn process_block_with_metadata_provider_and_trading_simulation<
        P,
    >(
        &mut self,
        block: &ProcessedBlock,
        metadata_provider: &P,
        trading_simulation: PoolTradingSimulationMode<'_>,
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

            match self
                .update_router
                .update_registry_from_processed_transaction_with_trading_simulation(
                    &mut self.registry,
                    &self.token_index,
                    &tx.processed,
                    trading_simulation,
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

    pub(in crate::tracking::block_processor) async fn process_block_with_token_and_pool_discovery_providers_and_trading_simulation<
        T,
        V,
    >(
        &mut self,
        block: &ProcessedBlock,
        metadata_provider: &T,
        pool_metadata_provider: &V,
        trading_simulation: PoolTradingSimulationMode<'_>,
    ) -> TokenBlockUpdateReport
    where
        T: TokenMetadataProvider,
        V: UniswapV2PoolMetadataProvider,
    {
        let block_started = Instant::now();
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

        let mut profile = BlockTokenProcessorProfile::default();
        let sort_started = Instant::now();
        let mut transactions: Vec<_> = block.transactions.iter().collect();
        transactions.sort_by_key(|tx| tx.processed.tx_index);
        profile.sort_ms = elapsed_millis(sort_started);

        let mut token_updates = Vec::new();
        let mut transaction_errors = Vec::new();
        let mut created_token_addresses = BTreeSet::new();
        let mut updated_token_addresses = BTreeSet::new();
        let mut processed_transaction_count = 0;
        let mut metadata_tx_index = HashMap::new();

        for tx in transactions {
            if let Some(error) = &tx.processing_error {
                profile.processing_error_transactions += 1;
                self.last_block_failure_count += 1;
                transaction_errors.push(TokenTransactionUpdateError {
                    tx_hash: hash_string(&tx.processed.hash),
                    tx_index: tx.processed.tx_index,
                    message: error.clone(),
                });
                continue;
            }
            if !should_apply_transaction_to_token_state(&tx.processed) {
                profile.skipped_transactions += 1;
                continue;
            }
            profile.applicable_transactions += 1;

            let token_metadata_started = Instant::now();
            index_metadata_transaction(&mut metadata_tx_index, &tx.processed);

            let pending_tx_hashes = pending_metadata_tx_hashes(&metadata_tx_index, &tx.processed);
            let created_result = self
                .discover_created_tokens(&tx.processed, &pending_tx_hashes, metadata_provider)
                .await;
            profile.token_metadata_ms += elapsed_millis(token_metadata_started);
            match created_result {
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

            let router_started = Instant::now();
            let update_result = self
                .update_router
                .update_registry_from_processed_transaction_with_discovery_and_trading_simulation(
                    &mut self.registry,
                    &self.token_index,
                    &tx.processed,
                    pool_metadata_provider,
                    trading_simulation,
                    Some(&block.header),
                    Some(&mut profile.applier),
                )
                .await;
            profile.router_ms += elapsed_millis(router_started);
            match update_result {
                Ok(reports) => {
                    processed_transaction_count += 1;
                    let index_refresh_started = Instant::now();
                    let mut transaction_token_addresses = BTreeSet::new();
                    for report in reports {
                        let token_address = report.token_address.clone();
                        transaction_token_addresses.insert(token_address.clone());
                        updated_token_addresses.insert(token_address.clone());
                        self.refresh_token_index(&token_address, block_number);
                        token_updates.push(report);
                    }
                    profile.index_refresh_ms += elapsed_millis(index_refresh_started);
                    let network_update_started = Instant::now();
                    self.apply_network_updates_for_transaction(
                        &tx.processed,
                        transaction_token_addresses,
                    );
                    profile.network_update_ms += elapsed_millis(network_update_started);
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

        let finalize_started = Instant::now();
        if self.start_block.is_none() {
            self.start_block = Some(block_number);
        }
        self.latest_processed_block = Some(block_number);
        self.processed_blocks.insert(block_number, true);

        self.updated_token_addresses = updated_token_addresses.into_iter().collect();
        profile.finalize_ms = elapsed_millis(finalize_started);

        let created_token_count = created_token_addresses.len();
        let updated_token_count = self.updated_token_addresses.len();
        let token_update_count = token_updates.len();
        let transaction_error_count = transaction_errors.len();
        log_block_token_processor_profile(
            self,
            block_number,
            block.transactions.len(),
            processed_transaction_count,
            created_token_count,
            updated_token_count,
            token_update_count,
            transaction_error_count,
            block_started,
            &profile,
        );

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
}
