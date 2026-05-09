use alloy_primitives::{address, b256, Address, U256};
use tx_processor::ProcessedTransaction;

use super::*;

fn initialize_event() -> ProcessedV4InitializeEvent {
    ProcessedV4InitializeEvent {
        pool_manager_address: address!("000000000004444c5dc75cb358380d2e3de08a90"),
        event_id: b256!("1111111111111111111111111111111111111111111111111111111111111111"),
        currency0: address!("0000000000000000000000000000000000000000"),
        currency1: address!("0000000000000000000000000000000000000001"),
        fee: 3000,
        tick_spacing: 60,
        hooks: address!("0000000000000000000000000000000000000000"),
        sqrt_price_x96: U256::from(1u128) << 96,
        tick: 0,
        log_index: 1,
    }
}

fn tx() -> (ProcessedTransaction, UniswapV2TxContext) {
    (
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
            0,
            Vec::new(),
        ),
        UniswapV2TxContext::new(100, 1_700, "0xTX"),
    )
}

#[test]
fn initialize_preserves_native_eth_currency_and_uses_weth_display_denom() {
    let event = initialize_event();
    let pool = UniswapV4Pool::from_initialize_event(
        &event,
        "0x0000000000000000000000000000000000000001",
        display_denom_for_v4_currency(event.currency0),
        BasePoolConfig {
            token_decimals: 18,
            denom_decimals: Some(18),
            token1_is_denom: Some(false),
            ..BasePoolConfig::new(18)
        },
    );

    assert_eq!(pool.pool_key.currency0, V4_NATIVE_ETH_ADDRESS);
    assert_eq!(pool.base.identity.denom_address, WETH_ADDRESS);
}

#[test]
fn unknown_pool_id_is_not_a_match() {
    let event = initialize_event();
    let pool = UniswapV4Pool::from_initialize_event(
        &event,
        "0x0000000000000000000000000000000000000001",
        display_denom_for_v4_currency(event.currency0),
        BasePoolConfig {
            token_decimals: 18,
            denom_decimals: Some(18),
            token1_is_denom: Some(false),
            ..BasePoolConfig::new(18)
        },
    );

    assert!(!pool.matches_event(
        event.pool_manager_address,
        b256!("2222222222222222222222222222222222222222222222222222222222222222")
    ));
}

#[test]
fn modifies_active_liquidity_only_when_current_tick_inside_range() {
    let event = initialize_event();
    let mut pool = UniswapV4Pool::from_initialize_event(
        &event,
        "0x0000000000000000000000000000000000000001",
        display_denom_for_v4_currency(event.currency0),
        BasePoolConfig {
            token_decimals: 18,
            denom_decimals: Some(18),
            token1_is_denom: Some(false),
            ..BasePoolConfig::new(18)
        },
    );
    let (mut processed, ctx) = tx();
    processed
        .uniswap_v4_modifies
        .push(ProcessedV4ModifyLiquidityEvent {
            pool_manager_address: event.pool_manager_address,
            event_id: event.event_id,
            sender: address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
            tick_lower: -60,
            tick_upper: 60,
            liquidity_delta: 1_000_000_000_000_000_000i128,
            salt: B256::ZERO,
            log_index: 2,
        });

    pool.update_from_processed_transaction(&processed, &ctx)
        .unwrap();

    assert_eq!(pool.active_liquidity, 1_000_000_000_000_000_000u128);
    assert_eq!(pool.base.price(), 1.0);
}

