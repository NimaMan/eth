// pool_subscriber/types.rs
//
// Type definitions for pool data structures, primarily used for deserializing 
// JSON messages from the Python ZMQ publisher.

use serde::Deserialize;
use std::collections::HashMap;

/// Represents an individual pool update with ETH reserve and related metadata.
#[derive(Debug, Clone, Deserialize)]
pub struct PoolUpdate {
    /// Current ETH reserve level in the pool
    pub eth_reserve: f64,
    
    /// Address of the token in this pool
    pub token_address: String,
    
    /// Ethereum block number when this pool data was observed
    pub block_number: u64,
    
    /// Unix timestamp when the update was processed
    pub update_time: f64,
}

/// Represents the complete message format received from the Python publisher.
/// The message contains metadata and a map of pool addresses to their current state.
#[derive(Debug, Clone, Deserialize)]
pub struct PoolUpdatesMessage {
    /// Message type, expected to be "pool_updates"
    #[serde(rename = "type")]
    pub message_type: String,
    
    /// Unix timestamp when the message was created
    pub timestamp: f64,
    
    /// Map of pool addresses to their current state
    /// Keys are pool contract addresses (as hex strings)
    /// Values are the current state of each pool
    pub data: HashMap<String, PoolUpdate>,
}

/// Represents a simplified pool state for storage in the pool state cache.
/// This contains just the essential information needed for scam detection.
#[derive(Debug, Clone)]
pub struct PoolState {
    /// Current ETH reserve level in the pool
    pub eth_reserve: f64,
    
    /// Address of the token in this pool
    pub token_address: String,
    
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
            token_address: update.token_address,
            last_updated_block: update.block_number,
            last_updated_time: update.update_time,
            received_at: std::time::Instant::now(),
        }
    }
} 