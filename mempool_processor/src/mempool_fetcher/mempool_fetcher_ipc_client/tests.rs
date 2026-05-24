use super::*;
use alloy_primitives::{address, Address};
use serde_json::json;
use std::collections::HashMap;

use crate::token_tracking::{
    types::PoolLifecycle, CacheConfig, Pool, PoolType, Token, TokenTrackingCache, TokenUpdate,
    TokenWithPools,
};

#[tokio::test]
async fn preserves_nonzero_erc20_approval_without_signal_filter() {
    let tx = tx(
        Some(address!("1111111111111111111111111111111111111111")),
        erc20_approve_calldata(
            address!("7a250d5630B4cF539739dF2C5dAcb4c659F2488D"),
            U256::from(1000u64),
        ),
    );

    assert!(matches!(
        critical_lane_candidate(&tx, None).await,
        Some(PreserveCandidate::LiquidityApproval)
    ));
}

#[tokio::test]
async fn does_not_preserve_zero_erc20_approval_when_ingress_is_full() {
    let tx = tx(
        Some(address!("1111111111111111111111111111111111111111")),
        erc20_approve_calldata(
            address!("7a250d5630B4cF539739dF2C5dAcb4c659F2488D"),
            U256::ZERO,
        ),
    );

    assert!(critical_lane_candidate(&tx, None).await.is_none());
}

#[tokio::test]
async fn preserves_position_approval_without_signal_filter() {
    let tx = tx(
        Some(address!("C36442b4a4522E871399CD717aBDD847Ab11FE88")),
        set_approval_for_all_calldata(address!("7a250d5630B4cF539739dF2C5dAcb4c659F2488D"), true),
    );

    assert!(matches!(
        critical_lane_candidate(&tx, None).await,
        Some(PreserveCandidate::PositionApproval)
    ));
}

#[tokio::test]
async fn preserves_liquidity_removal_without_signal_filter() {
    let token = address!("1111111111111111111111111111111111111111");
    let tx = tx(
        Some(address!("7a250d5630B4cF539739dF2C5dAcb4c659F2488D")),
        remove_liquidity_eth_calldata(token),
    );

    assert!(matches!(
        critical_lane_candidate(&tx, None).await,
        Some(PreserveCandidate::LiquidityRemoval)
    ));
}

#[tokio::test]
async fn filters_unrelated_regular_transfer_from_signal_queue() {
    let cache = Arc::new(test_cache().await);
    let mut tx = tx(
        Some(address!("1111111111111111111111111111111111111111")),
        erc20_transfer_calldata(address!("2222222222222222222222222222222222222222")),
    );
    tx.from = address!("dddddddddddddddddddddddddddddddddddddddd").to_vec();

    assert!(!should_enqueue_for_signal_path(&tx, Some(&cache)).await);
    assert!(critical_lane_candidate(&tx, Some(&cache)).await.is_none());
}

#[tokio::test]
async fn filters_plain_transfer_to_tracked_token_from_unrelated_sender() {
    let cache = Arc::new(test_cache().await);
    let mut tx = tx(
        Some(address!("9999999999999999999999999999999999999999")),
        erc20_transfer_calldata(address!("2222222222222222222222222222222222222222")),
    );
    tx.from = address!("dddddddddddddddddddddddddddddddddddddddd").to_vec();

    assert!(!should_enqueue_for_signal_path(&tx, Some(&cache)).await);
}

#[tokio::test]
async fn keeps_tracked_creator_tx_on_signal_queue() {
    let cache = Arc::new(test_cache().await);
    let mut tx = tx(
        Some(address!("1111111111111111111111111111111111111111")),
        Vec::new(),
    );
    tx.from = address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa").to_vec();

    assert!(should_enqueue_for_signal_path(&tx, Some(&cache)).await);
}

