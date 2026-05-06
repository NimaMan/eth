//! Block-level token orchestration over processed Rust transactions.

use std::collections::HashMap;

use alloy_primitives::{Address, B256};
use eyre::Result;
use serde::{Deserialize, Serialize};
use tx_processor::ProcessedTransaction;

use crate::erc20::{ERC20Token, ERC20TokenMetadata};
use crate::pools::BasePoolConfig;

pub mod block_processor;
pub mod cache;
pub mod metadata;
pub mod token_builder;

pub use block_processor::{
    BlockTokenProcessor, TokenBlockUpdateReport, TokenTransactionUpdateError,
    DEFAULT_TOKEN_CACHE_SIZE,
};
pub use cache::{TokenCacheEntry, TokenCacheStatus, TokenStateCache};
pub use metadata::{
    NoopUniswapV2PoolMetadataProvider, RethTokenMetadataProvider, StaticTokenMetadataProvider,
    StaticUniswapV2PoolMetadataProvider, TokenMetadataLookup, TokenMetadataProvider,
    TokenPipelineMetadataProvider, UniswapV2PoolMetadata, UniswapV2PoolMetadataLookup,
    UniswapV2PoolMetadataProvider,
};
pub use token_builder::TokenStateBuilder;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TokenStateUpdateReport {
    pub token_address: String,
    pub token_state_updated: bool,
    pub discovered_uniswap_v2_pools: Vec<String>,
    pub updated_uniswap_v2_pools: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TokenStateManager {
    pub tokens: HashMap<String, ERC20Token>,
    pub history_limit: usize,
    pub known_routers: Vec<String>,
}

impl TokenStateManager {
    pub fn new(history_limit: usize) -> Self {
        Self {
            tokens: HashMap::new(),
            history_limit,
            known_routers: Vec::new(),
        }
    }

    pub fn with_known_routers(
        history_limit: usize,
        routers: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            tokens: HashMap::new(),
            history_limit,
            known_routers: routers
                .into_iter()
                .map(|router| normalize_address_string(router))
                .collect(),
        }
    }

    pub fn add_token(&mut self, metadata: ERC20TokenMetadata) -> Option<ERC20Token> {
        let address = normalize_address_string(metadata.address.clone());
        self.tokens.insert(address, ERC20Token::new(metadata))
    }

    pub fn token(&self, address: impl AsRef<str>) -> Option<&ERC20Token> {
        self.tokens.get(&normalize_address(address))
    }

    pub fn token_mut(&mut self, address: impl AsRef<str>) -> Option<&mut ERC20Token> {
        self.tokens.get_mut(&normalize_address(address))
    }

    pub fn update_from_processed_transaction(
        &mut self,
        tx: &ProcessedTransaction,
    ) -> Result<Vec<TokenStateUpdateReport>> {
        let token_addresses: Vec<_> = self.tokens.keys().cloned().collect();
        let mut reports = Vec::new();

        for token_address in token_addresses {
            let Some(token) = self.tokens.get_mut(&token_address) else {
                continue;
            };

            let token_state_updated = touches_token_state(tx, &token_address);
            if token_state_updated {
                token.update_token_state_from_processed_transaction(tx)?;
            }

            let discovered = discover_uniswap_v2_pools_for_token(
                token,
                tx,
                self.history_limit,
                &self.known_routers,
            );

            let pool_addresses = token.pool_addresses();
            let mut updated = Vec::new();
            for pool_address in pool_addresses {
                if touches_v2_pool(tx, &pool_address) {
                    token.update_uniswap_v2_pool_from_processed_transaction(&pool_address, tx)?;
                    updated.push(pool_address);
                }
            }

            if token_state_updated || !discovered.is_empty() || !updated.is_empty() {
                reports.push(TokenStateUpdateReport {
                    token_address,
                    token_state_updated,
                    discovered_uniswap_v2_pools: discovered,
                    updated_uniswap_v2_pools: updated,
                });
            }
        }

        Ok(reports)
    }

    pub async fn update_from_processed_transaction_with_v2_pool_metadata<P>(
        &mut self,
        tx: &ProcessedTransaction,
        pool_metadata_provider: &P,
    ) -> Result<Vec<TokenStateUpdateReport>>
    where
        P: UniswapV2PoolMetadataProvider,
    {
        let token_addresses: Vec<_> = self.tokens.keys().cloned().collect();
        let mut reports = Vec::new();

        for token_address in token_addresses {
            let Some(token) = self.tokens.get_mut(&token_address) else {
                continue;
            };

            let token_state_updated = touches_token_state(tx, &token_address);
            if token_state_updated {
                token.update_token_state_from_processed_transaction(tx)?;
            }

            let mut discovered = discover_uniswap_v2_pools_for_token_with_metadata(
                token,
                tx,
                self.history_limit,
                &self.known_routers,
                pool_metadata_provider,
            )
            .await?;
            discovered.extend(
                discover_uniswap_v2_pools_from_swaps_for_token(
                    token,
                    tx,
                    self.history_limit,
                    &self.known_routers,
                    pool_metadata_provider,
                )
                .await?,
            );

            let pool_addresses = token.pool_addresses();
            let mut updated = Vec::new();
            for pool_address in pool_addresses {
                if touches_v2_pool(tx, &pool_address) {
                    token.update_uniswap_v2_pool_from_processed_transaction(&pool_address, tx)?;
                    updated.push(pool_address);
                }
            }

            if token_state_updated || !discovered.is_empty() || !updated.is_empty() {
                discovered.sort();
                discovered.dedup();
                reports.push(TokenStateUpdateReport {
                    token_address,
                    token_state_updated,
                    discovered_uniswap_v2_pools: discovered,
                    updated_uniswap_v2_pools: updated,
                });
            }
        }

        Ok(reports)
    }
}

