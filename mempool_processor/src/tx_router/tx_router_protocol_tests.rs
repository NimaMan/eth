use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use alloy_primitives::U256;
use reth_chain_query::common_addresses::POOL_FACTORIES;
use reth_chain_query::to_checksum_address;
use serde_json::json;

use super::{CreatorFunctionType, SimulationPriority, TransactionCategory, TransactionRouter};
use crate::mempool_fetcher::MempoolTransaction;
use crate::token_tracking::types::PoolLifecycle;
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

fn exit_pool_calldata(pool_id: [u8; 32]) -> Vec<u8> {
    let mut input = hex::decode("8bdb3913").unwrap();
    input.extend_from_slice(&pool_id);
    input
}