#[test]
fn modify_liquidity_tracks_position_nft_owner_as_lp_holder() {
    let event = initialize_event();
    let mut pool = UniswapV4Pool::from_initialize_event(
        &event,
        "0x0000000000000000000000000000000000000001",
        display_denom_for_v4_currency(event.currency0),
        BasePoolConfig {
            token_decimals: 18,
            denom_decimals: Some(18),
            token1_is_denom: Some(false),
            ..BasePoolConfig::new(18)
        },
    );
    let (mut processed, ctx) = tx();
    let position_manager = address!("bd216513d74c8cf14cf4747e6aaa6420ff64ee9e");
    let owner = address!("0d82a9f1ae5b693c9b00c8e874057fb78824cfd3");
    let salt = b256!("000000000000000000000000000000000000000000000000000000000003ea79");
    processed.erc721_transfers.push(ERC721TransferEvent {
        token_address: position_manager,
        from_address: Address::ZERO,
        to_address: owner,
        token_id: position_token_id(salt),
        log_index: 3,
    });
    processed
        .uniswap_v4_modifies
        .push(ProcessedV4ModifyLiquidityEvent {
            pool_manager_address: event.pool_manager_address,
            event_id: event.event_id,
            sender: position_manager,
            tick_lower: -60,
            tick_upper: 60,
            liquidity_delta: 100i128,
            salt,
            log_index: 2,
        });

    pool.update_from_processed_transaction(&processed, &ctx)
        .unwrap();

    let holders = pool.lp_holders();
    assert_eq!(holders.len(), 1);
    assert_eq!(
        holders[0].address,
        "0x0d82a9f1ae5b693c9b00c8e874057fb78824cfd3"
    );
    assert_eq!(holders[0].balance, 100.0);
    assert_eq!(pool.lp_total_supply(), 100.0);
    assert_eq!(
        pool.modify_liquidity_events[0]["liquidity_provider"],
        "0x0d82a9f1ae5b693c9b00c8e874057fb78824cfd3"
    );
    assert_eq!(
        pool.position_manager_address.as_deref(),
        Some("0xbd216513d74c8cf14cf4747e6aaa6420ff64ee9e")
    );
}

#[test]
fn repeated_modify_uses_existing_position_owner_when_no_transfer_event() {
    let event = initialize_event();
    let mut pool = UniswapV4Pool::from_initialize_event(
        &event,
        "0x0000000000000000000000000000000000000001",
        display_denom_for_v4_currency(event.currency0),
        BasePoolConfig {
            token_decimals: 18,
            denom_decimals: Some(18),
            token1_is_denom: Some(false),
            ..BasePoolConfig::new(18)
        },
    );
    let (mut processed, ctx) = tx();
    let position_manager = address!("bd216513d74c8cf14cf4747e6aaa6420ff64ee9e");
    let owner = address!("0d82a9f1ae5b693c9b00c8e874057fb78824cfd3");
    let salt = b256!("000000000000000000000000000000000000000000000000000000000003ea79");
    processed.erc721_transfers.push(ERC721TransferEvent {
        token_address: position_manager,
        from_address: Address::ZERO,
        to_address: owner,
        token_id: position_token_id(salt),
        log_index: 3,
    });
    processed
        .uniswap_v4_modifies
        .push(ProcessedV4ModifyLiquidityEvent {
            pool_manager_address: event.pool_manager_address,
            event_id: event.event_id,
            sender: position_manager,
            tick_lower: -60,
            tick_upper: 60,
            liquidity_delta: 100i128,
            salt,
            log_index: 2,
        });
    pool.update_from_processed_transaction(&processed, &ctx)
        .unwrap();

    let (mut processed, ctx) = tx();
    processed.from_address = address!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
    processed
        .uniswap_v4_modifies
        .push(ProcessedV4ModifyLiquidityEvent {
            pool_manager_address: event.pool_manager_address,
            event_id: event.event_id,
            sender: position_manager,
            tick_lower: -60,
            tick_upper: 60,
            liquidity_delta: -25i128,
            salt,
            log_index: 2,
        });
    pool.update_from_processed_transaction(&processed, &ctx)
        .unwrap();

    let holders = pool.lp_holders();
    assert_eq!(holders.len(), 1);
    assert_eq!(
        holders[0].address,
        "0x0d82a9f1ae5b693c9b00c8e874057fb78824cfd3"
    );
    assert_eq!(holders[0].balance, 75.0);
}

