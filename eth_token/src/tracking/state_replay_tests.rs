use super::tests::{metadata, tx};
use super::*;
use alloy_primitives::{address, U256};
use tx_processor::tx_processor::data_models::{UniswapV2PairCreatedEvent, UniswapV2SyncEvent};

#[test]
fn tracked_token_index_indexes_pool_to_token_mapping() {
    let mut registry = TokenRegistry::new();
    let update_router = ProcessedTokenUpdateRouter::new(100);
    registry.add_token(metadata());
    let mut tx = tx();
    tx.uniswap_v2_pair_created_events
        .push(UniswapV2PairCreatedEvent {
            pair_address: address!("3333333333333333333333333333333333333333"),
            token0: address!("1111111111111111111111111111111111111111"),
            token1: address!("2222222222222222222222222222222222222222"),
            factory_address: address!("5c69bee701ef814a2b6a3edd4b1652cb9cc5aa6f"),
            log_index: 1,
        });
    let token_index = TrackedTokenIndex::from_registry(&registry, 100);
    update_router
        .update_registry_from_processed_transaction(&mut registry, &token_index, &tx)
        .unwrap();

    let index = TrackedTokenIndex::from_registry(&registry, 100);

    assert_eq!(
        index.token_for_pool("0x3333333333333333333333333333333333333333"),
        Some("0x1111111111111111111111111111111111111111")
    );
    assert_eq!(
        index.resolve_token_address("0x3333333333333333333333333333333333333333"),
        Some("0x1111111111111111111111111111111111111111")
    );
}

#[test]
fn token_state_builder_replays_processed_transactions_in_order() {
    let builder = TokenStateBuilder::new(metadata(), 100);
    let mut creation_tx = tx();
    creation_tx.tx_index = 0;
    creation_tx.contract_address = Some(address!("1111111111111111111111111111111111111111"));

    let mut pair_tx = tx();
    pair_tx.tx_index = 1;
    pair_tx
        .uniswap_v2_pair_created_events
        .push(UniswapV2PairCreatedEvent {
            pair_address: address!("3333333333333333333333333333333333333333"),
            token0: address!("1111111111111111111111111111111111111111"),
            token1: address!("2222222222222222222222222222222222222222"),
            factory_address: address!("5c69bee701ef814a2b6a3edd4b1652cb9cc5aa6f"),
            log_index: 1,
        });

    let mut sync_tx = tx();
    sync_tx.tx_index = 2;
    sync_tx.uniswap_v2_syncs.push(UniswapV2SyncEvent {
        pair_address: address!("3333333333333333333333333333333333333333"),
        reserve0: U256::from(100_000_000_000_000_000_000_u128),
        reserve1: U256::from(2_000_000_000_000_000_000_u128),
        log_index: 2,
    });

    let token = builder
        .build_from_processed_transactions([sync_tx, creation_tx, pair_tx])
        .unwrap();

    assert_eq!(token.creation_block, Some(100));
    let pool = token
        .uniswap_v2_pool("0x3333333333333333333333333333333333333333")
        .unwrap();
    assert_eq!(pool.base.token_reserve(), 100.0);
    assert_eq!(pool.base.denom_reserve(), 2.0);
}
