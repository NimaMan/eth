use alloy_primitives::{address, b256, Address, U256};
use rust_decimal::Decimal;
use std::str::FromStr;
use tx_processor::tx_processor::data_models::trace_models::InternalTransaction;
use tx_processor::tx_processor::data_models::tx_models::ETHTransfer;
use tx_processor::tx_processor::data_models::TransactionFees;
use tx_processor::tx_processor::data_models::{DepositEvent, ERC20TransferEvent};
use tx_processor::ProcessedTransaction;

use crate::pnl::common::address_string;

use super::*;

const TOKEN: Address = address!("1111111111111111111111111111111111111111");
const DENOM: Address = address!("2222222222222222222222222222222222222222");
const WETH: Address = address!("c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2");
const POOL: Address = address!("3333333333333333333333333333333333333333");
const FEE_PAYER: Address = address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
const DENOM_PAYER: Address = address!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
const TOKEN_RECEIVER: Address = address!("cccccccccccccccccccccccccccccccccccccccc");
const TWO_ETH: u128 = 2_000_000_000_000_000_000;

#[test]
fn records_split_actor_pool_trade_and_conservation() {
    let mut tracker = TokenPnlTracker::default();
    let mut tx = tx();
    tx.fees = TransactionFees::new(U256::from(10), 21_000, 21_000);
    tx.bribe_amount = U256::from(7);
    tx.erc20_transfers.push(ERC20TransferEvent {
        token_address: DENOM,
        from_address: DENOM_PAYER,
        to_address: POOL,
        amount: U256::from(1_000),
        log_index: 1,
    });
    tx.erc20_transfers.push(ERC20TransferEvent {
        token_address: TOKEN,
        from_address: POOL,
        to_address: TOKEN_RECEIVER,
        amount: U256::from(50_000),
        log_index: 2,
    });

    tracker.record_v2_pool_transaction(
        address_string(&POOL),
        address_string(&TOKEN),
        address_string(&DENOM),
        9,
        18,
        100,
        &tx,
    );

    let pool = tracker.pool(address_string(&POOL)).expect("pool pnl");
    let conservation = pool.conservation_summary();
    assert!(conservation.token_is_conserved);
    assert!(conservation.denom_is_conserved);
    assert_eq!(conservation.pool_token_delta_raw, "-50000");
    assert_eq!(conservation.pool_denom_delta_raw, "1000");

    // FEE_PAYER only paid gas with no token/denom movement in this tx —
    // net zero in both → excluded by the pass-through filter.
    assert!(pool.position(address_string(&FEE_PAYER)).is_none());

    let denom_payer = pool
        .position(address_string(&DENOM_PAYER))
        .expect("denom payer");
    assert_eq!(denom_payer.denom_out_raw, U256::from(1_000));
    assert_eq!(denom_payer.token_in_raw, U256::ZERO);

    let token_receiver = pool
        .position(address_string(&TOKEN_RECEIVER))
        .expect("token receiver");
    assert_eq!(token_receiver.token_in_raw, U256::from(50_000));
    assert_eq!(token_receiver.denom_out_raw, U256::ZERO);
}

