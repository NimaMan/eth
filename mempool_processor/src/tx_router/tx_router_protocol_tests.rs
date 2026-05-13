use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use alloy_primitives::{address, Address, U256};
use reth_chain_query::common_addresses::POOL_FACTORIES;
use reth_chain_query::to_checksum_address;
use serde_json::json;

use super::{CreatorFunctionType, SimulationPriority, TransactionCategory, TransactionRouter};
use crate::liquidity_approval_call::{PERMIT2_ADDRESS, PERMIT2_APPROVE_SELECTOR};
use crate::mempool_fetcher::MempoolTransaction;
use crate::position_approval_call::SET_APPROVAL_FOR_ALL_SELECTOR;
use crate::token_tracking::types::{ConcentratedLiquidityPosition, PoolLifecycle};
use crate::token_tracking::{
    Pool, PoolType, Token, TokenTrackingCache, TokenUpdate, TokenWithPools,
};

#[tokio::test]
async fn routes_balancer_vault_lp_approval_when_pool_share_is_tracked() {
    let cache = Arc::new(TokenTrackingCache::with_defaults());
    let token_address = "0x1111111111111111111111111111111111111111".to_string();
    let pool_address = "0x4444444444444444444444444444444444444444".to_string();
    hydrate_token_with_pool(&cache, &token_address, &pool_address).await;

    let router = TransactionRouter::new(Some(cache));
    let tx = MempoolTransaction {
        hash: "0xtx".to_string(),
        data: json!({}),
        detection_ns: 0,
        detection_time: Instant::now(),
        latency_ns: 0,
        from: address_bytes("0x2222222222222222222222222222222222222222"),
        to: Some(address_bytes(&pool_address)),
        input: approve_calldata(
            &to_checksum_address(&POOL_FACTORIES["balancer_vault"]),
            U256::from(1_000_000u64),
        ),
        value: U256::ZERO,
        gas_price: Some(U256::ZERO),
        functions: vec!["approve".to_string()],
        function_category: Some(CreatorFunctionType::Other("approve".to_string())),
    };

    let classification = router.classify(&tx).await;
    assert_eq!(classification.priority, SimulationPriority::Critical);
    assert!(!classification.requires_simulation);
    assert!(!classification.requires_buy_sell_test);
    assert!(matches!(
        classification.category,
        TransactionCategory::CreatorTransaction {
            function_type: CreatorFunctionType::LiquidityPoolApproval,
            ..
        }
    ));
}

#[tokio::test]
async fn routes_tracked_lp_approval_to_unknown_spender() {
    let cache = Arc::new(TokenTrackingCache::with_defaults());
    let token_address = "0x1111111111111111111111111111111111111111".to_string();
    let pool_address = "0x4444444444444444444444444444444444444444".to_string();
    hydrate_token_with_pool(&cache, &token_address, &pool_address).await;

    let router = TransactionRouter::new(Some(cache));
    let tx = MempoolTransaction {
        hash: "0xtx".to_string(),
        data: json!({}),
        detection_ns: 0,
        detection_time: Instant::now(),
        latency_ns: 0,
        from: address_bytes("0x2222222222222222222222222222222222222222"),
        to: Some(address_bytes(&pool_address)),
        input: approve_calldata(
            "0x9999999999999999999999999999999999999999",
            U256::from(1_000_000u64),
        ),
        value: U256::ZERO,
        gas_price: Some(U256::ZERO),
        functions: vec!["approve".to_string()],
        function_category: Some(CreatorFunctionType::Other("approve".to_string())),
    };

    let classification = router.classify(&tx).await;
    assert_eq!(classification.priority, SimulationPriority::Critical);
    assert!(!classification.requires_simulation);
    assert!(matches!(
        classification.category,
        TransactionCategory::CreatorTransaction {
            function_type: CreatorFunctionType::LiquidityPoolApproval,
            ..
        }
    ));
}

