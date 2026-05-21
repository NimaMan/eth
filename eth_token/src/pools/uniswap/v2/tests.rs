use super::*;

fn pool() -> UniswapV2Pool {
    UniswapV2Pool::new(
        "0xPOOL",
        "0xTOKEN",
        "0xDENOM",
        BasePoolConfig {
            token_decimals: 18,
            denom_decimals: Some(18),
            token1_is_denom: Some(true),
            history_limit: 10,
            denom_threshold: 0.05,
            threshold_unit: Some("ETH".to_string()),
            test_buy_amount_eth: 0.01,
        },
        ["0xROUTER"],
    )
}

#[test]
fn sync_updates_reserves_in_pool_orientation() {
    let mut pool = pool();
    let tx = UniswapV2TxContext::new(100, 1_700, "0xTX");

    pool.process_sync(
        &UniswapV2SyncEvent {
            pair_address: "0xpool".to_string(),
            reserve0: "100000000000000000000".to_string(),
            reserve1: "2000000000000000000".to_string(),
        },
        &tx,
    )
    .unwrap();

    assert_eq!(pool.base.token_reserve(), 100.0);
    assert_eq!(pool.base.denom_reserve(), 2.0);
    assert_eq!(pool.base.price(), 0.02);
    assert_eq!(pool.latest_sync().unwrap()["tx_hash"], "0xTX");
}

#[test]
fn swap_tracks_volumes_direction_without_setting_simulator_status() {
    let mut pool = pool();
    let tx = UniswapV2TxContext::new(101, 1_701, "0xSWAP");

    pool.process_swap(
        &UniswapV2SwapEvent {
            pair_address: "0xPOOL".to_string(),
            sender: Some("0xSENDER".to_string()),
            to: Some("0xTO".to_string()),
            amount0_in: "0".to_string(),
            amount1_in: "1000000000000000000".to_string(),
            amount0_out: "50000000000000000000".to_string(),
            amount1_out: "0".to_string(),
        },
        &tx,
    )
    .unwrap();

    assert!(!pool.base.state.can_buy);
    assert!(!pool.base.trading_enabled());
    assert_eq!(pool.base.state.total_swaps, 1);
    assert_eq!(pool.base.state.denom_volume_in, 1_000_000_000_000_000_000.0);
    assert_eq!(pool.recent_swaps(1)[0]["is_buy"], true);
}

#[test]
fn sell_swap_does_not_mark_pool_as_buyable() {
    let mut pool = pool();
    let tx = UniswapV2TxContext::new(101, 1_701, "0xSWAP");

    pool.process_swap(
        &UniswapV2SwapEvent {
            pair_address: "0xPOOL".to_string(),
            sender: Some("0xSENDER".to_string()),
            to: Some("0xTO".to_string()),
            amount0_in: "50000000000000000000".to_string(),
            amount1_in: "0".to_string(),
            amount0_out: "0".to_string(),
            amount1_out: "1000000000000000000".to_string(),
        },
        &tx,
    )
    .unwrap();

    assert!(!pool.base.state.can_buy);
    assert!(!pool.base.trading_enabled());
    assert_eq!(pool.recent_swaps(1)[0]["is_sell"], true);
}

#[test]
fn mint_and_burn_append_events_and_counters() {
    let mut pool = pool();
    let tx = UniswapV2TxContext::new(102, 1_702, "0xLIQ");

    pool.process_mint(
        &UniswapV2MintEvent {
            pair_address: "0xPOOL".to_string(),
            to: Some("0xTO".to_string()),
            amount: Some("10".to_string()),
        },
        &tx,
    )
    .unwrap();
    pool.process_burn(
        &UniswapV2BurnEvent {
            pair_address: "0xPOOL".to_string(),
            sender: Some("0xFROM".to_string()),
            amount0: "3".to_string(),
            amount1: "4".to_string(),
        },
        &tx,
    )
    .unwrap();

    assert_eq!(pool.base.state.total_mints, 1);
    assert_eq!(pool.base.state.total_burns, 1);
    assert_eq!(pool.base.burn_events[0]["amount"], 3.0);
}