fn touches_token_state(tx: &ProcessedTransaction, token_address: &str) -> bool {
    tx.erc20_contracts
        .iter()
        .any(|address| same_address_str(*address, token_address))
        || tx
            .erc20_transfers
            .iter()
            .any(|event| same_address_str(event.token_address, token_address))
        || tx
            .erc20_approval_events
            .iter()
            .any(|event| same_address_str(event.token_address, token_address))
        || tx
            .ownership_transferred_events
            .iter()
            .any(|event| same_address_str(event.contract_address, token_address))
        || tx
            .ownership_transfer_started_events
            .iter()
            .any(|event| same_address_str(event.contract_address, token_address))
        || tx
            .access_control_role_granted_events
            .iter()
            .any(|event| same_address_str(event.contract_address, token_address))
        || tx
            .access_control_role_revoked_events
            .iter()
            .any(|event| same_address_str(event.contract_address, token_address))
        || tx
            .proxy_admin_changed_events
            .iter()
            .any(|event| same_address_str(event.contract_address, token_address))
        || tx
            .trading_enabled_events
            .iter()
            .any(|event| same_address_str(event.token_address, token_address))
        || tx
            .trading_disabled_events
            .iter()
            .any(|event| same_address_str(event.token_address, token_address))
        || tx
            .contract_address
            .is_some_and(|address| same_address_str(address, token_address))
        || tx
            .contract_creation_events
            .iter()
            .any(|event| same_address_str(event.contract_address, token_address))
}

fn discover_uniswap_v2_pools_for_token(
    token: &mut ERC20Token,
    tx: &ProcessedTransaction,
    history_limit: usize,
    known_routers: &[String],
) -> Vec<String> {
    let token_address = token.contract_address.clone();
    let mut discovered = Vec::new();

    for event in &tx.uniswap_v2_pair_created_events {
        let token_is_token0 = same_address_str(event.token0, &token_address);
        let token_is_token1 = same_address_str(event.token1, &token_address);
        if !token_is_token0 && !token_is_token1 {
            continue;
        }

        let pool_address = address_string(&event.pair_address);
        if token.uniswap_v2_pool(&pool_address).is_some() {
            continue;
        }

        let denom_address = if token_is_token0 {
            event.token1
        } else {
            event.token0
        };
        let pool = token.create_uniswap_v2_pool(
            pool_address.clone(),
            address_string(&denom_address),
            BasePoolConfig {
                token_decimals: token.decimals,
                denom_decimals: None,
                token1_is_denom: Some(token_is_token0),
                history_limit,
                denom_threshold: 0.0,
                threshold_unit: None,
                test_buy_amount_eth: crate::pools::base::DEFAULT_TEST_BUY_ETH,
            },
            known_routers,
        );
        pool.base.creation_block = Some(tx.block_number);
        pool.base.creation_tx = Some(hash_string(&tx.hash));
        pool.base.creation_timestamp = Some(tx.block_timestamp);
        discovered.push(pool_address);
    }

    discovered
}

