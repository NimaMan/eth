use alloy_primitives::{address, B256, U256};
use tx_processor::tx_processor::data_models::{
    ERC20ApprovalEvent, ERC20TransferEvent, TradingEnabledEvent, UniswapV2MintEvent,
    UniswapV2SwapEvent,
};
use tx_processor::ProcessedTransaction;

use crate::erc20::ERC20TokenMetadata;
use crate::manager::replay_context::triggers::tx_is_token_control_replay_candidate;
use crate::manager::TokenRegistry;

use super::BlockReplayContext;

fn registry() -> TokenRegistry {
    let token_address = "0xdf22ce0de1c93bae44efc948770f65352631c403";
    let owner = "0x90ed7090d469f83e474aaef297be834a113ede67";
    let pool = "0x90920d41573c981afb151213de19c9e8c9601b98";
    let weth = "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2";

    let mut registry = TokenRegistry::new();
    registry.add_token(ERC20TokenMetadata::new(
        token_address,
        "Mooner",
        "MOONER",
        18,
        "1000000000000000000000000000",
    ));
    let token = registry.token_mut(token_address).unwrap();
    token.handle_contract_creation(100, 1_700, "0xcreate", owner, 0);
    token.create_uniswap_v2_pool(
        pool,
        weth,
        crate::pools::BasePoolConfig {
            token_decimals: 18,
            denom_decimals: Some(18),
            token1_is_denom: Some(false),
            history_limit: 10,
            denom_threshold: 0.0,
            threshold_unit: None,
            test_buy_amount_eth: crate::pools::base::DEFAULT_TEST_BUY_ETH,
        },
        std::iter::empty::<&str>(),
    );
    registry
}

fn tx(hash_byte: u8, tx_index: u64, from: alloy_primitives::Address) -> ProcessedTransaction {
    let mut hash = [0u8; 32];
    hash[31] = hash_byte;
    ProcessedTransaction::new(
        B256::from(hash),
        100,
        1_700,
        tx_index,
        from,
        None,
        U256::ZERO,
        true,
        0,
        2,
        Vec::new(),
    )
}

#[test]
fn token_control_prior_is_keyed_by_token_not_sender() {
    let registry = registry();
    let mut context = BlockReplayContext::default();

    let token = address!("df22ce0de1c93bae44efc948770f65352631c403");
    let owner = address!("90ed7090d469f83e474aaef297be834a113ede67");
    let buyer = address!("54bace10fb12ed190515328908383467477c69f5");
    let pool = address!("90920d41573c981afb151213de19c9e8c9601b98");

    let mut enable_tx = tx(1, 30, owner);
    enable_tx.to_address = Some(token);
    enable_tx.actions.push("Trading Enable".to_string());
    enable_tx.trading_enabled_events.push(TradingEnabledEvent {
        token_address: token,
        block_number: 100,
        log_index: 0,
    });

    let mut buy_tx = tx(2, 31, buyer);
    buy_tx.uniswap_v2_swaps.push(UniswapV2SwapEvent {
        pair_address: pool,
        sender: buyer,
        to: buyer,
        amount0_in: U256::from(1),
        amount1_in: U256::ZERO,
        amount0_out: U256::ZERO,
        amount1_out: U256::from(10),
        log_index: 1,
    });
    buy_tx.erc20_transfers.push(ERC20TransferEvent {
        token_address: token,
        from_address: pool,
        to_address: buyer,
        amount: U256::from(10),
        log_index: 2,
    });

    assert!(context
        .prior_txs_for_transaction(&registry, &buy_tx)
        .is_empty());

    context.observe_transaction(&registry, &enable_tx);
    let prior_txs = context.prior_txs_for_transaction(&registry, &buy_tx);

    assert_eq!(prior_txs.len(), 1);
    assert_eq!(prior_txs[0].hash, enable_tx.hash);
    assert_ne!(prior_txs[0].from_address, buy_tx.from_address);
}

#[test]
fn same_block_erc20_approval_is_token_prior_for_router_replay() {
    let registry = registry();
    let mut context = BlockReplayContext::default();

    let token = address!("df22ce0de1c93bae44efc948770f65352631c403");
    let owner = address!("90ed7090d469f83e474aaef297be834a113ede67");
    let pool = address!("90920d41573c981afb151213de19c9e8c9601b98");
    let router = address!("7a250d5630b4cf539739df2c5dacb4c659f2488d");

    let mut approve_tx = tx(11, 8, owner);
    approve_tx.to_address = Some(token);
    approve_tx.erc20_approval_events.push(ERC20ApprovalEvent {
        token_address: token,
        owner,
        spender: router,
        amount: U256::from(50_000),
        log_index: 0,
    });

    let mut router_tx = tx(12, 9, owner);
    router_tx.to_address = Some(router);
    router_tx.erc20_transfers.push(ERC20TransferEvent {
        token_address: token,
        from_address: owner,
        to_address: pool,
        amount: U256::from(50_000),
        log_index: 1,
    });
    router_tx.uniswap_v2_mints.push(UniswapV2MintEvent {
        pair_address: pool,
        sender: router,
        amount0: U256::from(50_000),
        amount1: U256::from(1),
        log_index: 2,
    });

    assert!(context
        .prior_txs_for_transaction(&registry, &router_tx)
        .is_empty());

    context.observe_transaction(&registry, &approve_tx);
    let prior_txs = context.prior_txs_for_transaction(&registry, &router_tx);

    assert_eq!(prior_txs.len(), 1);
    assert_eq!(prior_txs[0].hash, approve_tx.hash);
}

