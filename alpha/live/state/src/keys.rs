//! Canonical live-state key builders.

use alloy_primitives::Address;

pub const LATEST_BLOCK_NUMBER_KEY: &str = "eth/live/latest/block_number";
pub const LATEST_BLOCK_HASH_KEY: &str = "eth/live/latest/block_hash";
pub const PROCESSED_BLOCK_STREAM_KEY: &str = "eth/live/blocks";
pub const RECENT_BLOCKS_KEY: &str = "eth/live/recent_blocks";
pub const LATEST_CHAIN_STATE_KEY: &str = "eth/live/latest/chain_state_block_number";
pub const TOKEN_INDEX_KEY: &str = "eth/live/token/snapshot/index";

const BLOCK_PREFIX: &str = "eth/live/block";
const TOKEN_SNAPSHOT_PREFIX: &str = "eth/live/token/snapshot";
const POSITION_PREFIX: &str = "eth/live/position";

pub const fn latest_block_number_key() -> &'static str {
    LATEST_BLOCK_NUMBER_KEY
}

pub const fn latest_block_hash_key() -> &'static str {
    LATEST_BLOCK_HASH_KEY
}

pub const fn processed_block_stream_key() -> &'static str {
    PROCESSED_BLOCK_STREAM_KEY
}

pub const fn recent_blocks_key() -> &'static str {
    RECENT_BLOCKS_KEY
}

pub const fn latest_chain_state_block_number_key() -> &'static str {
    LATEST_CHAIN_STATE_KEY
}

pub const fn token_index_key() -> &'static str {
    TOKEN_INDEX_KEY
}

pub fn block_meta_key(block_number: u64) -> String {
    format!("{BLOCK_PREFIX}/{block_number}/meta")
}

pub fn block_header_key(block_number: u64) -> String {
    format!("{BLOCK_PREFIX}/{block_number}/header")
}

pub fn processed_transactions_key(block_number: u64) -> String {
    processed_tx_map_key(block_number)
}

pub fn processed_tx_map_key(block_number: u64) -> String {
    format!("{BLOCK_PREFIX}/{block_number}/txs")
}

pub fn tx_index_key(block_number: u64) -> String {
    format!("{BLOCK_PREFIX}/{block_number}/tx_index")
}

pub fn block_addresses_key(block_number: u64) -> String {
    format!("{BLOCK_PREFIX}/{block_number}/addresses")
}

pub fn chain_state_snapshot_key(block_number: u64) -> String {
    format!("{BLOCK_PREFIX}/{block_number}/chain_state_snapshot")
}

pub fn token_key(token_address: impl AsRef<str>) -> String {
    token_snapshot_key(token_address)
}

pub fn token_snapshot_key(token_address: impl AsRef<str>) -> String {
    format!("{TOKEN_SNAPSHOT_PREFIX}/{}", token_address.as_ref())
}

pub fn token_snapshot_key_for_address(token_address: Address) -> String {
    token_snapshot_key(normalized_address_string(token_address))
}

pub fn position_key(portfolio_id: &str, token_address: &str) -> String {
    format!(
        "{POSITION_PREFIX}/{portfolio_id}/{}",
        token_address.to_ascii_lowercase()
    )
}

pub fn position_key_for_address(portfolio_id: &str, token_address: Address) -> String {
    position_key(portfolio_id, &normalized_address_string(token_address))
}

pub fn normalized_address_string(address: Address) -> String {
    address.to_string().to_ascii_lowercase()
}