#[tokio::test]
async fn routes_permit2_approval_when_ownership_token_is_tracked_pool() {
    let cache = Arc::new(TokenTrackingCache::with_defaults());
    let token_address = "0x1111111111111111111111111111111111111111".to_string();
    let pool_address = "0x4444444444444444444444444444444444444444".to_string();
    hydrate_token_with_pool(&cache, &token_address, &pool_address).await;

    let router = TransactionRouter::new(Some(cache));
    let tx = MempoolTransaction {
        hash: "0xtx".to_string(),
        data: json!({}),
        detection_ns: 0,
        detection_time: Instant::now(),
        latency_ns: 0,
        from: address_bytes("0x2222222222222222222222222222222222222222"),
        to: Some(PERMIT2_ADDRESS.to_vec()),
        input: permit2_approve_calldata(
            address_from_hex(&pool_address),
            address!("9999999999999999999999999999999999999999"),
            U256::from(1_000_000u64),
        ),
        value: U256::ZERO,
        gas_price: Some(U256::ZERO),
        functions: vec!["permit2_approve".to_string()],
        function_category: Some(CreatorFunctionType::Other("permit2_approve".to_string())),
    };

    let classification = router.classify(&tx).await;
    assert_eq!(classification.priority, SimulationPriority::Critical);
    assert!(!classification.requires_simulation);
    assert!(matches!(
        classification.category,
        TransactionCategory::CreatorTransaction {
            function_type: CreatorFunctionType::LiquidityPoolApproval,
            ..
        }
    ));
}

#[tokio::test]
async fn routes_concentrated_position_approval_when_token_id_is_mapped() {
    let cache = Arc::new(TokenTrackingCache::with_defaults());
    let token_address = "0x1111111111111111111111111111111111111111".to_string();
    let pool_address = "0x4444444444444444444444444444444444444444".to_string();
    let position_manager = "0xc36442b4a4522e871399cd717abdd847ab11fe88".to_string();
    hydrate_token_with_concentrated_position(
        &cache,
        &token_address,
        &pool_address,
        &position_manager,
    )
    .await;

    let router = TransactionRouter::new(Some(cache));
    let tx = MempoolTransaction {
        hash: "0xtx".to_string(),
        data: json!({}),
        detection_ns: 0,
        detection_time: Instant::now(),
        latency_ns: 0,
        from: address_bytes("0x2222222222222222222222222222222222222222"),
        to: Some(address_bytes(&position_manager)),
        input: approve_calldata(
            "0x9999999999999999999999999999999999999999",
            U256::from(42u64),
        ),
        value: U256::ZERO,
        gas_price: Some(U256::ZERO),
        functions: vec!["approve".to_string()],
        function_category: Some(CreatorFunctionType::Other("approve".to_string())),
    };

    let classification = router.classify(&tx).await;
    assert_eq!(classification.priority, SimulationPriority::Critical);
    assert!(!classification.requires_simulation);
    assert!(matches!(
        classification.category,
        TransactionCategory::CreatorTransaction {
            function_type: CreatorFunctionType::LiquidityPoolApproval,
            ..
        }
    ));
}

#[tokio::test]
async fn routes_concentrated_position_operator_approval_when_owner_has_mapped_position() {
    let cache = Arc::new(TokenTrackingCache::with_defaults());
    let token_address = "0x1111111111111111111111111111111111111111".to_string();
    let pool_address = "0x4444444444444444444444444444444444444444".to_string();
    let position_manager = "0xc36442b4a4522e871399cd717abdd847ab11fe88".to_string();
    hydrate_token_with_concentrated_position(
        &cache,
        &token_address,
        &pool_address,
        &position_manager,
    )
    .await;

    let router = TransactionRouter::new(Some(cache));
    let tx = MempoolTransaction {
        hash: "0xtx".to_string(),
        data: json!({}),
        detection_ns: 0,
        detection_time: Instant::now(),
        latency_ns: 0,
        from: address_bytes("0x2222222222222222222222222222222222222222"),
        to: Some(address_bytes(&position_manager)),
        input: set_approval_for_all_calldata("0x9999999999999999999999999999999999999999", true),
        value: U256::ZERO,
        gas_price: Some(U256::ZERO),
        functions: vec!["setApprovalForAll".to_string()],
        function_category: Some(CreatorFunctionType::Other("setApprovalForAll".to_string())),
    };

    let classification = router.classify(&tx).await;
    assert_eq!(classification.priority, SimulationPriority::Critical);
    assert!(!classification.requires_simulation);
    assert!(matches!(
        classification.category,
        TransactionCategory::CreatorTransaction {
            function_type: CreatorFunctionType::LiquidityPoolApproval,
            ..
        }
    ));
}

