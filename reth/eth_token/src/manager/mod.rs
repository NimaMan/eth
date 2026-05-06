//! Block-level token orchestration over processed Rust transactions.

use std::collections::HashMap;

use alloy_primitives::{Address, B256};
use serde::{Deserialize, Serialize};

use crate::erc20::{ERC20Token, ERC20TokenMetadata};

pub mod block_processor;
pub mod index;
pub mod metadata;
pub mod token_builder;
pub mod update_router;

pub use block_processor::{
    BlockTokenProcessor, TokenBlockUpdateReport, TokenTransactionUpdateError,
    DEFAULT_TRACKED_TOKEN_INDEX_SIZE,
};
pub use index::{TrackedTokenIndex, TrackedTokenIndexEntry, TrackedTokenStatus};
pub use metadata::{
    NoopUniswapV2PoolMetadataProvider, RethChainDiscoveryProvider, StaticTokenMetadataProvider,
    StaticUniswapV2PoolMetadataProvider, TokenDiscoveryProvider, TokenMetadataLookup,
    TokenMetadataProvider, UniswapV2PoolMetadata, UniswapV2PoolMetadataLookup,
    UniswapV2PoolMetadataProvider,
};
pub use token_builder::TokenStateBuilder;
pub use update_router::ProcessedTokenUpdateRouter;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TokenStateUpdateReport {
    pub token_address: String,
    pub token_state_updated: bool,
    pub discovered_uniswap_v2_pools: Vec<String>,
    pub updated_uniswap_v2_pools: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TokenRegistry {
    pub tokens: HashMap<String, ERC20Token>,
}

impl TokenRegistry {
    pub fn new() -> Self {
        Self {
            tokens: HashMap::new(),
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

    pub fn token_addresses(&self) -> Vec<String> {
        self.tokens.keys().cloned().collect()
    }
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
        ContractCreationEvent, ERC20TransferEvent, UniswapV2MintEvent, UniswapV2PairCreatedEvent,
        UniswapV2SwapEvent, UniswapV2SyncEvent,
    };
    use tx_processor::{ProcessedBlock, ProcessedBlockTransactions, ProcessedTransaction};

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

    #[derive(Clone)]
    struct NonV2PoolMetadataProvider;

    impl UniswapV2PoolMetadataProvider for NonV2PoolMetadataProvider {
        fn uniswap_v2_pool_metadata<'a>(
            &'a self,
            _lookup: &'a UniswapV2PoolMetadataLookup,
        ) -> Pin<Box<dyn Future<Output = eyre::Result<Option<UniswapV2PoolMetadata>>> + 'a>>
        {
            Box::pin(async { Err(eyre::eyre!("token0() view call failed or empty output")) })
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
        let mut registry = TokenRegistry::new();
        let update_router = ProcessedTokenUpdateRouter::new(100);
        registry.add_token(metadata());
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

        let token_index = TrackedTokenIndex::from_registry(&registry, 100);
        let reports = update_router
            .update_registry_from_processed_transaction(&mut registry, &token_index, &tx)
            .unwrap();

        assert_eq!(reports.len(), 1);
        assert!(!reports[0].token_state_updated);
        assert_eq!(reports[0].discovered_uniswap_v2_pools.len(), 1);
        assert_eq!(reports[0].updated_uniswap_v2_pools.len(), 1);
        let token = registry
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
        let mut registry = TokenRegistry::new();
        let update_router = ProcessedTokenUpdateRouter::new(100);
        registry.add_token(metadata());
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

        let token_index = TrackedTokenIndex::from_registry(&registry, 100);
        let reports = update_router
            .update_registry_from_processed_transaction_with_discovery(
                &mut registry,
                &token_index,
                &tx,
                &pool_metadata,
            )
            .await
            .unwrap();

        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].discovered_uniswap_v2_pools.len(), 1);
        assert_eq!(reports[0].updated_uniswap_v2_pools.len(), 1);
        let token = registry
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
        let mut registry = TokenRegistry::new();
        let update_router = ProcessedTokenUpdateRouter::new(100);
        registry.add_token(metadata());
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

        let token_index = TrackedTokenIndex::from_registry(&registry, 100);
        let reports = update_router
            .update_registry_from_processed_transaction(&mut registry, &token_index, &tx)
            .unwrap();

        assert_eq!(reports.len(), 1);
        assert!(reports[0].token_state_updated);
        let token = registry
            .token("0x1111111111111111111111111111111111111111")
            .unwrap();
        assert_eq!(token.total_supply_from_transfers(), 5.0);
    }

    #[test]
    fn block_processor_sorts_transactions_and_tracks_block_report() {
        let mut registry = TokenRegistry::new();
        registry.add_token(metadata());
        let mut processor = BlockTokenProcessor::with_registry(registry, 100);

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
            .registry
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
            .registry
            .token("0x1111111111111111111111111111111111111111")
            .unwrap();
        assert_eq!(token.creation_block, Some(100));
        assert_eq!(
            token.creator_address.as_deref(),
            Some("0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
        );
        assert!(processor
            .token_index
            .contains_token("0x1111111111111111111111111111111111111111"));
        assert_eq!(lookups.borrow().len(), 1);
        assert_eq!(lookups.borrow()[0].pending_tx_hashes, vec![creation_hash]);
    }

    #[tokio::test]
    async fn block_processor_discovery_provider_discovers_existing_v2_pool_from_swap() {
        let mut registry = TokenRegistry::new();
        registry.add_token(metadata());
        let mut processor = BlockTokenProcessor::with_registry(registry, 100);
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
            .process_block_with_token_and_pool_discovery_providers(
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
                .token_index
                .token_for_pool("0x3333333333333333333333333333333333333333"),
            Some("0x1111111111111111111111111111111111111111")
        );
        let token = processor
            .registry
            .token("0x1111111111111111111111111111111111111111")
            .unwrap();
        let pool = token
            .uniswap_v2_pool("0x3333333333333333333333333333333333333333")
            .unwrap();
        assert_eq!(pool.base.denom_reserve(), 2.0);
    }

    #[tokio::test]
    async fn block_processor_ignores_false_positive_v2_mint_metadata_miss() {
        let mut processor = BlockTokenProcessor::new(100);
        let token_metadata_provider = StaticTokenMetadataProvider::default();
        let pool_metadata_provider = NonV2PoolMetadataProvider;

        let mut mint_tx = tx();
        mint_tx.uniswap_v2_mints.push(UniswapV2MintEvent {
            pair_address: address!("8eea6cc08d824b20efb3bf7c248de694cb1f75f4"),
            sender: address!("c4dcb059dd98b45b090da8982234c61d0b9e84f9"),
            amount0: U256::from(172_144_196_175_146_071_u128),
            amount1: U256::from(114_373_457_271_042_510_297_504_u128),
            log_index: 1,
        });

        let block = ProcessedBlock {
            header: block_header(),
            transactions: vec![block_transaction(mint_tx)],
        };

        let report = processor
            .process_block_with_token_and_pool_discovery_providers(
                &block,
                &token_metadata_provider,
                &pool_metadata_provider,
            )
            .await;

        assert_eq!(report.transaction_count, 1);
        assert_eq!(report.processed_transaction_count, 1);
        assert_eq!(report.failed_transaction_count, 0);
        assert!(report.transaction_errors.is_empty());
        assert!(processor.registry.tokens.is_empty());
    }

    #[test]
    fn tracked_token_index_indexes_pool_to_token_mapping() {
        let mut registry = TokenRegistry::new();
        let update_router = ProcessedTokenUpdateRouter::new(100);
        registry.add_token(metadata());
        let mut tx = tx();
        tx.uniswap_v2_pair_created_events
            .push(UniswapV2PairCreatedEvent {
                pair_address: address!("3333333333333333333333333333333333333333"),
                token0: address!("1111111111111111111111111111111111111111"),
                token1: address!("2222222222222222222222222222222222222222"),
                log_index: 1,
            });
        let token_index = TrackedTokenIndex::from_registry(&registry, 100);
        update_router
            .update_registry_from_processed_transaction(&mut registry, &token_index, &tx)
            .unwrap();

        let index = TrackedTokenIndex::from_registry(&registry, 100);

        assert_eq!(
            index.token_for_pool("0x3333333333333333333333333333333333333333"),
            Some("0x1111111111111111111111111111111111111111")
        );
        assert_eq!(
            index.resolve_token_address("0x3333333333333333333333333333333333333333"),
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
