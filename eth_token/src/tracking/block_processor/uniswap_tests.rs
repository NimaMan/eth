use alloy_primitives::{address, b256, Bytes, U256};
use reth_chain_query::provider::{BlockHeader, TransactionData, TransactionReceipt};
use tx_processor::tx_processor::data_models::{
    UniswapV3InitializeEvent, UniswapV3MintEvent, UniswapV3PoolCreatedEvent,
    UniswapV4InitializeEvent, UniswapV4ModifyLiquidityEvent,
};
use tx_processor::{ProcessedBlock, ProcessedBlockTransactions, ProcessedTransaction};

use crate::erc20::ERC20TokenMetadata;
use crate::tracking::{BlockTokenProcessor, TokenRegistry};

const TOKEN_ADDRESS: &str = "0x1111111111111111111111111111111111111111";
const TOKEN_RAW: alloy_primitives::Address = address!("1111111111111111111111111111111111111111");
const DENOM_RAW: alloy_primitives::Address = address!("2222222222222222222222222222222222222222");
const V3_POOL_ADDRESS: &str = "0x3333333333333333333333333333333333333333";
const V3_POOL_RAW: alloy_primitives::Address = address!("3333333333333333333333333333333333333333");
const POOL_MANAGER_RAW: alloy_primitives::Address =
    address!("000000000004444c5dc75cb358380d2e3de08a90");
const ZERO_RAW: alloy_primitives::Address = address!("0000000000000000000000000000000000000000");

fn metadata() -> ERC20TokenMetadata {
    ERC20TokenMetadata::new(TOKEN_ADDRESS, "Token", "TKN", 18, "100000000000000000000")
}

