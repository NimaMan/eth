// token_tracking/types.rs
//
// Type definitions for token and pool data structures from Python ZMQ publisher

use serde::{Deserialize, Serialize, Deserializer};
use std::collections::HashMap;

/// Pool-specific information within a token
#[derive(Debug, Clone, Deserialize)]
pub struct PoolInfo {
    /// Pool contract address
    pub pool_address: String,
    
    /// Pool type (V2, V3, V4)
    pub pool_type: String,
    
    /// Currency of the denomination (usually ETH or stablecoin)
    pub denom_currency: String,
    
    /// Address of the denomination token
    pub denom_address: String,
    
    /// Current denomination reserve in the pool
    pub denom_reserve: f64,
    
    /// Current token reserve in the pool
    pub token_reserve: f64,
    
    /// Block number of latest update
    pub latest_block_number: u64,
    
    /// Unix timestamp of last update
    /// TODO: Remove Option once Python publisher is updated to always send this field
    pub last_update_time: Option<f64>,
    
    /// Pool ID for V4 pools
    pub pool_id: Option<String>,
    
    /// Fee tier (for V3/V4 pools)
    pub fee_tier: Option<u32>,
    
    /// Whether this pool is identified as a scam
    pub is_scam: bool,
    
    /// Reason for scam classification
    pub scam_label: Option<String>,
}

/// Token information including all its pools
#[derive(Debug, Clone, Deserialize)]
pub struct TokenInfo {
    /// Token contract address
    pub token_address: String,
    
    /// Creator wallet address
    pub creator_address: String,
    
    /// Block when token was created
    pub creation_block: u64,
    
    /// Transaction hash of token creation
    #[serde(alias = "creation_tx")]
    pub creation_txn: String,
    
    /// Whether trading is enabled
    pub trading_enabled: bool,
    
    /// Transaction hash when trading was enabled
    #[serde(alias = "trading_enabled_tx")]
    pub trading_enabled_txn: Option<String>,
    
    /// Current owner address
    pub current_owner: String,
    
    /// Whether ownership is renounced
    pub ownership_renounced: bool,
    
    /// Whether token is marked as scam
    pub is_scam: bool,
    
    /// Scam classification reason
    pub scam_label: Option<String>,
    
    /// Block of latest activity
    pub latest_activity_block: u64,
    
    /// Current buy tax percentage (0-100)
    #[serde(skip)]
    pub buy_tax: Option<u8>,
    
    /// Current sell tax percentage (0-100)
    #[serde(skip)]
    pub sell_tax: Option<u8>,
    
    /// Last tax update transaction hash
    pub last_tax_update_txn: Option<String>,
    
    /// Map of pool addresses to pool information
    pub pools: HashMap<String, PoolInfo>,
    
    /// Token metadata
    #[serde(alias = "token_symbol")]
    pub symbol: Option<String>,
    #[serde(alias = "token_name")]
    pub name: Option<String>,
    #[serde(alias = "token_decimals")]
    pub decimals: Option<u8>,
    #[serde(deserialize_with = "deserialize_supply")]
    pub total_supply: Option<String>,
    
    /// Tax setter addresses (list from Python)
    #[serde(default)]
    pub tax_setter_addresses: Vec<String>,
    
    /// Derived tax setter addresses  
    #[serde(skip)]
    pub buy_tax_setter: Option<String>,
    #[serde(skip)]
    pub sell_tax_setter: Option<String>,
    
    /// Current buy tax from Python
    #[serde(alias = "current_buy_tax")]
    pub buy_tax_python: Option<f64>,
    
    /// Current sell tax from Python  
    #[serde(alias = "current_sell_tax")]
    pub sell_tax_python: Option<f64>,
    
    /// Simulation results from Rust
    pub simulation_data: Option<SimulationData>,
}

/// Custom deserializer for total_supply that handles both strings and numbers
fn deserialize_supply<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrNumber {
        String(String),
        Number(f64),
    }

    match Option::<StringOrNumber>::deserialize(deserializer)? {
        Some(StringOrNumber::String(s)) => Ok(Some(s)),
        Some(StringOrNumber::Number(n)) => Ok(Some(n.to_string())),
        None => Ok(None),
    }
}

/// Simulation results data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationData {
    /// Whether buy transactions succeed
    pub can_buy: bool,
    
    /// Whether sell transactions succeed
    pub can_sell: bool,
    
    /// Measured buy tax from simulation
    pub measured_buy_tax: Option<f64>,
    
    /// Measured sell tax from simulation
    pub measured_sell_tax: Option<f64>,
    
    /// Whether token is detected as honeypot
    pub is_honeypot: bool,
    
    /// Block number when simulation was last run
    pub last_simulated_block: u64,
    
    /// Error message if simulation failed
    pub simulation_error: Option<String>,
}

/// Message format for token updates from Python
#[derive(Debug, Clone, Deserialize)]
pub struct TokenUpdatesMessage {
    /// Message type: "full_update" or "block_update"
    #[serde(rename = "type")]
    pub message_type: String,
    
    /// Unix timestamp when message was created
    pub timestamp: f64,
    
    /// Number of tokens in this update
    pub token_count: usize,
    
    /// Map of token addresses to their information
    pub data: HashMap<String, TokenInfo>,
}

/// Response format for REP socket queries
#[derive(Debug, Clone, Deserialize)]
pub struct TokenQueryResponse {
    /// Status: "success" or "error"
    pub status: String,
    
    /// Number of tokens returned
    pub count: Option<usize>,
    
    /// Error message if status is "error"
    pub error: Option<String>,
    