#[test]
fn keeps_transaction_scoped_router_movements_and_marks_pool_direct_entries() {
    let mut tracker = TokenPnlTracker::default();
    let router = address!("dddddddddddddddddddddddddddddddddddddddd");
    let mut tx = tx();
    tx.erc20_transfers.push(ERC20TransferEvent {
        token_address: DENOM,
        from_address: DENOM_PAYER,
        to_address: router,
        amount: U256::from(1_000),
        log_index: 1,
    });
    tx.erc20_transfers.push(ERC20TransferEvent {
        token_address: DENOM,
        from_address: router,
        to_address: POOL,
        amount: U256::from(1_000),
        log_index: 2,
    });
    tx.erc20_transfers.push(ERC20TransferEvent {
        token_address: TOKEN,
        from_address: POOL,
        to_address: router,
        amount: U256::from(5_000),
        log_index: 3,
    });
    tx.erc20_transfers.push(ERC20TransferEvent {
        token_address: TOKEN,
        from_address: router,
        to_address: TOKEN_RECEIVER,
        amount: U256::from(5_000),
        log_index: 4,
    });

    tracker.record_v2_pool_transaction(
        address_string(&POOL),
        address_string(&TOKEN),
        address_string(&DENOM),
        9,
        18,
        100,
        &tx,
    );

    let pool = tracker.pool(address_string(&POOL)).expect("pool pnl");
    // Router nets zero in both token and denom → excluded by pass-through filter.
    assert!(pool.position(address_string(&router)).is_none());
    assert_eq!(
        pool.position(address_string(&DENOM_PAYER))
            .expect("payer")
            .denom_out_raw,
        U256::from(1_000)
    );
    assert_eq!(
        pool.position(address_string(&TOKEN_RECEIVER))
            .expect("receiver")
            .token_in_raw,
        U256::from(5_000)
    );
    // Router entries (including pool_direct ones) are not committed.
    assert_eq!(
        pool.recent_entries
            .iter()
            .filter(|entry| entry.pool_direct)
            .count(),
        0
    );
    assert!(pool.conservation_summary().token_is_conserved);
    assert!(pool.conservation_summary().denom_is_conserved);
}

#[test]
fn treats_native_eth_transfers_as_denom_for_weth_pools() {
    let mut tracker = TokenPnlTracker::default();
    let router = address!("dddddddddddddddddddddddddddddddddddddddd");
    let mut tx = tx();
    tx.erc20_transfers.push(ERC20TransferEvent {
        token_address: WETH,
        from_address: POOL,
        to_address: router,
        amount: U256::from(1_000),
        log_index: 1,
    });
    tx.internal_transactions.push(InternalTransaction {
        from_address: router,
        to_address: Some(TOKEN_RECEIVER),
        value: U256::from(1_000),
        gas: 0,
        gas_used: 0,
        trace_type: "call".to_string(),
        call_type: Some("call".to_string()),
        depth: 1,
        error: None,
    });

    tracker.record_v2_pool_transaction(
        address_string(&POOL),
        address_string(&TOKEN),
        address_string(&WETH),
        9,
        18,
        100,
        &tx,
    );

    let pool = tracker.pool(address_string(&POOL)).expect("pool pnl");
    // Router receives WETH ERC20 and forwards native ETH — net denom zero → excluded.
    assert!(pool.position(address_string(&router)).is_none());
    assert_eq!(
        pool.position(address_string(&TOKEN_RECEIVER))
            .expect("receiver")
            .denom_in_raw,
        U256::from(1_000)
    );
    assert!(pool.conservation_summary().denom_is_conserved);
}