#[test]
fn position_transfer_updates_existing_lp_holder() {
    let event = initialize_event();
    let mut pool = UniswapV4Pool::from_initialize_event(
        &event,
        "0x0000000000000000000000000000000000000001",
        display_denom_for_v4_currency(event.currency0),
        BasePoolConfig {
            token_decimals: 18,
            denom_decimals: Some(18),
            token1_is_denom: Some(false),
            ..BasePoolConfig::new(18)
        },
    );
    let (mut processed, ctx) = tx();
    let position_manager = address!("bd216513d74c8cf14cf4747e6aaa6420ff64ee9e");
    let owner = address!("0d82a9f1ae5b693c9b00c8e874057fb78824cfd3");
    let next_owner = address!("7ca2d5fa2c6b3e01294a74e353c89141837ad784");
    let salt = b256!("000000000000000000000000000000000000000000000000000000000003ea79");
    processed.erc721_transfers.push(ERC721TransferEvent {
        token_address: position_manager,
        from_address: Address::ZERO,
        to_address: owner,
        token_id: position_token_id(salt),
        log_index: 3,
    });
    processed
        .uniswap_v4_modifies
        .push(ProcessedV4ModifyLiquidityEvent {
            pool_manager_address: event.pool_manager_address,
            event_id: event.event_id,
            sender: position_manager,
            tick_lower: -60,
            tick_upper: 60,
            liquidity_delta: 100i128,
            salt,
            log_index: 2,
        });
    pool.update_from_processed_transaction(&processed, &ctx)
        .unwrap();

    let (mut transfer_tx, ctx) = tx();
    transfer_tx.erc721_transfers.push(ERC721TransferEvent {
        token_address: position_manager,
        from_address: owner,
        to_address: next_owner,
        token_id: position_token_id(salt),
        log_index: 1,
    });

    assert!(pool.touches_position_transfer(&transfer_tx));
    pool.update_from_processed_transaction(&transfer_tx, &ctx)
        .unwrap();

    let holders = pool.lp_holders();
    assert_eq!(holders.len(), 1);
    assert_eq!(
        holders[0].address,
        "0x7ca2d5fa2c6b3e01294a74e353c89141837ad784"
    );
    assert_eq!(holders[0].balance, 100.0);
    assert_eq!(
        pool.liquidity_position_events
            .last()
            .and_then(|event| event["event"].as_str()),
        Some("position_transfer")
    );
}

#[test]
fn position_approval_marks_v4_lp_liquidity_as_approved() {
    let event = initialize_event();
    let mut pool = UniswapV4Pool::from_initialize_event(
        &event,
        "0x0000000000000000000000000000000000000001",
        display_denom_for_v4_currency(event.currency0),
        BasePoolConfig {
            token_decimals: 18,
            denom_decimals: Some(18),
            token1_is_denom: Some(false),
            ..BasePoolConfig::new(18)
        },
    );
    let position_manager = address!("bd216513d74c8cf14cf4747e6aaa6420ff64ee9e");
    let owner = address!("0d82a9f1ae5b693c9b00c8e874057fb78824cfd3");
    let router = address!("3fc91a3afd70395cd496c647d5a6cc9d4b2b7fad");
    let salt = b256!("000000000000000000000000000000000000000000000000000000000003ea79");
    pool.set_known_routers([address_string(&router)]);

    let (mut processed, ctx) = tx();
    processed.erc721_transfers.push(ERC721TransferEvent {
        token_address: position_manager,
        from_address: Address::ZERO,
        to_address: owner,
        token_id: position_token_id(salt),
        log_index: 3,
    });
    processed
        .uniswap_v4_modifies
        .push(ProcessedV4ModifyLiquidityEvent {
            pool_manager_address: event.pool_manager_address,
            event_id: event.event_id,
            sender: position_manager,
            tick_lower: -60,
            tick_upper: 60,
            liquidity_delta: 100i128,
            salt,
            log_index: 2,
        });
    pool.update_from_processed_transaction(&processed, &ctx)
        .unwrap();

    let (mut approval_tx, ctx) = tx();
    approval_tx
        .erc721_approval_events
        .push(ERC721ApprovalEvent {
            token_address: position_manager,
            owner,
            approved_address: router,
            token_id: position_token_id(salt),
            log_index: 1,
        });

    assert!(pool.touches_position_approval(&approval_tx));
    pool.update_from_processed_transaction(&approval_tx, &ctx)
        .unwrap();

    let router = address_string(&router);
    let holders = pool.lp_holders();
    assert_eq!(holders[0].approvals[&router].amount, 100.0);
    assert!(holders[0].approvals[&router].is_router);
    assert_eq!(pool.total_approved_to_routers(), 100.0);
    assert_eq!(pool.lp_approved_percentage(), 100.0);
    assert_eq!(pool.last_lp_approval_block(), Some(100));
    assert_eq!(pool.holders_with_approvals(), vec![address_string(&owner)]);
}

