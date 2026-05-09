use super::*;
use crate::pools::uniswap::v2::{
    UniswapV2SwapEvent, UniswapV2SyncEvent, UniswapV2TransactionEvents, UNISWAP_V2_PROTOCOL,
};
use crate::pools::{
    BalancerPoolToken, CurvePoolToken, BALANCER_V2_PROTOCOL, CURVE_V1_PROTOCOL,
    SUSHISWAP_V2_PROTOCOL,
};
use alloy_primitives::Address;

fn token() -> ERC20Token {
    ERC20Token::new(ERC20TokenMetadata::new(
        "0x0000000000000000000000000000000000000001",
        "Token",
        "TKN",
        18,
        "1000000000000000000000",
    ))
}

#[test]
fn token_live_mode_defaults_false_and_can_be_enabled() {
    let metadata = ERC20TokenMetadata::new(
        "0x0000000000000000000000000000000000000001",
        "Token",
        "TKN",
        18,
        "1000000000000000000000",
    );

    assert!(!ERC20Token::new(metadata.clone()).is_live_mode);
    assert!(ERC20Token::with_live_mode(metadata, true).is_live_mode);
}

#[test]
fn token_owns_v2_pools_and_reports_summary() {
    let mut token = token();
    token.create_uniswap_v2_pool(
        "0x0000000000000000000000000000000000000002",
        "0x0000000000000000000000000000000000000003",
        BasePoolConfig {
            denom_decimals: Some(18),
            token1_is_denom: Some(true),
            ..BasePoolConfig::new(18)
        },
        std::iter::empty::<&str>(),
    );

    assert!(token.has_pool());
    assert_eq!(
        token.token_life_cycle_status,
        Some(TokenLifecycleState::PairCreation)
    );
    assert_eq!(token.pool_addresses().len(), 1);
    assert_eq!(
        token.get_token_summary().protocols,
        vec![UNISWAP_V2_PROTOCOL]
    );
}

#[test]
fn token_reports_non_uniswap_pool_protocols() {
    let mut token = token();
    token.create_sushiswap_v2_pool(
        "0x0000000000000000000000000000000000000002",
        "0x0000000000000000000000000000000000000003",
        BasePoolConfig {
            denom_decimals: Some(18),
            token1_is_denom: Some(true),
            ..BasePoolConfig::new(18)
        },
        std::iter::empty::<&str>(),
    );
    token.create_curve_pool(
        "0x0000000000000000000000000000000000000004",
        "0x0000000000000000000000000000000000000003",
        BasePoolConfig {
            denom_decimals: Some(18),
            token1_is_denom: Some(true),
            ..BasePoolConfig::new(18)
        },
        Some("curve-test".to_string()),
        Some("0x0000000000000000000000000000000000000005".to_string()),
        0,
        1,
        vec![CurvePoolToken {
            symbol: Some("TKN".to_string()),
            address: token.contract_address.clone(),
            decimals: token.decimals,
            index: 0,
        }],
    );
    token.create_balancer_pool(
        "0x0000000000000000000000000000000000000006",
        "0x0000000000000000000000000000000000000003",
        BasePoolConfig {
            denom_decimals: Some(18),
            token1_is_denom: Some(true),
            ..BasePoolConfig::new(18)
        },
        "0x1111111111111111111111111111111111111111111111111111111111111111",
        "0x0000000000000000000000000000000000000007",
        Some(30),
        vec![BalancerPoolToken {
            symbol: Some("TKN".to_string()),
            address: token.contract_address.clone(),
            decimals: token.decimals,
            index: 0,
            weight: Some("80".to_string()),
        }],
    );

    let summary = token.get_token_summary();

    assert_eq!(summary.pool_count, 3);
    assert!(summary
        .protocols
        .contains(&SUSHISWAP_V2_PROTOCOL.to_string()));
    assert!(summary.protocols.contains(&CURVE_V1_PROTOCOL.to_string()));
    assert!(summary
        .protocols
        .contains(&BALANCER_V2_PROTOCOL.to_string()));
}

