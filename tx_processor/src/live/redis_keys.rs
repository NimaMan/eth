//! Redis key helpers for the standalone live block publisher compatibility path.

const LATEST_BLOCK_NUMBER_KEY: &str = "eth/live/latest/block_number";
const LATEST_BLOCK_HASH_KEY: &str = "eth/live/latest/block_hash";
const LATEST_CHAIN_STATE_BLOCK_NUMBER_KEY: &str = "eth/live/latest/chain_state_block_number";
const PROCESSED_BLOCK_STREAM_KEY: &str = "eth/live/blocks";
const RECENT_BLOCKS_KEY: &str = "eth/live/recent_blocks";
const BLOCK_PREFIX: &str = "eth/live/block/";

pub const fn latest_block_number_key() -> &'static str {
    LATEST_BLOCK_NUMBER_KEY
}

pub const fn latest_block_hash_key() -> &'static str {
    LATEST_BLOCK_HASH_KEY
}

pub const fn latest_chain_state_block_number_key() -> &'static str {
    LATEST_CHAIN_STATE_BLOCK_NUMBER_KEY
}

pub const fn processed_block_stream_key() -> &'static str {
    PROCESSED_BLOCK_STREAM_KEY
}

pub const fn recent_blocks_key() -> &'static str {
    RECENT_BLOCKS_KEY
}

pub fn block_meta_key(block_number: u64) -> String {
    format!("{BLOCK_PREFIX}{block_number}/meta")
}

pub fn block_header_key(block_number: u64) -> String {
    format!("{BLOCK_PREFIX}{block_number}/header")
}

pub fn processed_transactions_key(block_number: u64) -> String {
    format!("{BLOCK_PREFIX}{block_number}/txs")
}

pub fn tx_index_key(block_number: u64) -> String {
    format!("{BLOCK_PREFIX}{block_number}/tx_index")
}

pub fn block_addresses_key(block_number: u64) -> String {
    format!("{BLOCK_PREFIX}{block_number}/addresses")
}

pub fn chain_state_snapshot_key(block_number: u64) -> String {
    format!("{BLOCK_PREFIX}{block_number}/chain_state_snapshot")
}