    /// Token data if successful
    pub data: Option<HashMap<String, TokenInfo>>,
}

// Keep legacy types for backward compatibility
/// Legacy pool update structure
#[derive(Debug, Clone, Deserialize)]
pub struct PoolUpdate {
    /// Current ETH reserve level in the pool
    #[serde(alias = "denom_reserve")]
    pub eth_reserve: f64,
    
    /// Current token reserve level in the pool
    pub token_reserve: f64,
    
    /// Address of the token in this pool
    pub token_address: String,
    
    /// Pool type (V2, V3, V4)
    pub pool_type: String,
    
    /// Ethereum block number when this pool data was observed
    #[serde(alias = "latest_block_number")]
    pub block_number: u64,
    
    /// Unix timestamp when the update was processed
    #[serde(default)]
    pub update_time: f64,
}


/// Legacy message format
#[derive(Debug, Clone, Deserialize)]
pub struct PoolUpdatesMessage {
    /// Message type, expected to be "token_updates"
    #[serde(rename = "type")]
    pub message_type: String,
    
    /// Unix timestamp when the message was created
    pub timestamp: f64,
    
    /// Map of pool addresses to their current state
    pub data: HashMap<String, PoolUpdate>,
}

/// Represents a simplified pool state for storage in the pool state cache.
/// This contains just the essential information needed for scam detection.
#[derive(Debug, Clone)]
pub struct PoolState {
    /// Current ETH reserve level in the pool
    pub eth_reserve: f64,
    
    /// Current token reserve level in the pool
    pub token_reserve: f64,
    
    /// Address of the token in this pool
    pub token_address: String,
    
    /// Pool type (V2, V3, V4)
    pub pool_type: String,
    
    /// Ethereum block number when this pool data was last updated
    pub last_updated_block: u64,
    
    /// Unix timestamp when the pool state was last updated
    pub last_updated_time: f64,
    
    /// System timestamp when we received this update (for staleness checks)
    pub received_at: std::time::Instant,
}

impl PoolState {
    /// Check if this pool state is stale (older than the specified duration)
    pub fn is_stale(&self, max_age: std::time::Duration) -> bool {
        self.received_at.elapsed() > max_age
    }
    
    /// Get the age of this pool state
    pub fn age(&self) -> std::time::Duration {
        self.received_at.elapsed()
    }
}

impl From<PoolUpdate> for PoolState {
    fn from(update: PoolUpdate) -> Self {
        Self {
            eth_reserve: update.eth_reserve,
            token_reserve: update.token_reserve,
            token_address: update.token_address,
            pool_type: update.pool_type,
            last_updated_block: update.block_number,
            last_updated_time: update.update_time,
            received_at: std::time::Instant::now(),
        }
    }
}

/// Represents basic token creator information.
#[derive(Debug, Clone, Deserialize)]
pub struct TokenCreator {
    /// Ethereum address of the token creator
    pub creator_address: String,
    
    /// Address of the token created
    pub token_address: String,
    
    /// Block number when the token was created
    pub creation_block: u64,
    
    /// Transaction hash where the token was created
    pub creation_tx_hash: String,
    
    /// Unix timestamp when the token was created
    pub creation_time: f64,
    
    /// Whether this creator uses private mempool (true) or public mempool (false)
    pub uses_private_mempool: bool,
}

/// Message format for token creator updates from Python.
#[derive(Debug, Clone, Deserialize)]
pub struct TokenCreatorMessage {
    /// Message type, expected to be "token_creator_update"
    #[serde(rename = "type")]
    pub message_type: String,
    
    /// Unix timestamp when the message was created
    pub timestamp: f64,
    
    /// Token creator information
    pub creator: TokenCreator,
}

/// Message format for bulk token creator data requests.
#[derive(Debug, Clone, Deserialize)]
pub struct TokenCreatorsMessage {
    /// Message type, expected to be "token_creators"
    #[serde(rename = "type")]
    pub message_type: String,
    
    /// Unix timestamp when the message was created
    pub timestamp: f64,
    
    /// Map of token addresses to their creator information
    /// Keys are token contract addresses (as hex strings)
    /// Values are the creator information for each token
    pub data: HashMap<String, TokenCreator>,
}

/// Observed transaction from a creator for mempool visibility tracking
#[derive(Debug, Clone)]
pub struct ObservedTransaction {
    pub tx_hash: String,
    pub block_number: Option<u64>,
    pub function_name: String,
    pub timestamp: u64,
    pub seen_in_mempool: bool,
}

/// Represents the current state of a token creator in the cache.
#[derive(Debug, Clone)]
pub struct TokenCreatorState {
    /// Token creator information
    pub creator: TokenCreator,
    
    /// System timestamp when we received this creator info (for staleness checks)
    pub received_at: std::time::Instant,
    
    /// Observed transactions from this creator
    pub observed_transactions: Vec<ObservedTransaction>,
    
    /// Mempool usage pattern (None = unknown, Some(true) = private, Some(false) = public)
    pub mempool_usage_determined: Option<bool>,
}

impl TokenCreatorState {
    /// Check if this creator state is stale (older than the specified duration)
    pub fn is_stale(&self, max_age: std::time::Duration) -> bool {
        self.received_at.elapsed() > max_age
    }
    
    /// Get the age of this creator state
    pub fn age(&self) -> std::time::Duration {
        self.received_at.elapsed()
    }
}

impl From<TokenCreator> for TokenCreatorState {
    fn from(creator: TokenCreator) -> Self {
        Self {
            creator,
            received_at: std::time::Instant::now(),
            observed_transactions: Vec::new(),
            mempool_usage_determined: None,
        }
    }
} 