#[test]
fn token_control_candidate_detects_trading_enable_event() {
    let registry = registry();
    let token_state = registry
        .token("0xdf22ce0de1c93bae44efc948770f65352631c403")
        .unwrap();
    let token = address!("df22ce0de1c93bae44efc948770f65352631c403");
    let owner = address!("90ed7090d469f83e474aaef297be834a113ede67");

    let mut enable_tx = tx(3, 30, owner);
    enable_tx.to_address = Some(token);
    enable_tx.trading_enabled_events.push(TradingEnabledEvent {
        token_address: token,
        block_number: 100,
        log_index: 0,
    });

    assert!(tx_is_token_control_replay_candidate(
        token_state,
        &enable_tx
    ));
}

#[test]
fn pool_setup_prior_is_recorded_but_swaps_are_not_pool_priors() {
    let registry = registry();
    let mut context = BlockReplayContext::default();

    let pool = address!("90920d41573c981afb151213de19c9e8c9601b98");
    let sender = address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    let buyer = address!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");

    let mut mint_tx = tx(4, 10, sender);
    mint_tx.uniswap_v2_mints.push(UniswapV2MintEvent {
        pair_address: pool,
        sender,
        amount0: U256::from(100),
        amount1: U256::from(1),
        log_index: 0,
    });

    let mut swap_tx = tx(5, 11, buyer);
    swap_tx.uniswap_v2_swaps.push(UniswapV2SwapEvent {
        pair_address: pool,
        sender: buyer,
        to: buyer,
        amount0_in: U256::from(1),
        amount1_in: U256::ZERO,
        amount0_out: U256::ZERO,
        amount1_out: U256::from(10),
        log_index: 1,
    });

    context.observe_transaction(&registry, &mint_tx);
    let prior_txs = context.prior_txs_for_transaction(&registry, &swap_tx);
    assert_eq!(prior_txs.len(), 1);
    assert_eq!(prior_txs[0].hash, mint_tx.hash);

    let mut lp_transfer_tx = tx(10, 12, sender);
    lp_transfer_tx.erc20_transfers.push(ERC20TransferEvent {
        token_address: pool,
        from_address: sender,
        to_address: address!("000000000000000000000000000000000000dead"),
        amount: U256::from(10),
        log_index: 2,
    });
    let lp_transfer_priors = context.prior_txs_for_transaction(&registry, &lp_transfer_tx);
    assert_eq!(lp_transfer_priors.len(), 1);
    assert_eq!(lp_transfer_priors[0].hash, mint_tx.hash);

    context.observe_transaction(&registry, &swap_tx);
    let prior_txs_after_swap = context.prior_txs_for_transaction(&registry, &swap_tx);
    assert_eq!(prior_txs_after_swap.len(), 1);
    assert_eq!(prior_txs_after_swap[0].hash, mint_tx.hash);
}

#[test]
fn direct_owner_setup_calls_to_token_are_same_block_priors() {
    let registry = registry();
    let mut context = BlockReplayContext::default();

    let token = address!("df22ce0de1c93bae44efc948770f65352631c403");
    let owner = address!("90ed7090d469f83e474aaef297be834a113ede67");
    let pool = address!("90920d41573c981afb151213de19c9e8c9601b98");
    let buyer = address!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");

    let mut fund_token_tx = tx(6, 22, owner);
    fund_token_tx.to_address = Some(token);
    fund_token_tx.value = U256::from(1_000_000_000_000_000_000u128);

    let mut transfer_to_token_tx = tx(7, 23, owner);
    transfer_to_token_tx.to_address = Some(token);
    transfer_to_token_tx
        .erc20_transfers
        .push(ERC20TransferEvent {
            token_address: token,
            from_address: owner,
            to_address: token,
            amount: U256::from(100),
            log_index: 0,
        });

    let mut add_liquidity_tx = tx(8, 24, owner);
    add_liquidity_tx.to_address = Some(token);
    add_liquidity_tx.uniswap_v2_mints.push(UniswapV2MintEvent {
        pair_address: pool,
        sender: owner,
        amount0: U256::from(100),
        amount1: U256::from(1),
        log_index: 1,
    });

    let mut swap_tx = tx(9, 25, buyer);
    swap_tx.uniswap_v2_swaps.push(UniswapV2SwapEvent {
        pair_address: pool,
        sender: buyer,
        to: buyer,
        amount0_in: U256::from(1),
        amount1_in: U256::ZERO,
        amount0_out: U256::ZERO,
        amount1_out: U256::from(10),
        log_index: 2,
    });
    swap_tx.erc20_transfers.push(ERC20TransferEvent {
        token_address: token,
        from_address: pool,
        to_address: buyer,
        amount: U256::from(10),
        log_index: 3,
    });

    context.observe_transaction(&registry, &fund_token_tx);
    context.observe_transaction(&registry, &transfer_to_token_tx);
    let add_liquidity_priors = context.prior_txs_for_transaction(&registry, &add_liquidity_tx);
    assert_eq!(
        add_liquidity_priors
            .iter()
            .map(|tx| tx.hash)
            .collect::<Vec<_>>(),
        vec![fund_token_tx.hash, transfer_to_token_tx.hash]
    );

    context.observe_transaction(&registry, &add_liquidity_tx);
    let swap_priors = context.prior_txs_for_transaction(&registry, &swap_tx);
    assert_eq!(
        swap_priors.iter().map(|tx| tx.hash).collect::<Vec<_>>(),
        vec![
            fund_token_tx.hash,
            transfer_to_token_tx.hash,
            add_liquidity_tx.hash
        ]
    );
}
