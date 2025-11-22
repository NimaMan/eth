//! Canonical Redis key helpers shared across simulators and processors.

const LATEST_BLOCK_NUMBER_KEY: &str = "block:latest_block_number";
const BLOCK_HEADER_PREFIX: &str = "block:block_header:";
const PROCESSED_TX_PREFIX: &str = "block:processed_transactions:";
const CHAIN_STATE_PREFIX: &str = "block:chain_state_snapshot:";
const LATEST_CHAIN_STATE_KEY: &str = "block:latest_chain_state_block_number";
const TOKEN_SNAPSHOT_PREFIX: &str = "token:snapshot:";
const POSITION_PREFIX: &str = "position:";

/// Redis key for the latest known block number.
pub const fn latest_block_number_key() -> &'static str {
    LATEST_BLOCK_NUMBER_KEY
}

/// Redis key for the latest chain-state snapshot number.
pub const fn latest_chain_state_block_number_key() -> &'static str {
    LATEST_CHAIN_STATE_KEY
}

/// Key holding serialized block headers.
pub fn block_header_key(block_number: u64) -> String {
    format!("{BLOCK_HEADER_PREFIX}{block_number}")
}

/// Key holding serialized processed transactions for a block.
pub fn processed_transactions_key(block_number: u64) -> String {
    format!("{PROCESSED_TX_PREFIX}{block_number}")
}

/// Alias matching the Python helper name.
pub fn processed_tx_map_key(block_number: u64) -> String {
    processed_transactions_key(block_number)
}

/// Key holding serialized chain-state snapshots (overlays).
pub fn chain_state_snapshot_key(block_number: u64) -> String {
    format!("{CHAIN_STATE_PREFIX}{block_number}")
}

/// Key for a live token snapshot (Python parity).
pub fn token_snapshot_key(address: &str) -> String {
    format!("{TOKEN_SNAPSHOT_PREFIX}{address}")
}

/// Key for cached position data (portfolio/token pair).
pub fn position_key(portfolio_id: &str, token_address: &str) -> String {
    format!(
        "{POSITION_PREFIX}{portfolio_id}:{}",
        token_address.to_ascii_lowercase()
    )
}