fn processor_with_token() -> BlockTokenProcessor {
    let mut registry = TokenRegistry::new();
    registry.add_token(metadata());
    BlockTokenProcessor::with_registry(registry, 100)
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

fn tx(tx_index: u64, hash: alloy_primitives::B256) -> ProcessedTransaction {
    ProcessedTransaction::new(
        hash,
        100,
        1_700,
        tx_index,
        address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
        None,
        U256::ZERO,
        true,
        0,
        2,
        Vec::new(),
    )
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

fn block(transactions: Vec<ProcessedTransaction>) -> ProcessedBlock {
    ProcessedBlock {
        header: block_header(),
        transactions: transactions.into_iter().map(block_transaction).collect(),
    }
}

#[tokio::test]
async fn block_processor_discovers_updates_and_indexes_uniswap_v3_pool() {
    let mut processor = processor_with_token();
    let sqrt_price_x96 = U256::from(1u128) << 96;
    let mut create_tx = tx(
        0,
        b256!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
    );
    create_tx.uniswap_v3_pools.push(UniswapV3PoolCreatedEvent {
        factory_address: address!("1f98431c8ad98523631ae4a59f267346ea31f984"),
        token0: TOKEN_RAW,
        token1: DENOM_RAW,
        fee: 3000,
        tick_spacing: 60,
        pool: V3_POOL_RAW,
        log_index: 1,
    });
    create_tx
        .uniswap_v3_initializations
        .push(UniswapV3InitializeEvent {
            pool_address: V3_POOL_RAW,
            sqrt_price_x96,
            tick: 0,
            log_index: 2,
        });
    create_tx.uniswap_v3_mints.push(UniswapV3MintEvent {
        pool_address: V3_POOL_RAW,
        sender: address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
        owner: address!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"),
        tick_lower: -60,
        tick_upper: 60,
        amount: U256::from(1_000_000_000_000_000_000u128),
        amount0: U256::ZERO,
        amount1: U256::ZERO,
        log_index: 3,
    });

    let report = processor
        .process_block_with_test_simulator(&block(vec![create_tx]))
        .await;

    assert_eq!(report.failed_transaction_count, 0);
    assert_eq!(report.processed_transaction_count, 1);
    assert_eq!(report.updated_token_addresses, vec![TOKEN_ADDRESS]);
    assert_eq!(report.token_updates.len(), 1);
    assert_eq!(
        report.token_updates[0].discovered_uniswap_v3_pools,
        vec![V3_POOL_ADDRESS]
    );
    assert_eq!(
        report.token_updates[0].updated_uniswap_v3_pools,
        vec![V3_POOL_ADDRESS]
    );
    assert_eq!(
        report.token_updates[0].simulated_uniswap_v3_pools,
        vec![V3_POOL_ADDRESS]
    );
    assert_eq!(
        processor.token_index.token_for_pool(V3_POOL_ADDRESS),
        Some(TOKEN_ADDRESS)
    );

    let token = processor.registry.token(TOKEN_ADDRESS).unwrap();
    let pool = token.uniswap_v3_pool(V3_POOL_ADDRESS).unwrap();
    assert_eq!(pool.fee_tier, 3000);
    assert_eq!(pool.current_tick, Some(0));
    assert_eq!(pool.active_liquidity, 1_000_000_000_000_000_000u128);
    assert_eq!(pool.base.price(), 1.0);
}

#[tokio::test]
async fn block_processor_discovers_updates_and_indexes_uniswap_v4_pool() {
    let mut processor = processor_with_token();
    let sqrt_price_x96 = U256::from(1u128) << 96;
    let mut init_tx = tx(
        0,
        b256!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"),
    );
    let pool_id = b256!("1111111111111111111111111111111111111111111111111111111111111111");
    init_tx
        .uniswap_v4_initializes
        .push(UniswapV4InitializeEvent {
            pool_manager_address: POOL_MANAGER_RAW,
            event_id: pool_id,
            currency0: ZERO_RAW,
            currency1: TOKEN_RAW,
            fee: 3000,
            tick_spacing: 60,
            hooks: ZERO_RAW,
            sqrt_price_x96,
            tick: 0,
            log_index: 1,
        });
    init_tx
        .uniswap_v4_modifies
        .push(UniswapV4ModifyLiquidityEvent {
            pool_manager_address: POOL_MANAGER_RAW,
            event_id: pool_id,
            sender: address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
            tick_lower: -60,
            tick_upper: 60,
            liquidity_delta: 1_000_000_000_000_000_000i128,
            salt: b256!("0000000000000000000000000000000000000000000000000000000000000000"),
            log_index: 2,
        });

    let report = processor
        .process_block_with_test_simulator(&block(vec![init_tx]))
        .await;

    assert_eq!(report.failed_transaction_count, 0);
    assert_eq!(report.processed_transaction_count, 1);
    assert_eq!(report.updated_token_addresses, vec![TOKEN_ADDRESS]);
    assert_eq!(report.token_updates.len(), 1);
    assert_eq!(report.token_updates[0].discovered_uniswap_v4_pools.len(), 1);
    assert_eq!(report.token_updates[0].updated_uniswap_v4_pools.len(), 1);
    assert_eq!(report.token_updates[0].simulated_uniswap_v4_pools.len(), 0);

    let pool_key = report.token_updates[0].discovered_uniswap_v4_pools[0].clone();
    assert_eq!(
        processor.token_index.token_for_pool(&pool_key),
        Some(TOKEN_ADDRESS)
    );

    let token = processor.registry.token(TOKEN_ADDRESS).unwrap();
    let pool = token.uniswap_v4_pool(&pool_key).unwrap();
    assert_eq!(pool.pool_manager_address, format!("{POOL_MANAGER_RAW:#x}"));
    assert_eq!(pool.pool_id, format!("{pool_id:#x}"));
    assert_eq!(
        pool.pool_key.currency0,
        "0x0000000000000000000000000000000000000000"
    );
    assert_eq!(
        pool.base.identity.denom_address,
        "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2"
    );
    assert_eq!(pool.current_tick, Some(0));
    assert_eq!(pool.active_liquidity, 1_000_000_000_000_000_000u128);
    assert_eq!(pool.base.price(), 1.0);
    assert!(!pool.base.state.can_buy);
    assert!(!pool.base.state.can_sell);
    assert!(pool.base.last_trading_failure_class.is_none());
}