#[test]
fn token_trading_enabled_is_pool_derived() {
    let mut token = token();
    token.status_manager.trading_enabled = true;
    token.status_manager.trading_enabled_block = Some(100);
    token.refresh_lifecycle_status();

    assert!(!token.trading_enabled());
    assert_eq!(token.trading_enabled_block(), None);
    assert_eq!(token.trading_enabled_tx(), None);
    assert_ne!(
        token.token_life_cycle_status,
        Some(TokenLifecycleState::TradingEnabled)
    );

    {
        let pool = token.create_uniswap_v2_pool(
            "0x0000000000000000000000000000000000000002",
            "0x0000000000000000000000000000000000000003",
            BasePoolConfig {
                denom_decimals: Some(18),
                token1_is_denom: Some(true),
                ..BasePoolConfig::new(18)
            },
            std::iter::empty::<&str>(),
        );
        pool.base
            .set_simulated_buy_status(true, 200, "0xBUY", 1_700);
    }
    token.refresh_lifecycle_status();

    assert!(token.trading_enabled());
    assert_eq!(token.trading_enabled_block(), Some(200));
    assert_eq!(token.trading_enabled_tx(), Some("0xBUY".to_string()));
    assert_eq!(
        token.token_life_cycle_status,
        Some(TokenLifecycleState::TradingEnabled)
    );
}

#[test]
fn updates_v2_pool_from_event_batch() {
    let mut token = token();
    let pool_address = "0x0000000000000000000000000000000000000002";
    token.create_uniswap_v2_pool(
        pool_address,
        "0x0000000000000000000000000000000000000003",
        BasePoolConfig {
            denom_decimals: Some(18),
            token1_is_denom: Some(true),
            ..BasePoolConfig::new(18)
        },
        std::iter::empty::<&str>(),
    );

    let tx = UniswapV2TxContext {
        block_number: 10,
        block_timestamp: 1_700,
        tx_hash: "0xTX".to_string(),
        from_address: Some("0xMAKER".to_string()),
    };
    let events = UniswapV2TransactionEvents {
        syncs: vec![UniswapV2SyncEvent {
            pair_address: pool_address.to_string(),
            reserve0: "100000000000000000000".to_string(),
            reserve1: "2000000000000000000".to_string(),
        }],
        swaps: vec![UniswapV2SwapEvent {
            pair_address: pool_address.to_string(),
            sender: Some("0x0000000000000000000000000000000000000004".to_string()),
            to: Some("0x0000000000000000000000000000000000000005".to_string()),
            amount0_in: "0".to_string(),
            amount1_in: "1000000000000000000".to_string(),
            amount0_out: "50000000000000000000".to_string(),
            amount1_out: "0".to_string(),
        }],
        ..Default::default()
    };

    token
        .update_uniswap_v2_pool_events(pool_address, &events, &tx)
        .unwrap();

    let pool = token.uniswap_v2_pool(pool_address).unwrap();
    assert_eq!(pool.base.token_reserve(), 100.0);
    assert_eq!(pool.base.denom_reserve(), 2.0);
    assert_eq!(token.latest_block_number, Some(10));

    let block = token.activity.blocks.get(&10).unwrap();
    assert_eq!(block.num_tx, 1);
    assert_eq!(
        block.buy_volume_by_denom["0x0000000000000000000000000000000000000003"],
        1.0
    );
    let summary = token.get_token_summary();
    assert_eq!(summary.activity_block_count, 1);
    assert_eq!(summary.total_transactions, 1);
    assert_eq!(
        summary.total_buy_volume_by_denom["0x0000000000000000000000000000000000000003"],
        1.0
    );
}

#[test]
fn token_activity_tracks_eth_bribe_without_double_counting_tx() {
    let mut token = token();
    let mut tx = ProcessedTransaction::new(
        alloy_primitives::B256::repeat_byte(0x42),
        12,
        1_712,
        0,
        Address::repeat_byte(0x11),
        None,
        alloy_primitives::U256::ZERO,
        true,
        0,
        0,
        Vec::new(),
    );
    tx.bribe_amount = alloy_primitives::U256::from(25_000_000_000_000_000_u128);

    token
        .update_token_state_from_processed_transaction(&tx)
        .unwrap();
    token
        .update_token_state_from_processed_transaction(&tx)
        .unwrap();

    let block = token.activity.blocks.get(&12).unwrap();
    assert_eq!(block.num_tx, 1);
    assert_eq!(block.total_bribe_eth, 0.025);
    assert_eq!(token.activity.total_bribe_eth(), 0.025);
}