#[tokio::test]
async fn routes_balancer_exit_pool_by_composite_pool_identifier() {
    let cache = Arc::new(TokenTrackingCache::with_defaults());
    let token_address = "0x1111111111111111111111111111111111111111".to_string();
    let vault = to_checksum_address(&POOL_FACTORIES["balancer_vault"]);
    let pool_id = [0x99u8; 32];
    let pool_identifier = format!("{}#0x{}", vault, hex::encode(pool_id));
    hydrate_token_with_pool(&cache, &token_address, &pool_identifier).await;

    let router = TransactionRouter::new(Some(cache));
    let tx = MempoolTransaction {
        hash: "0xtx".to_string(),
        data: json!({}),
        detection_ns: 0,
        detection_time: Instant::now(),
        latency_ns: 0,
        from: address_bytes("0x2222222222222222222222222222222222222222"),
        to: Some(address_bytes(&vault)),
        input: exit_pool_calldata(pool_id),
        value: U256::ZERO,
        gas_price: Some(U256::ZERO),
        functions: vec!["exitPool".to_string()],
        function_category: Some(CreatorFunctionType::LiquidityRemoval),
    };

    let classification = router.classify(&tx).await;
    assert_eq!(classification.priority, SimulationPriority::Critical);
    assert!(classification.requires_simulation);
    match classification.category {
        TransactionCategory::CreatorTransaction {
            target_address,
            target_token: Some(_),
            function_type: CreatorFunctionType::LiquidityRemoval,
            ..
        } => assert!(target_address.eq_ignore_ascii_case(&pool_identifier)),
        other => panic!("unexpected category: {other:?}"),
    }
}

async fn hydrate_token_with_pool(
    cache: &TokenTrackingCache,
    token_address: &str,
    pool_address: &str,
) {
    let creator_address = "0x3333333333333333333333333333333333333333".to_string();
    let token = Token {
        address: token_address.to_string(),
        symbol: "TEST".to_string(),
        name: "Test".to_string(),
        decimals: 18,
        total_supply: Some("1000".to_string()),
        creator_address: creator_address.clone(),
        current_owner: creator_address,
        tax_setter_addresses: Vec::new(),
        ownership_renounced: false,
        renouncement_block: None,
        buy_tax: None,
        sell_tax: None,
        last_tax_change_block: None,
        tax_history: Vec::new(),
        creation_block: 1,
        creation_tx: "0xcreation".to_string(),
        creation_timestamp: None,
        latest_activity_block: 1,
        is_scam: false,
        scam_label: None,
        total_liquidity: 0.0,
    };
    let pool = Pool {
        address: pool_address.to_string(),
        token_address: token_address.to_string(),
        pool_type: PoolType::Balancer,
        token_reserve: 1_000.0,
        eth_reserve: 1.0,
        denom_currency: "ETH".to_string(),
        denom_address: "0x0000000000000000000000000000000000000000".to_string(),
        trading_enabled: true,
        trading_enabled_block: Some(1),
        trading_enabled_tx: None,
        fee_tier: None,
        pool_id: None,
        position_manager_address: None,
        lp_total_supply: None,
        liquidity_positions: Vec::new(),
        last_updated_block: 1,
        last_updated_time: 0.0,
        is_scam: false,
        scam_label: None,
        lp_tokens_approved_percentage: Some(25.0),
        lifecycle: PoolLifecycle::Active,
        control_addresses: Vec::new(),
        can_buy: true,
        can_sell: true,
        received_at: Instant::now(),
    };

    let mut pools = HashMap::new();
    pools.insert(pool_address.to_string(), pool);
    let mut data = HashMap::new();
    data.insert(token_address.to_string(), TokenWithPools { token, pools });
    cache
        .batch_update(TokenUpdate {
            message_type: "test".to_string(),
            token_count: 1,
            block_number: 1,
            timestamp: 0.0,
            data,
        })
        .await;
}

