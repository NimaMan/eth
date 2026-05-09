use eth_live_state::keys;

#[test]
fn canonical_keys_match_legacy_live_state_contract() {
    assert_eq!(
        keys::latest_block_number_key(),
        "eth/live/latest/block_number"
    );
    assert_eq!(keys::latest_block_hash_key(), "eth/live/latest/block_hash");
    assert_eq!(keys::processed_block_stream_key(), "eth/live/blocks");
    assert_eq!(keys::recent_blocks_key(), "eth/live/recent_blocks");
    assert_eq!(
        keys::latest_chain_state_block_number_key(),
        "eth/live/latest/chain_state_block_number"
    );

    assert_eq!(keys::block_meta_key(42), "eth/live/block/42/meta");
    assert_eq!(keys::block_header_key(42), "eth/live/block/42/header");
    assert_eq!(keys::processed_tx_map_key(42), "eth/live/block/42/txs");
    assert_eq!(
        keys::processed_transactions_key(42),
        "eth/live/block/42/txs"
    );
    assert_eq!(keys::tx_index_key(42), "eth/live/block/42/tx_index");
    assert_eq!(keys::block_addresses_key(42), "eth/live/block/42/addresses");
    assert_eq!(
        keys::chain_state_snapshot_key(42),
        "eth/live/block/42/chain_state_snapshot"
    );

    assert_eq!(keys::token_key("0xAbC"), "eth/live/token/snapshot/0xAbC");
    assert_eq!(
        keys::token_snapshot_key("0xAbC"),
        "eth/live/token/snapshot/0xAbC"
    );
    assert_eq!(keys::token_index_key(), "eth/live/token/snapshot/index");
    assert_eq!(
        keys::position_key("main", "0xAbC"),
        "eth/live/position/main/0xabc"
    );
}