#[test]
fn banana_gun_shaped_weth_trade_excludes_router_from_aggregate_pnl() {
    let mut tracker = TokenPnlTracker::default();
    let router = address!("dddddddddddddddddddddddddddddddddddddddd");
    let implementation = address!("eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee");
    let mut tx = tx();
    tx.eth_transfers.push(ETHTransfer {
        from_address: FEE_PAYER,
        to_address: router,
        amount: U256::from(TWO_ETH),
    });
    tx.internal_transactions.push(InternalTransaction {
        from_address: FEE_PAYER,
        to_address: Some(router),
        value: U256::from(TWO_ETH),
        gas: 0,
        gas_used: 0,
        trace_type: "call".to_string(),
        call_type: Some("CALL".to_string()),
        depth: 0,
        error: None,
    });
    tx.internal_transactions.push(InternalTransaction {
        from_address: router,
        to_address: Some(implementation),
        value: U256::from(TWO_ETH),
        gas: 0,
        gas_used: 0,
        trace_type: "call".to_string(),
        call_type: Some("DELEGATECALL".to_string()),
        depth: 1,
        error: None,
    });
    tx.internal_transactions.push(InternalTransaction {
        from_address: router,
        to_address: Some(WETH),
        value: U256::from(TWO_ETH),
        gas: 0,
        gas_used: 0,
        trace_type: "call".to_string(),
        call_type: Some("CALL".to_string()),
        depth: 1,
        error: None,
    });
    tx.deposit_events.push(DepositEvent {
        id: None,
        token_address: None,
        withdrawal_address: None,
        amount: Some(U256::from(TWO_ETH)),
        unlock_time: None,
        pair_address: Some(WETH),
        sender: Some(router),
        log_index: Some(1),
    });
    tx.erc20_transfers.push(ERC20TransferEvent {
        token_address: WETH,
        from_address: router,
        to_address: POOL,
        amount: U256::from(TWO_ETH),
        log_index: 2,
    });
    tx.erc20_transfers.push(ERC20TransferEvent {
        token_address: TOKEN,
        from_address: POOL,
        to_address: FEE_PAYER,
        amount: U256::from(50_000),
        log_index: 3,
    });

    tracker.record_v2_pool_transaction(
        address_string(&POOL),
        address_string(&TOKEN),
        address_string(&WETH),
        9,
        18,
        100,
        &tx,
    );

    let pool = tracker.pool(address_string(&POOL)).expect("pool pnl");
    assert!(pool.position(address_string(&router)).is_none());
    assert!(pool.position(address_string(&implementation)).is_none());
    assert!(pool.position(address_string(&WETH)).is_none());

    let user = pool
        .position(address_string(&FEE_PAYER))
        .expect("user position");
    assert_eq!(user.denom_out_raw, U256::from(TWO_ETH));
    assert_eq!(user.token_in_raw, U256::from(50_000));

    let conservation = pool.conservation_summary();
    assert!(conservation.token_is_conserved);
    assert!(conservation.denom_is_conserved);
    assert_eq!(conservation.pool_denom_delta_raw, TWO_ETH.to_string());
    assert_eq!(conservation.pool_token_delta_raw, "-50000");
}

#[test]
fn weth_pool_pnl_proxy_subtracts_native_fees_and_priority_fees() {
    let mut tracker = TokenPnlTracker::default();
    let mut tx = tx();
    tx.fees = TransactionFees::new(U256::from(10), 21_000, 21_000);
    tx.bribe_amount = U256::from(7);
    tx.erc20_transfers.push(ERC20TransferEvent {
        token_address: WETH,
        from_address: FEE_PAYER,
        to_address: POOL,
        amount: U256::from(1_000_000_000_000_000_000_u64),
        log_index: 1,
    });

    tracker.record_v2_pool_transaction(
        address_string(&POOL),
        address_string(&TOKEN),
        address_string(&WETH),
        9,
        18,
        100,
        &tx,
    );

    let pool = tracker.pool(address_string(&POOL)).expect("pool pnl");
    let summary = pool
        .address_summaries(Some(0.0))
        .into_iter()
        .find(|summary| summary.address == address_string(&FEE_PAYER))
        .expect("fee payer summary");

    assert_eq!(summary.denom_cashflow, Decimal::from(-1));
    assert_eq!(
        summary.native_fee,
        Decimal::from_str("0.00000000000021").unwrap()
    );
    assert_eq!(
        summary.native_priority_fee,
        Decimal::from_str("0.000000000000000007").unwrap()
    );
    assert_eq!(
        summary.pnl_proxy_denom.expect("pnl proxy"),
        Decimal::from_str("-1.000000000000210007").unwrap()
    );
}

fn tx() -> ProcessedTransaction {
    ProcessedTransaction::new(
        b256!("0101010101010101010101010101010101010101010101010101010101010101"),
        100,
        1_700,
        1,
        FEE_PAYER,
        None,
        U256::ZERO,
        true,
        0,
        2,
        Vec::new(),
    )
}
