use super::*;
use crate::erc20::ERC20TokenMetadata;
use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

use alloy_primitives::{address, b256, Address, Bytes, U256};
use reth_chain_query::common_addresses::KnownV2Protocol;
use reth_chain_query::provider::{BlockHeader, TransactionData, TransactionReceipt};
use tx_processor::tx_processor::data_models::{
    ContractCreationEvent, ERC20TransferEvent, UniswapV2MintEvent, UniswapV2PairCreatedEvent,
    UniswapV2SwapEvent, UniswapV2SyncEvent, UniswapV3InitializeEvent, UniswapV3MintEvent,
    UniswapV3PoolCreatedEvent, UniswapV4InitializeEvent, UniswapV4SwapEvent,
};
use tx_processor::{ProcessedBlock, ProcessedBlockTransactions, ProcessedTransaction};

use crate::chain_metadata::{
    StaticTokenMetadataProvider, StaticUniswapV2PoolMetadataProvider, TokenMetadataLookup,
    TokenMetadataProvider, UniswapV2PoolMetadata, UniswapV2PoolMetadataLookup,
    UniswapV2PoolMetadataProvider,
};
use crate::pools::SUSHISWAP_V2_PROTOCOL;

pub(crate) fn metadata() -> ERC20TokenMetadata {
    ERC20TokenMetadata::new(
        "0x1111111111111111111111111111111111111111",
        "Token",
        "TKN",
        18,
        "100000000000000000000",
    )
}

pub(crate) fn tx() -> ProcessedTransaction {
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
    ) -> Pin<Box<dyn Future<Output = eyre::Result<Option<UniswapV2PoolMetadata>>> + 'a>> {
        Box::pin(async { Err(eyre::eyre!("token0() view call failed or empty output")) })
    }
}

#[derive(Clone)]
struct RecordingV2PoolMetadataProvider {
    lookups: Rc<RefCell<usize>>,
}

impl UniswapV2PoolMetadataProvider for RecordingV2PoolMetadataProvider {
    fn uniswap_v2_pool_metadata<'a>(
        &'a self,
        _lookup: &'a UniswapV2PoolMetadataLookup,
    ) -> Pin<Box<dyn Future<Output = eyre::Result<Option<UniswapV2PoolMetadata>>> + 'a>> {
        Box::pin(async move {
            *self.lookups.borrow_mut() += 1;
            Ok(None)
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

#[tokio::test]
async fn pair_created_discovery_does_not_call_v2_pool_metadata_provider() {
    let mut registry = TokenRegistry::new();
    let update_router = ProcessedTokenUpdateRouter::new(100);
    registry.add_token(metadata());
    let mut tx = tx();
    tx.uniswap_v2_pair_created_events
        .push(UniswapV2PairCreatedEvent {
            pair_address: address!("3333333333333333333333333333333333333333"),
            token0: address!("1111111111111111111111111111111111111111"),
            token1: address!("2222222222222222222222222222222222222222"),
            factory_address: address!("5c69bee701ef814a2b6a3edd4b1652cb9cc5aa6f"),
            log_index: 1,
        });

    let lookups = Rc::new(RefCell::new(0));
    let provider = RecordingV2PoolMetadataProvider {
        lookups: lookups.clone(),
    };
    let token_index = TrackedTokenIndex::from_registry(&registry, 100);
    let reports = update_router
        .update_registry_from_processed_transaction_with_discovery(
            &mut registry,
            &token_index,
            &tx,
            &provider,
        )
        .await
        .unwrap();

    assert_eq!(*lookups.borrow(), 0);
    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].discovered_known_v2_pools.len(), 1);
}