#[test]
fn lp_tracker_tracks_supply_balances_and_router_approvals() {
    let mut pool = pool();
    pool.lp_tracker
        .record_transfer(ZERO_ADDRESS, "0xHOLDER", 100.0, 1, "0xMINT", None);
    pool.lp_tracker
        .record_approval("0xHOLDER", "0xROUTER", 80.0, 2, "0xAPPROVE", Some(55));

    assert_eq!(pool.lp_tracker.total_supply, 100.0);
    assert_eq!(pool.lp_tracker.share("0xholder"), 100.0);
    assert_eq!(pool.lp_tracker.total_approved_to_routers(), 80.0);
    assert_eq!(pool.lp_tracker.approved_percentage(), 80.0);
    assert_eq!(pool.lp_tracker.last_approval_block(), Some(2));
    assert_eq!(
        pool.lp_tracker.last_approval_event().unwrap()["tx_hash"],
        "0xAPPROVE"
    );
    assert_eq!(pool.lp_tracker.holder_snapshots()[0].address, "0xholder");
    assert_eq!(
        pool.lp_tracker.holders_with_approvals(),
        vec!["0xholder".to_string()]
    );
}

#[test]
fn pool_data_for_publishing_matches_python_keys() {
    let mut pool = pool();
    pool.base
        .set_simulated_buy_status(true, 200, "0xBUY", 1_800);
    pool.base.update_reserves(100.0, 2.0, 200, 1_800, "0xSYNC");

    let data = pool.pool_data_for_publishing();

    assert_eq!(data["pool_address"], "0xpool");
    assert_eq!(data["trading_enabled"], true);
    assert_eq!(data["trading_enabled_block"], 200);
    assert_eq!(data["trading_enabled_tx"], "0xBUY");
}

#[test]
fn update_from_events_processes_v2_event_batch() {
    let mut pool = pool();
    let tx = UniswapV2TxContext::new(300, 1_900, "0xBATCH");
    let events = UniswapV2TransactionEvents {
        syncs: vec![UniswapV2SyncEvent {
            pair_address: "0xPOOL".to_string(),
            reserve0: "100000000000000000000".to_string(),
            reserve1: "2000000000000000000".to_string(),
        }],
        swaps: vec![UniswapV2SwapEvent {
            pair_address: "0xPOOL".to_string(),
            sender: None,
            to: None,
            amount0_in: "0".to_string(),
            amount1_in: "1".to_string(),
            amount0_out: "2".to_string(),
            amount1_out: "0".to_string(),
        }],
        mints: vec![UniswapV2MintEvent {
            pair_address: "0xPOOL".to_string(),
            to: None,
            amount: None,
        }],
        burns: vec![UniswapV2BurnEvent {
            pair_address: "0xPOOL".to_string(),
            sender: None,
            amount0: "0".to_string(),
            amount1: "0".to_string(),
        }],
    };

    pool.update_from_events(&events, &tx).unwrap();

    assert_eq!(pool.base.sync_events.len(), 1);
    assert_eq!(pool.base.swap_events.len(), 1);
    assert_eq!(pool.base.mint_events[0]["amount"], 0.0);
    assert_eq!(pool.base.burn_events.len(), 1);
}

#[test]
fn lp_event_helpers_scale_raw_amounts_like_python() {
    let mut pool = pool();

    pool.process_lp_transfer(&LPTransferEvent {
        from_address: ZERO_ADDRESS.to_string(),
        to_address: "0xHOLDER".to_string(),
        amount: "100000000000000000000".to_string(),
        block_number: Some(1),
        tx_hash: Some("0xMINT".to_string()),
        log_index: Some(7),
    })
    .unwrap();
    pool.process_lp_approval(&LPApprovalEvent {
        owner: "0xHOLDER".to_string(),
        spender: "0xROUTER".to_string(),
        amount: "25000000000000000000".to_string(),
        block_number: Some(2),
        tx_hash: Some("0xAPPROVE".to_string()),
        block_timestamp: Some(55),
    })
    .unwrap();

    assert_eq!(pool.lp_share("0xholder"), 100.0);
    assert_eq!(pool.total_approved_to_routers(), 25.0);
    assert_eq!(pool.lp_approved_percentage(), 25.0);
    assert_eq!(pool.last_lp_approval_block(), Some(2));
    assert_eq!(pool.holders_with_approvals(), vec!["0xholder".to_string()]);
}