async fn discover_uniswap_v2_pools_for_token_with_metadata<P>(
    token: &mut ERC20Token,
    tx: &ProcessedTransaction,
    history_limit: usize,
    known_routers: &[String],
    pool_metadata_provider: &P,
) -> Result<Vec<String>>
where
    P: UniswapV2PoolMetadataProvider,
{
    let token_address = token.contract_address.clone();
    let mut discovered = Vec::new();

    for event in &tx.uniswap_v2_pair_created_events {
        let token_is_token0 = same_address_str(event.token0, &token_address);
        let token_is_token1 = same_address_str(event.token1, &token_address);
        if !token_is_token0 && !token_is_token1 {
            continue;
        }

        let pool_address = address_string(&event.pair_address);
        if token.uniswap_v2_pool(&pool_address).is_some() {
            continue;
        }

        let metadata = pool_metadata_provider
            .uniswap_v2_pool_metadata(&UniswapV2PoolMetadataLookup {
                token_address: parse_address_lossy(&token_address),
                pool_address: event.pair_address,
                block_number: tx.block_number,
                transaction_hash: tx.hash,
                tx_index: tx.tx_index,
            })
            .await?;

        let (denom_address, config) = metadata
            .as_ref()
            .and_then(|metadata| v2_pool_config_from_metadata(token, metadata, history_limit))
            .unwrap_or_else(|| {
                let denom_address = if token_is_token0 {
                    event.token1
                } else {
                    event.token0
                };
                (
                    address_string(&denom_address),
                    BasePoolConfig {
                        token_decimals: token.decimals,
                        denom_decimals: None,
                        token1_is_denom: Some(token_is_token0),
                        history_limit,
                        denom_threshold: 0.0,
                        threshold_unit: None,
                        test_buy_amount_eth: crate::pools::base::DEFAULT_TEST_BUY_ETH,
                    },
                )
            });

        let pool = token.create_uniswap_v2_pool(
            pool_address.clone(),
            denom_address,
            config,
            known_routers,
        );
        pool.base.creation_block = Some(tx.block_number);
        pool.base.creation_tx = Some(hash_string(&tx.hash));
        pool.base.creation_timestamp = Some(tx.block_timestamp);
        discovered.push(pool_address);
    }

    Ok(discovered)
}

async fn discover_uniswap_v2_pools_from_swaps_for_token<P>(
    token: &mut ERC20Token,
    tx: &ProcessedTransaction,
    history_limit: usize,
    known_routers: &[String],
    pool_metadata_provider: &P,
) -> Result<Vec<String>>
where
    P: UniswapV2PoolMetadataProvider,
{
    let token_address = token.contract_address.clone();
    let mut discovered = Vec::new();

    for event in &tx.uniswap_v2_swaps {
        let pool_address = address_string(&event.pair_address);
        if token.uniswap_v2_pool(&pool_address).is_some()
            || discovered.iter().any(|known| known == &pool_address)
        {
            continue;
        }

        let Some(metadata) = pool_metadata_provider
            .uniswap_v2_pool_metadata(&UniswapV2PoolMetadataLookup {
                token_address: parse_address_lossy(&token_address),
                pool_address: event.pair_address,
                block_number: tx.block_number,
                transaction_hash: tx.hash,
                tx_index: tx.tx_index,
            })
            .await?
        else {
            continue;
        };

        let Some((denom_address, config)) =
            v2_pool_config_from_metadata(token, &metadata, history_limit)
        else {
            continue;
        };

        let pool = token.create_uniswap_v2_pool(
            pool_address.clone(),
            denom_address,
            config,
            known_routers,
        );
        pool.base.creation_block = Some(tx.block_number);
        pool.base.creation_tx = Some(hash_string(&tx.hash));
        pool.base.creation_timestamp = Some(tx.block_timestamp);
        discovered.push(pool_address);
    }

    Ok(discovered)
}