async fn hydrate_token_with_concentrated_position(
    cache: &TokenTrackingCache,
    token_address: &str,
    pool_address: &str,
    position_manager: &str,
) {
    let creator_address = "0x3333333333333333333333333333333333333333".to_string();
    let owner = "0x2222222222222222222222222222222222222222".to_string();
    let token = Token {
        address: token_address.to_string(),
        symbol: "TEST".to_string(),
        name: "Test".to_string(),
        decimals: 18,
        total_supply: Some("1000".to_string()),
        creator_address: creator_address.clone(),
        current_owner: creator_address,
        tax_setter_addresses: Vec::new(),
        ownership_renounced: false,
        renouncement_block: None,
        buy_tax: None,
        sell_tax: None,
        last_tax_change_block: None,
        tax_history: Vec::new(),
        creation_block: 1,
        creation_tx: "0xcreation".to_string(),
        creation_timestamp: None,
        latest_activity_block: 1,
        is_scam: false,
        scam_label: None,
        total_liquidity: 0.0,
    };
    let pool = Pool {
        address: pool_address.to_string(),
        token_address: token_address.to_string(),
        pool_type: PoolType::UniswapV3,
        token_reserve: 1_000.0,
        eth_reserve: 1.0,
        denom_currency: "ETH".to_string(),
        denom_address: "0x0000000000000000000000000000000000000000".to_string(),
        trading_enabled: true,
        trading_enabled_block: Some(1),
        trading_enabled_tx: None,
        fee_tier: Some(3000),
        pool_id: None,
        position_manager_address: Some(position_manager.to_string()),
        lp_total_supply: Some(1000.0),
        liquidity_positions: vec![ConcentratedLiquidityPosition {
            position_id: "0x2a".to_string(),
            token_id: Some("0x2a".to_string()),
            owner,
            position_manager_address: Some(position_manager.to_string()),
            liquidity: "250".to_string(),
            position_share_pct: Some(25.0),
            tick_lower: Some(-60),
            tick_upper: Some(60),
            last_update_block: Some(1),
            last_update_tx: Some("0xposition".to_string()),
        }],
        last_updated_block: 1,
        last_updated_time: 0.0,
        is_scam: false,
        scam_label: None,
        lp_tokens_approved_percentage: None,
        lifecycle: PoolLifecycle::Active,
        control_addresses: Vec::new(),
        can_buy: true,
        can_sell: true,
        received_at: Instant::now(),
    };

    let mut pools = HashMap::new();
    pools.insert(pool_address.to_string(), pool);
    let mut data = HashMap::new();
    data.insert(token_address.to_string(), TokenWithPools { token, pools });
    cache
        .batch_update(TokenUpdate {
            message_type: "test".to_string(),
            token_count: 1,
            block_number: 1,
            timestamp: 0.0,
            data,
        })
        .await;
}

fn address_bytes(address: &str) -> Vec<u8> {
    hex::decode(address.trim_start_matches("0x")).unwrap()
}

fn approve_calldata(spender: &str, amount: U256) -> Vec<u8> {
    let mut input = hex::decode("095ea7b3").unwrap();
    input.extend_from_slice(&[0u8; 12]);
    input.extend_from_slice(&address_bytes(spender));
    input.extend_from_slice(&amount.to_be_bytes::<32>());
    input
}

fn set_approval_for_all_calldata(operator: &str, approved: bool) -> Vec<u8> {
    let mut input = SET_APPROVAL_FOR_ALL_SELECTOR.to_vec();
    input.extend_from_slice(&[0u8; 12]);
    input.extend_from_slice(&address_bytes(operator));
    input.extend_from_slice(&U256::from(approved as u8).to_be_bytes::<32>());
    input
}

fn exit_pool_calldata(pool_id: [u8; 32]) -> Vec<u8> {
    let mut input = hex::decode("8bdb3913").unwrap();
    input.extend_from_slice(&pool_id);
    input
}

fn permit2_approve_calldata(token: Address, spender: Address, amount: U256) -> Vec<u8> {
    let mut input = PERMIT2_APPROVE_SELECTOR.to_vec();
    input.extend_from_slice(&pad_address(token));
    input.extend_from_slice(&pad_address(spender));
    input.extend_from_slice(&amount.to_be_bytes::<32>());
    input.extend_from_slice(&U256::from(1234u64).to_be_bytes::<32>());
    input
}

fn pad_address(address: Address) -> [u8; 32] {
    let mut bytes = [0u8; 32];
    bytes[12..].copy_from_slice(address.as_slice());
    bytes
}

fn address_from_hex(address: &str) -> Address {
    Address::from_slice(&address_bytes(address))
}
