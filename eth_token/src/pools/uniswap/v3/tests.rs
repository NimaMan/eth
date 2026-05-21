use alloy_primitives::{address, b256, Address, U256};
use tx_processor::ProcessedTransaction;

use super::*;

fn pool() -> UniswapV3Pool {
    UniswapV3Pool::new(
        "0x0000000000000000000000000000000000000003",
        "0x0000000000000000000000000000000000000001",
        "0x0000000000000000000000000000000000000002",
        "0x0000000000000000000000000000000000000001",
        "0x0000000000000000000000000000000000000002",
        3000,
        60,
        BasePoolConfig {
            token_decimals: 18,
            denom_decimals: Some(18),
            token1_is_denom: Some(true),
            ..BasePoolConfig::new(18)
        },
    )
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
fn swap_sets_post_swap_price_tick_and_liquidity() {
    let mut pool = pool();
    let (mut processed, ctx) = tx();
    let sqrt = U256::from(1u128) << 96;
    processed
        .uniswap_v3_initializations
        .push(ProcessedV3InitializeEvent {
            pool_address: address!("0000000000000000000000000000000000000003"),
            sqrt_price_x96: sqrt,
            tick: 0,
            log_index: 1,
        });
    processed.uniswap_v3_mints.push(ProcessedV3MintEvent {
        pool_address: address!("0000000000000000000000000000000000000003"),
        sender: address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
        owner: address!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"),
        tick_lower: -60,
        tick_upper: 60,
        amount: U256::from(1_000_000_000_000_000_000u128),
        amount0: U256::ZERO,
        amount1: U256::ZERO,
        log_index: 2,
    });
    processed.uniswap_v3_swaps.push(ProcessedV3SwapEvent {
        pool_address: address!("0000000000000000000000000000000000000003"),
        sender: address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
        recipient: address!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"),
        amount0: -100,
        amount1: 100,
        sqrt_price_x96: sqrt,
        liquidity: 1_000_000_000_000_000_000u128,
        tick: 0,
        log_index: 3,
    });

    pool.update_from_processed_transaction(&processed, &ctx)
        .unwrap();

    assert_eq!(pool.current_tick, Some(0));
    assert_eq!(pool.active_liquidity, 1_000_000_000_000_000_000u128);
    assert_eq!(pool.base.price(), 1.0);
    assert!(!pool.base.state.can_buy);
    assert_eq!(pool.base.state.total_swaps, 1);
    assert_eq!(pool.base.swap_events[0]["is_buy"], true);
    assert_eq!(pool.base.swap_events[0]["is_sell"], false);
}

#[test]
fn mint_tracks_position_nft_owner_as_lp_holder() {
    let mut pool = pool();
    let (mut processed, ctx) = tx();
    let position_manager = address!("c36442b4a4522e871399cd717abdd847ab11fe88");
    let owner = address!("0d82a9f1ae5b693c9b00c8e874057fb78824cfd3");
    let token_id = U256::from(42);
    processed.erc721_transfers.push(ERC721TransferEvent {
        token_address: position_manager,
        from_address: Address::ZERO,
        to_address: owner,
        token_id,
        log_index: 1,
    });
    processed.uniswap_v3_mints.push(ProcessedV3MintEvent {
        pool_address: address!("0000000000000000000000000000000000000003"),
        sender: position_manager,
        owner: position_manager,
        tick_lower: -60,
        tick_upper: 60,
        amount: U256::from(100),
        amount0: U256::from(1),
        amount1: U256::from(2),
        log_index: 2,
    });
    processed
        .uniswap_v3_increases
        .push(ProcessedV3IncreaseLiquidityEvent {
            token_id,
            liquidity: U256::from(100),
            amount0: U256::from(1),
            amount1: U256::from(2),
            pool_address: position_manager,
            log_index: 3,
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
    assert_eq!(
        pool.position_manager_address.as_deref(),
        Some("0xc36442b4a4522e871399cd717abdd847ab11fe88")
    );
    assert_eq!(
        pool.liquidity_position_events[0]["liquidity_provider"],
        "0x0d82a9f1ae5b693c9b00c8e874057fb78824cfd3"
    );
}

#[test]
fn burn_decreases_existing_position_owner_liquidity() {
    let mut pool = pool();
    let (mut processed, ctx) = tx();
    let position_manager = address!("c36442b4a4522e871399cd717abdd847ab11fe88");
    let owner = address!("0d82a9f1ae5b693c9b00c8e874057fb78824cfd3");
    let token_id = U256::from(42);
    processed.erc721_transfers.push(ERC721TransferEvent {
        token_address: position_manager,
        from_address: Address::ZERO,
        to_address: owner,
        token_id,
        log_index: 1,
    });
    processed.uniswap_v3_mints.push(ProcessedV3MintEvent {
        pool_address: address!("0000000000000000000000000000000000000003"),
        sender: position_manager,
        owner: position_manager,
        tick_lower: -60,
        tick_upper: 60,
        amount: U256::from(100),
        amount0: U256::from(1),
        amount1: U256::from(2),
        log_index: 2,
    });
    processed
        .uniswap_v3_increases
        .push(ProcessedV3IncreaseLiquidityEvent {
            token_id,
            liquidity: U256::from(100),
            amount0: U256::from(1),
            amount1: U256::from(2),
            pool_address: position_manager,
            log_index: 3,
        });
    pool.update_from_processed_transaction(&processed, &ctx)
        .unwrap();

    let (mut processed, ctx) = tx();
    processed.uniswap_v3_burns.push(ProcessedV3BurnEvent {
        pool_address: address!("0000000000000000000000000000000000000003"),
        owner: position_manager,
        tick_lower: -60,
        tick_upper: 60,
        amount: U256::from(25),
        amount0: U256::from(1),
        amount1: U256::from(2),
        log_index: 2,
    });
    processed
        .uniswap_v3_decreases
        .push(ProcessedV3DecreaseLiquidityEvent {
            token_id,
            liquidity: U256::from(25),
            amount0: U256::from(1),
            amount1: U256::from(2),
            pool_address: position_manager,
            log_index: 1,
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
    let mut pool = pool();
    let (mut processed, ctx) = tx();
    let position_manager = address!("c36442b4a4522e871399cd717abdd847ab11fe88");
    let owner = address!("0d82a9f1ae5b693c9b00c8e874057fb78824cfd3");
    let next_owner = address!("7ca2d5fa2c6b3e01294a74e353c89141837ad784");
    let token_id = U256::from(42);
    processed.erc721_transfers.push(ERC721TransferEvent {
        token_address: position_manager,
        from_address: Address::ZERO,
        to_address: owner,
        token_id,
        log_index: 1,
    });
    processed.uniswap_v3_mints.push(ProcessedV3MintEvent {
        pool_address: address!("0000000000000000000000000000000000000003"),
        sender: position_manager,
        owner: position_manager,
        tick_lower: -60,
        tick_upper: 60,
        amount: U256::from(100),
        amount0: U256::from(1),
        amount1: U256::from(2),
        log_index: 2,
    });
    processed
        .uniswap_v3_increases
        .push(ProcessedV3IncreaseLiquidityEvent {
            token_id,
            liquidity: U256::from(100),
            amount0: U256::from(1),
            amount1: U256::from(2),
            pool_address: position_manager,
            log_index: 3,
        });
    pool.update_from_processed_transaction(&processed, &ctx)
        .unwrap();

    let (mut transfer_tx, ctx) = tx();
    transfer_tx.erc721_transfers.push(ERC721TransferEvent {
        token_address: position_manager,
        from_address: owner,
        to_address: next_owner,
        token_id,
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
}