fn v2_pool_config_from_metadata(
    token: &ERC20Token,
    metadata: &UniswapV2PoolMetadata,
    history_limit: usize,
) -> Option<(String, BasePoolConfig)> {
    let token_address = normalize_address(&token.contract_address);
    let token_is_token0 = normalize_address(&metadata.token0) == token_address;
    let token_is_token1 = normalize_address(&metadata.token1) == token_address;
    if !token_is_token0 && !token_is_token1 {
        return None;
    }

    let (denom_address, denom_decimals, token1_is_denom) = if token_is_token0 {
        (metadata.token1.clone(), metadata.token1_decimals, true)
    } else {
        (metadata.token0.clone(), metadata.token0_decimals, false)
    };

    Some((
        denom_address,
        BasePoolConfig {
            token_decimals: token.decimals,
            denom_decimals: Some(denom_decimals),
            token1_is_denom: Some(token1_is_denom),
            history_limit,
            denom_threshold: 0.0,
            threshold_unit: None,
            test_buy_amount_eth: crate::pools::base::DEFAULT_TEST_BUY_ETH,
        },
    ))
}

fn touches_v2_pool(tx: &ProcessedTransaction, pool_address: &str) -> bool {
    tx.uniswap_v2_syncs
        .iter()
        .any(|event| same_address_str(event.pair_address, pool_address))
        || tx
            .uniswap_v2_swaps
            .iter()
            .any(|event| same_address_str(event.pair_address, pool_address))
        || tx
            .uniswap_v2_mints
            .iter()
            .any(|event| same_address_str(event.pair_address, pool_address))
        || tx
            .uniswap_v2_burns
            .iter()
            .any(|event| same_address_str(event.pair_address, pool_address))
        || tx
            .erc20_transfers
            .iter()
            .any(|event| same_address_str(event.token_address, pool_address))
        || tx
            .erc20_approval_events
            .iter()
            .any(|event| same_address_str(event.token_address, pool_address))
}

fn same_address_str(address: Address, value: &str) -> bool {
    address_string(&address) == normalize_address(value)
}

fn address_string(address: &Address) -> String {
    format!("{address:#x}")
}

fn hash_string(hash: &B256) -> String {
    format!("{hash:#x}")
}

fn normalize_address(value: impl AsRef<str>) -> String {
    value.as_ref().trim().to_ascii_lowercase()
}

fn normalize_address_string(value: impl Into<String>) -> String {
    normalize_address(value.into())
}