#[tokio::test]
async fn routes_tracked_lp_approval_to_critical_queue() {
    let cache = Arc::new(test_cache().await);
    let tx = tx(
        Some(address!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")),
        erc20_approve_calldata(
            address!("7a250d5630B4cF539739dF2C5dAcb4c659F2488D"),
            U256::from(1000u64),
        ),
    );

    assert!(matches!(
        critical_lane_candidate(&tx, Some(&cache)).await,
        Some(PreserveCandidate::LiquidityApproval)
    ));
}

#[tokio::test]
async fn does_not_route_untracked_lp_approval_to_signal_queue() {
    let cache = Arc::new(test_cache().await);
    let mut tx = tx(
        Some(address!("1111111111111111111111111111111111111111")),
        erc20_approve_calldata(
            address!("7a250d5630B4cF539739dF2C5dAcb4c659F2488D"),
            U256::from(1000u64),
        ),
    );
    tx.from = address!("dddddddddddddddddddddddddddddddddddddddd").to_vec();

    assert!(critical_lane_candidate(&tx, Some(&cache)).await.is_none());
    assert!(!should_enqueue_for_signal_path(&tx, Some(&cache)).await);
}

#[tokio::test]
async fn drains_critical_queue_before_normal_queue() {
    let client = MempoolFetcherIPCClient::new(Some("/tmp/not-used.sock")).unwrap();
    let mut normal = tx(
        Some(address!("1111111111111111111111111111111111111111")),
        Vec::new(),
    );
    normal.hash = "normal".to_string();
    let mut critical = tx(
        Some(address!("7a250d5630B4cF539739dF2C5dAcb4c659F2488D")),
        remove_liquidity_eth_calldata(address!("2222222222222222222222222222222222222222")),
    );
    critical.hash = "critical".to_string();

    client.tx_sender.try_send(normal).unwrap();
    client.queue_size.fetch_add(1, Ordering::Relaxed);
    client.critical_tx_sender.try_send(critical).unwrap();
    client.critical_queue_size.fetch_add(1, Ordering::Relaxed);

    let txs = client.get_transactions(2).await.unwrap();
    assert_eq!(txs.len(), 2);
    assert_eq!(txs[0].hash, "critical");
    assert_eq!(txs[1].hash, "normal");

    let stats = client.get_stats().await;
    assert_eq!(stats.queue_size, 0);
    assert_eq!(stats.critical_queue_size, 0);
}

#[tokio::test]
async fn split_instant_drains_keep_lanes_isolated() {
    let client = MempoolFetcherIPCClient::new(Some("/tmp/not-used.sock")).unwrap();
    let mut normal = tx(
        Some(address!("1111111111111111111111111111111111111111")),
        Vec::new(),
    );
    normal.hash = "normal".to_string();
    let mut critical = tx(
        Some(address!("7a250d5630B4cF539739dF2C5dAcb4c659F2488D")),
        remove_liquidity_eth_calldata(address!("2222222222222222222222222222222222222222")),
    );
    critical.hash = "critical".to_string();

    client.tx_sender.try_send(normal).unwrap();
    client.queue_size.fetch_add(1, Ordering::Relaxed);
    client.critical_tx_sender.try_send(critical).unwrap();
    client.critical_queue_size.fetch_add(1, Ordering::Relaxed);

    let critical_txs = client.get_critical_transactions_instant(10).await;
    assert_eq!(critical_txs.len(), 1);
    assert_eq!(critical_txs[0].hash, "critical");

    let stats = client.get_stats().await;
    assert_eq!(stats.queue_size, 1);
    assert_eq!(stats.critical_queue_size, 0);

    let normal_txs = client.get_normal_transactions_instant(10).await;
    assert_eq!(normal_txs.len(), 1);
    assert_eq!(normal_txs[0].hash, "normal");

    let stats = client.get_stats().await;
    assert_eq!(stats.queue_size, 0);
    assert_eq!(stats.critical_queue_size, 0);
}

fn tx(to: Option<Address>, input: Vec<u8>) -> MempoolTransaction {
    MempoolTransaction {
        hash: "0xtx".to_string(),
        data: json!({}),
        detection_ns: 0,
        detection_time: Instant::now(),
        latency_ns: 0,
        from: address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa").to_vec(),
        to: to.map(|address| address.to_vec()),
        input,
        value: U256::ZERO,
        gas_price: Some(U256::ZERO),
        functions: Vec::new(),
        function_category: None,
    }
}

fn erc20_approve_calldata(spender: Address, amount: U256) -> Vec<u8> {
    let mut input = vec![0x09, 0x5e, 0xa7, 0xb3];
    input.extend_from_slice(&pad_address(spender));
    input.extend_from_slice(&amount.to_be_bytes::<32>());
    input
}

fn erc20_transfer_calldata(to: Address) -> Vec<u8> {
    let mut input = vec![0xa9, 0x05, 0x9c, 0xbb];
    input.extend_from_slice(&pad_address(to));
    input.extend_from_slice(&U256::from(1u64).to_be_bytes::<32>());
    input
}

fn set_approval_for_all_calldata(operator: Address, approved: bool) -> Vec<u8> {
    let mut input = vec![0xa2, 0x2c, 0xb4, 0x65];
    input.extend_from_slice(&pad_address(operator));
    input.extend_from_slice(&U256::from(approved as u8).to_be_bytes::<32>());
    input
}

fn remove_liquidity_eth_calldata(token: Address) -> Vec<u8> {
    let mut input = vec![0x0c, 0x49, 0xcc, 0xbe];
    input.extend_from_slice(&pad_address(token));
    input
}

fn pad_address(address: Address) -> [u8; 32] {
    let mut padded = [0u8; 32];
    padded[12..].copy_from_slice(address.as_slice());
    padded
}

async fn test_cache() -> TokenTrackingCache {
    let cache = TokenTrackingCache::new(CacheConfig::default());
    let token_address = "0x9999999999999999999999999999999999999999".to_string();
    let creator_address = "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string();
    let pool_address = "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string();

    let token = Token {
        address: token_address.clone(),
        symbol: "TEST".to_string(),
        name: "Test Token".to_string(),
        decimals: 18,
        total_supply: Some("1000000".to_string()),
        creator_address,
        current_owner: "0xcccccccccccccccccccccccccccccccccccccccc".to_string(),
        tax_setter_addresses: Vec::new(),
        ownership_renounced: false,
        renouncement_block: None,
        buy_tax: Some(0.0),
        sell_tax: Some(0.0),
        last_tax_change_block: None,
        tax_history: Vec::new(),
        creation_block: 1,
        creation_tx: "0xcreate".to_string(),
        creation_timestamp: None,
        latest_activity_block: 1,
        is_scam: false,
        scam_label: None,
        total_liquidity: 0.0,
    };

    let pool = Pool {
        address: pool_address.clone(),
        token_address: token_address.clone(),
        pool_type: PoolType::UniswapV2,
        token_reserve: 1_000_000.0,
        eth_reserve: 10.0,
        denom_currency: "WETH".to_string(),
        denom_address: "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2".to_string(),
        trading_enabled: true,
        trading_enabled_block: Some(1),
        trading_enabled_tx: None,
        fee_tier: None,
        pool_id: None,
        lp_token_address: Some(pool_address.clone()),
        position_manager_address: None,
        lp_total_supply: Some(1.0),
        liquidity_positions: Vec::new(),
        last_updated_block: 1,
        last_updated_time: 1.0,
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
    pools.insert(pool_address, pool);
    let mut data = HashMap::new();
    data.insert(token_address, TokenWithPools { token, pools });

    cache
        .batch_update(TokenUpdate {
            message_type: "test".to_string(),
            token_count: 1,
            block_number: 1,
            timestamp: 1.0,
            data,
        })
        .await;
    cache
}