#[test]
fn approval_for_all_marks_all_owner_v4_lp_liquidity_as_approved() {
    let event = initialize_event();
    let mut pool = UniswapV4Pool::from_initialize_event(
        &event,
        "0x0000000000000000000000000000000000000001",
        display_denom_for_v4_currency(event.currency0),
        BasePoolConfig {
            token_decimals: 18,
            denom_decimals: Some(18),
            token1_is_denom: Some(false),
            ..BasePoolConfig::new(18)
        },
    );
    let position_manager = address!("bd216513d74c8cf14cf4747e6aaa6420ff64ee9e");
    let owner = address!("0d82a9f1ae5b693c9b00c8e874057fb78824cfd3");
    let operator = address!("000000000022d473030f116ddee9f6b43ac78ba3");
    let salt = b256!("000000000000000000000000000000000000000000000000000000000003ea79");
    pool.set_known_routers([address_string(&operator)]);

    let (mut processed, ctx) = tx();
    processed.erc721_transfers.push(ERC721TransferEvent {
        token_address: position_manager,
        from_address: Address::ZERO,
        to_address: owner,
        token_id: position_token_id(salt),
        log_index: 3,
    });
    processed
        .uniswap_v4_modifies
        .push(ProcessedV4ModifyLiquidityEvent {
            pool_manager_address: event.pool_manager_address,
            event_id: event.event_id,
            sender: position_manager,
            tick_lower: -60,
            tick_upper: 60,
            liquidity_delta: 250i128,
            salt,
            log_index: 2,
        });
    pool.update_from_processed_transaction(&processed, &ctx)
        .unwrap();

    let (mut approval_tx, ctx) = tx();
    approval_tx
        .approval_for_all_events
        .push(ApprovalForAllEvent {
            token_address: position_manager,
            owner,
            operator,
            approved: true,
            log_index: 1,
        });

    assert!(pool.touches_position_approval(&approval_tx));
    pool.update_from_processed_transaction(&approval_tx, &ctx)
        .unwrap();

    let operator = address_string(&operator);
    let holders = pool.lp_holders();
    assert_eq!(holders[0].approvals[&operator].amount, 250.0);
    assert!(holders[0].approvals[&operator].is_router);
    assert_eq!(pool.lp_approval_events.len(), 1);

    let (mut revoke_tx, ctx) = tx();
    revoke_tx.approval_for_all_events.push(ApprovalForAllEvent {
        token_address: position_manager,
        owner,
        operator: operator.parse().unwrap(),
        approved: false,
        log_index: 1,
    });
    pool.update_from_processed_transaction(&revoke_tx, &ctx)
        .unwrap();
    assert!(pool.lp_holders()[0].approvals.is_empty());
}