#[tokio::test]
async fn pair_created_discovery_uses_metadata_when_factory_is_missing() {
    let mut registry = TokenRegistry::new();
    let update_router = ProcessedTokenUpdateRouter::new(100);
    registry.add_token(metadata());
    let mut tx = tx();
    tx.uniswap_v2_pair_created_events
        .push(UniswapV2PairCreatedEvent {
            pair_address: address!("3333333333333333333333333333333333333333"),
            token0: address!("2222222222222222222222222222222222222222"),
            token1: address!("1111111111111111111111111111111111111111"),
            factory_address: Address::ZERO,
            log_index: 1,
        });
    tx.uniswap_v2_syncs.push(UniswapV2SyncEvent {
        pair_address: address!("3333333333333333333333333333333333333333"),
        reserve0: U256::from(2_000_000_u64),
        reserve1: U256::from(100_000_000_000_000_000_000_u128),
        log_index: 2,
    });

    let pool_metadata =
        StaticUniswapV2PoolMetadataProvider::new([UniswapV2PoolMetadata::new_with_protocol(
            KnownV2Protocol::PancakeSwapV2,
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
    assert_eq!(reports[0].discovered_known_v2_pools.len(), 1);
    assert_eq!(reports[0].updated_known_v2_pools.len(), 1);
    let token = registry
        .token("0x1111111111111111111111111111111111111111")
        .unwrap();
    let pool = token
        .uniswap_v2_pool("0x3333333333333333333333333333333333333333")
        .unwrap();
    assert_eq!(
        pool.base.identity.protocol,
        KnownV2Protocol::PancakeSwapV2.label()
    );
    assert_eq!(pool.base.config.denom_decimals, Some(6));
    assert_eq!(pool.base.config.token1_is_denom, Some(false));
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
            factory_address: address!("5c69bee701ef814a2b6a3edd4b1652cb9cc5aa6f"),
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
    assert_eq!(reports[0].discovered_known_v2_pools.len(), 1);
    assert_eq!(reports[0].updated_known_v2_pools.len(), 1);
    let token = registry
        .token("0x1111111111111111111111111111111111111111")
        .unwrap();
    let pool = token
        .uniswap_v2_pool("0x3333333333333333333333333333333333333333")
        .unwrap();
    assert_eq!(pool.base.token_reserve(), 100.0);
    assert_eq!(pool.base.denom_reserve(), 2.0);
}

#[test]
fn discovers_and_updates_sushiswap_v2_pool_for_tracked_token() {
    let mut registry = TokenRegistry::new();
    let update_router = ProcessedTokenUpdateRouter::new(100);
    registry.add_token(metadata());
    let mut tx = tx();
    tx.uniswap_v2_pair_created_events
        .push(UniswapV2PairCreatedEvent {
            pair_address: address!("3333333333333333333333333333333333333333"),
            token0: address!("1111111111111111111111111111111111111111"),
            token1: address!("2222222222222222222222222222222222222222"),
            factory_address: address!("c0aee478e3658e2610c5f7a4a2e1777ce9e4f2ac"),
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
    assert_eq!(reports[0].discovered_known_v2_pools.len(), 1);
    assert_eq!(reports[0].updated_known_v2_pools.len(), 1);
    let token = registry
        .token("0x1111111111111111111111111111111111111111")
        .unwrap();
    let pool = token
        .uniswap_v2_pool("0x3333333333333333333333333333333333333333")
        .unwrap();
    assert_eq!(pool.base.identity.protocol, SUSHISWAP_V2_PROTOCOL);
    assert_eq!(pool.base.token_reserve(), 100.0);
    assert_eq!(pool.base.denom_reserve(), 2.0);
}

#[test]
fn discovers_and_updates_pancakeswap_v2_pool_for_tracked_token() {
    let mut registry = TokenRegistry::new();
    let update_router = ProcessedTokenUpdateRouter::new(100);
    registry.add_token(metadata());
    let mut tx = tx();
    tx.uniswap_v2_pair_created_events
        .push(UniswapV2PairCreatedEvent {
            pair_address: address!("3333333333333333333333333333333333333333"),
            token0: address!("1111111111111111111111111111111111111111"),
            token1: address!("2222222222222222222222222222222222222222"),
            factory_address: KnownV2Protocol::PancakeSwapV2.factory(),
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
    assert_eq!(reports[0].discovered_known_v2_pools.len(), 1);
    assert_eq!(reports[0].updated_known_v2_pools.len(), 1);
    let token = registry
        .token("0x1111111111111111111111111111111111111111")
        .unwrap();
    let pool = token
        .uniswap_v2_pool("0x3333333333333333333333333333333333333333")
        .unwrap();
    assert_eq!(
        pool.base.identity.protocol,
        KnownV2Protocol::PancakeSwapV2.label()
    );
    assert_eq!(pool.base.token_reserve(), 100.0);
    assert_eq!(pool.base.denom_reserve(), 2.0);
}

#[test]
fn discovers_and_updates_uniswap_v3_pool_for_tracked_token() {
    let mut registry = TokenRegistry::new();
    let update_router = ProcessedTokenUpdateRouter::new(100);
    registry.add_token(metadata());
    let mut tx = tx();
    let sqrt = U256::from(1u128) << 96;
    tx.uniswap_v3_pools.push(UniswapV3PoolCreatedEvent {
        token0: address!("1111111111111111111111111111111111111111"),
        token1: address!("2222222222222222222222222222222222222222"),
        fee: 3000,
        tick_spacing: 60,
        pool: address!("3333333333333333333333333333333333333333"),
        log_index: 1,
    });
    tx.uniswap_v3_initializations
        .push(UniswapV3InitializeEvent {
            pool_address: address!("3333333333333333333333333333333333333333"),
            sqrt_price_x96: sqrt,
            tick: 0,
            log_index: 2,
        });
    tx.uniswap_v3_mints.push(UniswapV3MintEvent {
        pool_address: address!("3333333333333333333333333333333333333333"),
        sender: address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
        owner: address!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"),
        tick_lower: -60,
        tick_upper: 60,
        amount: U256::from(1_000_000_000_000_000_000u128),
        amount0: U256::ZERO,
        amount1: U256::ZERO,
        log_index: 3,
    });

    let token_index = TrackedTokenIndex::from_registry(&registry, 100);
    let reports = update_router
        .update_registry_from_processed_transaction(&mut registry, &token_index, &tx)
        .unwrap();

    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].discovered_uniswap_v3_pools.len(), 1);
    assert_eq!(reports[0].updated_uniswap_v3_pools.len(), 1);
    let token = registry
        .token("0x1111111111111111111111111111111111111111")
        .unwrap();
    let pool = token
        .uniswap_v3_pool("0x3333333333333333333333333333333333333333")
        .unwrap();
    assert_eq!(pool.fee_tier, 3000);
    assert_eq!(pool.base.price(), 1.0);
    assert_eq!(pool.active_liquidity, 1_000_000_000_000_000_000u128);
}

#[test]
fn discovers_v4_only_from_initialize_and_ignores_unknown_pool_id() {
    let mut registry = TokenRegistry::new();
    let update_router = ProcessedTokenUpdateRouter::new(100);
    registry.add_token(metadata());
    let mut init_tx = tx();
    let sqrt = U256::from(1u128) << 96;
    init_tx
        .uniswap_v4_initializes
        .push(UniswapV4InitializeEvent {
            pool_manager_address: address!("000000000004444c5dc75cb358380d2e3de08a90"),
            event_id: b256!("1111111111111111111111111111111111111111111111111111111111111111"),
            currency0: address!("0000000000000000000000000000000000000000"),
            currency1: address!("1111111111111111111111111111111111111111"),
            fee: 3000,
            tick_spacing: 60,
            hooks: address!("0000000000000000000000000000000000000000"),
            sqrt_price_x96: sqrt,
            tick: 0,
            log_index: 1,
        });

    let token_index = TrackedTokenIndex::from_registry(&registry, 100);
    let reports = update_router
        .update_registry_from_processed_transaction(&mut registry, &token_index, &init_tx)
        .unwrap();

    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].discovered_uniswap_v4_pools.len(), 1);
    assert_eq!(reports[0].updated_uniswap_v4_pools.len(), 1);
    let token = registry
        .token("0x1111111111111111111111111111111111111111")
        .unwrap();
    assert_eq!(token.v4_pools.len(), 1);
    let pool_key = token.v4_pools.keys().next().cloned().unwrap();
    let pool = token.uniswap_v4_pool(&pool_key).unwrap();
    assert_eq!(
        pool.pool_key.currency0,
        "0x0000000000000000000000000000000000000000"
    );
    assert_eq!(
        pool.base.identity.denom_address,
        "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2"
    );

    let token_index = TrackedTokenIndex::from_registry(&registry, 100);
    let mut unknown = tx();
    unknown.uniswap_v4_swaps.push(UniswapV4SwapEvent {
        pool_manager_address: address!("000000000004444c5dc75cb358380d2e3de08a90"),
        event_id: b256!("2222222222222222222222222222222222222222222222222222222222222222"),
        sender: address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
        amount0: -1,
        amount1: 1,
        sqrt_price_x96: sqrt,
        liquidity: 1_000_000_000_000_000_000u128,
        tick: 0,
        fee: 3000,
        log_index: 1,
    });
    let reports = update_router
        .update_registry_from_processed_transaction(&mut registry, &token_index, &unknown)
        .unwrap();

    assert!(reports.is_empty());
    let token = registry
        .token("0x1111111111111111111111111111111111111111")
        .unwrap();
    assert_eq!(token.v4_pools.len(), 1);
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
    assert_eq!(reports[0].discovered_known_v2_pools.len(), 1);
    assert_eq!(reports[0].updated_known_v2_pools.len(), 1);
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

#[tokio::test]
async fn discovers_pancakeswap_v2_pool_from_swap_metadata_for_tracked_token() {
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

    let pool_metadata =
        StaticUniswapV2PoolMetadataProvider::new([UniswapV2PoolMetadata::new_with_protocol(
            KnownV2Protocol::PancakeSwapV2,
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
    assert_eq!(reports[0].discovered_known_v2_pools.len(), 1);
    assert_eq!(reports[0].updated_known_v2_pools.len(), 1);
    let token = registry
        .token("0x1111111111111111111111111111111111111111")
        .unwrap();
    let pool = token
        .uniswap_v2_pool("0x3333333333333333333333333333333333333333")
        .unwrap();
    assert_eq!(
        pool.base.identity.protocol,
        KnownV2Protocol::PancakeSwapV2.label()
    );
    assert_eq!(pool.base.config.denom_decimals, Some(6));
    assert_eq!(pool.base.config.token1_is_denom, Some(false));
}

#[tokio::test]
async fn ignores_uniswap_v2_shaped_pool_event_when_metadata_provider_skips_pool() {
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

    let pool_metadata = StaticUniswapV2PoolMetadataProvider::default();
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

    assert!(reports.is_empty());
    let token = registry
        .token("0x1111111111111111111111111111111111111111")
        .unwrap();
    assert!(token
        .uniswap_v2_pool("0x3333333333333333333333333333333333333333")
        .is_none());
}

#[tokio::test]
async fn discovers_uniswap_v2_pool_from_liquidity_metadata_for_tracked_token() {
    let mut registry = TokenRegistry::new();
    let update_router = ProcessedTokenUpdateRouter::new(100);
    registry.add_token(metadata());
    let mut tx = tx();
    tx.erc20_transfers.push(ERC20TransferEvent {
        token_address: address!("3333333333333333333333333333333333333333"),
        from_address: address!("0000000000000000000000000000000000000000"),
        to_address: address!("0000000000000000000000000000000000000000"),
        amount: U256::from(1_000_u64),
        log_index: 1,
    });
    tx.erc20_transfers.push(ERC20TransferEvent {
        token_address: address!("3333333333333333333333333333333333333333"),
        from_address: address!("0000000000000000000000000000000000000000"),
        to_address: address!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"),
        amount: U256::from(2_000_000_000_000_000_000_u128),
        log_index: 2,
    });
    tx.uniswap_v2_syncs.push(UniswapV2SyncEvent {
        pair_address: address!("3333333333333333333333333333333333333333"),
        reserve0: U256::from(100_000_000_000_000_000_000_u128),
        reserve1: U256::from(2_000_000_000_000_000_000_u128),
        log_index: 3,
    });
    tx.uniswap_v2_mints.push(UniswapV2MintEvent {
        pair_address: address!("3333333333333333333333333333333333333333"),
        sender: address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
        amount0: U256::from(100_000_000_000_000_000_000_u128),
        amount1: U256::from(2_000_000_000_000_000_000_u128),
        log_index: 4,
    });

    let pool_metadata = StaticUniswapV2PoolMetadataProvider::new([UniswapV2PoolMetadata::new(
        "0x3333333333333333333333333333333333333333",
        "0x1111111111111111111111111111111111111111",
        "0x2222222222222222222222222222222222222222",
        18,
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
    assert_eq!(reports[0].discovered_known_v2_pools.len(), 1);
    assert_eq!(reports[0].updated_known_v2_pools.len(), 1);
    let token = registry
        .token("0x1111111111111111111111111111111111111111")
        .unwrap();
    let pool = token
        .uniswap_v2_pool("0x3333333333333333333333333333333333333333")
        .unwrap();
    assert_eq!(pool.base.token_reserve(), 100.0);
    assert_eq!(pool.base.denom_reserve(), 2.0);
    assert_eq!(pool.lp_tracker.transfers.len(), 2);
    assert_eq!(pool.lp_tracker.total_supply, 2.0);
    assert_eq!(
        pool.lp_share("0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"),
        100.0
    );
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

#[tokio::test]
async fn block_processor_sorts_transactions_and_tracks_block_report() {
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
            factory_address: address!("5c69bee701ef814a2b6a3edd4b1652cb9cc5aa6f"),
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

    let report = processor.process_block_with_test_simulator(&block).await;

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

    let duplicate = processor.process_block_with_test_simulator(&block).await;
    assert!(duplicate.already_processed);
    assert_eq!(duplicate.processed_transaction_count, 0);
}

#[test]
fn block_processor_live_mode_propagates_to_registry_tokens() {
    let mut registry = TokenRegistry::new();
    registry.add_token(metadata());
    let mut processor = BlockTokenProcessor::with_registry(registry, 100);

    assert!(!processor.is_live_mode);
    assert!(
        !processor
            .registry
            .token("0x1111111111111111111111111111111111111111")
            .unwrap()
            .is_live_mode
    );

    processor.set_live_mode(true);

    assert!(processor.is_live_mode);
    assert!(
        processor
            .registry
            .token("0x1111111111111111111111111111111111111111")
            .unwrap()
            .is_live_mode
    );
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
        .process_block_with_metadata_provider_test_simulator(&block, &provider)
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
        .process_block_with_token_and_pool_discovery_providers_test_simulator(
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
        .process_block_with_token_and_pool_discovery_providers_test_simulator(
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