fn parse_address_lossy(value: &str) -> Address {
    value.parse().unwrap_or(Address::ZERO)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::future::Future;
    use std::pin::Pin;
    use std::rc::Rc;

    use alloy_primitives::{address, b256, Bytes, U256};
    use reth_chain_query::provider::{BlockHeader, TransactionData, TransactionReceipt};
    use tx_processor::tx_processor::data_models::{
        ContractCreationEvent, ERC20TransferEvent, UniswapV2PairCreatedEvent, UniswapV2SwapEvent,
        UniswapV2SyncEvent,
    };
    use tx_processor::{ProcessedBlock, ProcessedBlockTransactions};

    fn metadata() -> ERC20TokenMetadata {
        ERC20TokenMetadata::new(
            "0x1111111111111111111111111111111111111111",
            "Token",
            "TKN",
            18,
            "100000000000000000000",
        )
    }

    fn tx() -> ProcessedTransaction {
        ProcessedTransaction::new(
            b256!("0000000000000000000000000000000000000000000000000000000000000001"),
            100,
            1_700,
            1,
            address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
            None,
            U256::ZERO,
            true,
            0,
            2,
            Vec::new(),
        )
    }

    #[derive(Clone)]
    struct RecordingMetadataProvider {
        metadata: ERC20TokenMetadata,
        lookups: Rc<RefCell<Vec<TokenMetadataLookup>>>,
    }

    impl TokenMetadataProvider for RecordingMetadataProvider {
        fn token_metadata<'a>(
            &'a self,
            lookup: &'a TokenMetadataLookup,
        ) -> Pin<Box<dyn Future<Output = eyre::Result<Option<ERC20TokenMetadata>>> + 'a>> {
            Box::pin(async move {
                self.lookups.borrow_mut().push(lookup.clone());
                Ok(Some(self.metadata.clone()))
            })
        }
    }

    fn block_header() -> BlockHeader {
        BlockHeader {
            number: 100,
            hash: b256!("9999999999999999999999999999999999999999999999999999999999999999"),
            parent_hash: b256!("8888888888888888888888888888888888888888888888888888888888888888"),
            timestamp: 1_700,
            gas_limit: 30_000_000,
            gas_used: 21_000,
            base_fee_per_gas: Some(1),
            withdrawals_root: None,
            blob_gas_used: None,
            excess_blob_gas: None,
            parent_beacon_block_root: None,
            requests_hash: None,
            block_access_list_hash: None,
            slot_number: None,
        }
    }

    fn block_transaction(processed: ProcessedTransaction) -> ProcessedBlockTransactions {
        ProcessedBlockTransactions {
            metadata: TransactionData {
                hash: processed.hash,
                block_number: processed.block_number,
                block_timestamp: processed.block_timestamp,
                tx_index: processed.tx_index,
                tx_number: processed.tx_index,
                from: processed.from_address,
                to: processed.to_address,
                value: processed.value,
                input: Bytes::from(processed.input.clone()),
                gas_price: U256::ZERO,
                gas_limit: 21_000,
                nonce: processed.nonce,
                transaction_type: processed.raw_tx_type,
                max_fee_per_gas: None,
                max_priority_fee_per_gas: None,
                access_list: Vec::new(),
                blob_versioned_hashes: Vec::new(),
                max_fee_per_blob_gas: None,
                signed_authorizations: Vec::new(),
            },
            receipt: TransactionReceipt {
                tx_hash: processed.hash,
                status: processed.status,
                gas_used: 21_000,
                logs: Vec::new(),
                cumulative_gas_used: 21_000,
                effective_gas_price: U256::ZERO,
                contract_address: processed.contract_address,
                blob_gas_used: None,
            },
            processed,
            trace: None,
            processing_error: None,
        }
    }

    #[test]
    fn discovers_and_updates_uniswap_v2_pool_for_tracked_token() {
        let mut manager = TokenStateManager::new(100);
        manager.add_token(metadata());
        let mut tx = tx();
        tx.uniswap_v2_pair_created_events
            .push(UniswapV2PairCreatedEvent {
                pair_address: address!("3333333333333333333333333333333333333333"),
                token0: address!("1111111111111111111111111111111111111111"),
                token1: address!("2222222222222222222222222222222222222222"),
                log_index: 1,
            });
        tx.uniswap_v2_syncs.push(UniswapV2SyncEvent {
            pair_address: address!("3333333333333333333333333333333333333333"),
            reserve0: U256::from(100_000_000_000_000_000_000_u128),
            reserve1: U256::from(2_000_000_000_000_000_000_u128),
            log_index: 2,
        });

        let reports = manager.update_from_processed_transaction(&tx).unwrap();

        assert_eq!(reports.len(), 1);
        assert!(!reports[0].token_state_updated);
        assert_eq!(reports[0].discovered_uniswap_v2_pools.len(), 1);
        assert_eq!(reports[0].updated_uniswap_v2_pools.len(), 1);
        let token = manager
            .token("0x1111111111111111111111111111111111111111")
            .unwrap();
        let pool = token
            .uniswap_v2_pool("0x3333333333333333333333333333333333333333")
            .unwrap();
        assert_eq!(pool.base.token_reserve(), 100.0);
        assert_eq!(pool.base.denom_reserve(), 2.0);
    }

    #[tokio::test]
    async fn discovers_uniswap_v2_pool_from_swap_metadata_for_tracked_token() {
        let mut manager = TokenStateManager::new(100);
        manager.add_token(metadata());
        let mut tx = tx();
        tx.uniswap_v2_swaps.push(UniswapV2SwapEvent {
            pair_address: address!("3333333333333333333333333333333333333333"),
            sender: address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
            to: address!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"),
            amount0_in: U256::from(2_000_000_u64),
            amount1_in: U256::ZERO,
            amount0_out: U256::ZERO,
            amount1_out: U256::from(50_000_000_000_000_000_000_u128),
            log_index: 1,
        });
        tx.uniswap_v2_syncs.push(UniswapV2SyncEvent {
            pair_address: address!("3333333333333333333333333333333333333333"),
            reserve0: U256::from(2_000_000_u64),
            reserve1: U256::from(100_000_000_000_000_000_000_u128),
            log_index: 2,
        });

        let pool_metadata = StaticUniswapV2PoolMetadataProvider::new([UniswapV2PoolMetadata::new(
            "0x3333333333333333333333333333333333333333",
            "0x2222222222222222222222222222222222222222",
            "0x1111111111111111111111111111111111111111",
            6,
            18,
        )]);

        let reports = manager
            .update_from_processed_transaction_with_v2_pool_metadata(&tx, &pool_metadata)
            .await
            .unwrap();

        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].discovered_uniswap_v2_pools.len(), 1);
        assert_eq!(reports[0].updated_uniswap_v2_pools.len(), 1);
        let token = manager
            .token("0x1111111111111111111111111111111111111111")
            .unwrap();
        let pool = token
            .uniswap_v2_pool("0x3333333333333333333333333333333333333333")
            .unwrap();
        assert_eq!(pool.base.config.denom_decimals, Some(6));
        assert_eq!(pool.base.config.token1_is_denom, Some(false));
        assert_eq!(pool.base.token_reserve(), 100.0);
        assert_eq!(pool.base.denom_reserve(), 2.0);
        assert_eq!(pool.base.creation_block, Some(100));
    }

    #[test]
    fn reports_token_state_update_for_tracked_token_transfer() {
        let mut manager = TokenStateManager::new(100);
        manager.add_token(metadata());
        let mut tx = tx();
        tx.erc20_contracts
            .insert(address!("1111111111111111111111111111111111111111"));
        tx.erc20_transfers.push(ERC20TransferEvent {
            token_address: address!("1111111111111111111111111111111111111111"),
            from_address: address!("0000000000000000000000000000000000000000"),
            to_address: address!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"),
            amount: U256::from(5_000_000_000_000_000_000_u128),
            log_index: 1,
        });

        let reports = manager.update_from_processed_transaction(&tx).unwrap();

        assert_eq!(reports.len(), 1);
        assert!(reports[0].token_state_updated);
        let token = manager
            .token("0x1111111111111111111111111111111111111111")
            .unwrap();
        assert_eq!(token.total_supply_from_transfers(), 5.0);
    }

    #[test]
    fn block_processor_sorts_transactions_and_tracks_block_report() {
        let mut state_manager = TokenStateManager::new(100);
        state_manager.add_token(metadata());
        let mut processor = BlockTokenProcessor::with_state_manager(state_manager);

        let mut pair_tx = tx();
        pair_tx.tx_index = 0;
        pair_tx.hash = b256!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        pair_tx
            .uniswap_v2_pair_created_events
            .push(UniswapV2PairCreatedEvent {
                pair_address: address!("3333333333333333333333333333333333333333"),
                token0: address!("1111111111111111111111111111111111111111"),
                token1: address!("2222222222222222222222222222222222222222"),
                log_index: 1,
            });

        let mut sync_tx = tx();
        sync_tx.tx_index = 1;
        sync_tx.hash = b256!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
        sync_tx.uniswap_v2_syncs.push(UniswapV2SyncEvent {
            pair_address: address!("3333333333333333333333333333333333333333"),
            reserve0: U256::from(100_000_000_000_000_000_000_u128),
            reserve1: U256::from(2_000_000_000_000_000_000_u128),
            log_index: 2,
        });

        let block = ProcessedBlock {
            header: block_header(),
            transactions: vec![block_transaction(sync_tx), block_transaction(pair_tx)],
        };

        let report = processor.process_block(&block);

        assert!(!report.already_processed);
        assert_eq!(report.block_number, 100);
        assert_eq!(report.transaction_count, 2);
        assert_eq!(report.processed_transaction_count, 2);
        assert_eq!(report.failed_transaction_count, 0);
        assert_eq!(report.updated_token_addresses.len(), 1);
        assert_eq!(processor.latest_processed_block, Some(100));
        assert_eq!(processor.start_block, Some(100));
        let token = processor
            .state_manager
            .token("0x1111111111111111111111111111111111111111")
            .unwrap();
        let pool = token
            .uniswap_v2_pool("0x3333333333333333333333333333333333333333")
            .unwrap();
        assert_eq!(pool.base.token_reserve(), 100.0);
        assert_eq!(pool.base.denom_reserve(), 2.0);

        let duplicate = processor.process_block(&block);
        assert!(duplicate.already_processed);
        assert_eq!(duplicate.processed_transaction_count, 0);
    }

    #[tokio::test]
    async fn block_processor_discovers_created_token_with_metadata_provider() {
        let lookups = Rc::new(RefCell::new(Vec::new()));
        let provider = RecordingMetadataProvider {
            metadata: metadata(),
            lookups: lookups.clone(),
        };
        let mut processor = BlockTokenProcessor::new(100);
        let mut creation_tx = tx();
        let creation_hash = creation_tx.hash;
        creation_tx.contract_address = Some(address!("1111111111111111111111111111111111111111"));
        creation_tx
            .contract_creation_events
            .push(ContractCreationEvent {
                contract_address: address!("1111111111111111111111111111111111111111"),
            });

        let block = ProcessedBlock {
            header: block_header(),
            transactions: vec![block_transaction(creation_tx)],
        };

        let report = processor
            .process_block_with_metadata_provider(&block, &provider)
            .await;

        assert_eq!(
            report.created_token_addresses,
            vec!["0x1111111111111111111111111111111111111111"]
        );
        assert_eq!(report.updated_token_addresses.len(), 1);
        let token = processor
            .state_manager
            .token("0x1111111111111111111111111111111111111111")
            .unwrap();
        assert_eq!(token.creation_block, Some(100));
        assert_eq!(
            token.creator_address.as_deref(),
            Some("0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
        );
        assert!(processor
            .token_cache
            .contains_token("0x1111111111111111111111111111111111111111"));
        assert_eq!(lookups.borrow().len(), 1);
        assert_eq!(lookups.borrow()[0].pending_tx_hashes, vec![creation_hash]);
    }

    #[tokio::test]
    async fn block_processor_pipeline_discovers_existing_v2_pool_from_swap() {
        let mut state_manager = TokenStateManager::new(100);
        state_manager.add_token(metadata());
        let mut processor = BlockTokenProcessor::with_state_manager(state_manager);
        let token_metadata_provider = StaticTokenMetadataProvider::default();
        let pool_metadata_provider =
            StaticUniswapV2PoolMetadataProvider::new([UniswapV2PoolMetadata::new(
                "0x3333333333333333333333333333333333333333",
                "0x2222222222222222222222222222222222222222",
                "0x1111111111111111111111111111111111111111",
                6,
                18,
            )]);

        let mut swap_tx = tx();
        swap_tx.uniswap_v2_swaps.push(UniswapV2SwapEvent {
            pair_address: address!("3333333333333333333333333333333333333333"),
            sender: address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
            to: address!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"),
            amount0_in: U256::from(2_000_000_u64),
            amount1_in: U256::ZERO,
            amount0_out: U256::ZERO,
            amount1_out: U256::from(50_000_000_000_000_000_000_u128),
            log_index: 1,
        });
        swap_tx.uniswap_v2_syncs.push(UniswapV2SyncEvent {
            pair_address: address!("3333333333333333333333333333333333333333"),
            reserve0: U256::from(2_000_000_u64),
            reserve1: U256::from(100_000_000_000_000_000_000_u128),
            log_index: 2,
        });

        let block = ProcessedBlock {
            header: block_header(),
            transactions: vec![block_transaction(swap_tx)],
        };

        let report = processor
            .process_block_with_metadata_and_v2_pool_provider(
                &block,
                &token_metadata_provider,
                &pool_metadata_provider,
            )
            .await;

        assert_eq!(report.failed_transaction_count, 0);
        assert_eq!(
            report.updated_token_addresses,
            vec!["0x1111111111111111111111111111111111111111"]
        );
        assert_eq!(
            processor
                .token_cache
                .token_for_pool("0x3333333333333333333333333333333333333333"),
            Some("0x1111111111111111111111111111111111111111")
        );
        let token = processor
            .state_manager
            .token("0x1111111111111111111111111111111111111111")
            .unwrap();
        let pool = token
            .uniswap_v2_pool("0x3333333333333333333333333333333333333333")
            .unwrap();
        assert_eq!(pool.base.denom_reserve(), 2.0);
    }

    #[test]
    fn token_state_cache_indexes_pool_to_token_mapping() {
        let mut manager = TokenStateManager::new(100);
        manager.add_token(metadata());
        let mut tx = tx();
        tx.uniswap_v2_pair_created_events
            .push(UniswapV2PairCreatedEvent {
                pair_address: address!("3333333333333333333333333333333333333333"),
                token0: address!("1111111111111111111111111111111111111111"),
                token1: address!("2222222222222222222222222222222222222222"),
                log_index: 1,
            });
        manager.update_from_processed_transaction(&tx).unwrap();

        let cache = TokenStateCache::from_state_manager(&manager, 100);

        assert_eq!(
            cache.token_for_pool("0x3333333333333333333333333333333333333333"),
            Some("0x1111111111111111111111111111111111111111")
        );
        assert_eq!(
            cache.resolve_token_address("0x3333333333333333333333333333333333333333"),
            Some("0x1111111111111111111111111111111111111111")
        );
    }

    #[test]
    fn token_state_builder_replays_processed_transactions_in_order() {
        let builder = TokenStateBuilder::new(metadata(), 100);
        let mut creation_tx = tx();
        creation_tx.tx_index = 0;
        creation_tx.contract_address = Some(address!("1111111111111111111111111111111111111111"));

        let mut pair_tx = tx();
        pair_tx.tx_index = 1;
        pair_tx
            .uniswap_v2_pair_created_events
            .push(UniswapV2PairCreatedEvent {
                pair_address: address!("3333333333333333333333333333333333333333"),
                token0: address!("1111111111111111111111111111111111111111"),
                token1: address!("2222222222222222222222222222222222222222"),
                log_index: 1,
            });

        let mut sync_tx = tx();
        sync_tx.tx_index = 2;
        sync_tx.uniswap_v2_syncs.push(UniswapV2SyncEvent {
            pair_address: address!("3333333333333333333333333333333333333333"),
            reserve0: U256::from(100_000_000_000_000_000_000_u128),
            reserve1: U256::from(2_000_000_000_000_000_000_u128),
            log_index: 2,
        });

        let token = builder
            .build_from_processed_transactions([sync_tx, creation_tx, pair_tx])
            .unwrap();

        assert_eq!(token.creation_block, Some(100));
        let pool = token
            .uniswap_v2_pool("0x3333333333333333333333333333333333333333")
            .unwrap();
        assert_eq!(pool.base.token_reserve(), 100.0);
        assert_eq!(pool.base.denom_reserve(), 2.0);
    }
}
