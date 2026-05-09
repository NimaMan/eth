//! Canonical Redis key helpers shared across simulators and processors.

const LATEST_BLOCK_NUMBER_KEY: &str = "eth/live/latest/block_number";
const LATEST_BLOCK_HASH_KEY: &str = "eth/live/latest/block_hash";
const PROCESSED_BLOCK_STREAM_KEY: &str = "eth/live/blocks";
const RECENT_BLOCKS_KEY: &str = "eth/live/recent_blocks";
const LATEST_CHAIN_STATE_KEY: &str = "eth/live/latest/chain_state_block_number";
const BLOCK_PREFIX: &str = "eth/live/block/";
const TOKEN_SNAPSHOT_PREFIX: &str = "eth/live/token/snapshot/";
const POSITION_PREFIX: &str = "eth/live/position/";

/// Redis key for the latest known block number.
pub const fn latest_block_number_key() -> &'static str {
    LATEST_BLOCK_NUMBER_KEY
}

/// Redis key for the latest known block hash.
pub const fn latest_block_hash_key() -> &'static str {
    LATEST_BLOCK_HASH_KEY
}

/// Redis stream carrying processed-block notifications.
pub const fn processed_block_stream_key() -> &'static str {
    PROCESSED_BLOCK_STREAM_KEY
}

/// Redis key for the latest chain-state snapshot number.
pub const fn latest_chain_state_block_number_key() -> &'static str {
    LATEST_CHAIN_STATE_KEY
}

/// Sorted-set key tracking recent block numbers.
pub const fn recent_blocks_key() -> &'static str {
    RECENT_BLOCKS_KEY
}

/// Key holding serialized block metadata.
pub fn block_meta_key(block_number: u64) -> String {
    format!("{BLOCK_PREFIX}{block_number}/meta")
}

/// Key holding serialized block headers.
pub fn block_header_key(block_number: u64) -> String {
    format!("{BLOCK_PREFIX}{block_number}/header")
}

/// Key holding serialized processed transactions for a block.
pub fn processed_transactions_key(block_number: u64) -> String {
    format!("{BLOCK_PREFIX}{block_number}/txs")
}

/// Alias matching the Python helper name.
pub fn processed_tx_map_key(block_number: u64) -> String {
    processed_transactions_key(block_number)
}

/// Sorted-set key mapping transaction index to transaction hash for a block.
pub fn tx_index_key(block_number: u64) -> String {
    format!("{BLOCK_PREFIX}{block_number}/tx_index")
}

/// Set key holding addresses observed while processing a block.
pub fn block_addresses_key(block_number: u64) -> String {
    format!("{BLOCK_PREFIX}{block_number}/addresses")
}

/// Key holding serialized chain-state snapshots (overlays).
pub fn chain_state_snapshot_key(block_number: u64) -> String {
    format!("{BLOCK_PREFIX}{block_number}/chain_state_snapshot")
}

/// Key for a live token snapshot (Python parity).
pub fn token_snapshot_key(address: &str) -> String {
    format!("{TOKEN_SNAPSHOT_PREFIX}{address}")
}

/// Key for cached position data (portfolio/token pair).
pub fn position_key(portfolio_id: &str, token_address: &str) -> String {
    format!(
        "{POSITION_PREFIX}{portfolio_id}/{}",
        token_address.to_ascii_lowercase()
    )
}
