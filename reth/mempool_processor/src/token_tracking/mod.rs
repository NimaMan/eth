// rust/mempool_processor/src/token_tracking/mod.rs
//
// Module for subscribing to pool and token creator updates published by the Python component.

pub mod types;
pub mod cache;
pub mod address_tracking_cache;
pub mod signal_integration;

// Re-export commonly used types
pub use cache::{PoolStateCache, TokenCreatorCache, TokenTrackingCache};
pub use address_tracking_cache::{AddressTrackingCache, AddressRole};
pub use signal_integration::SignalIntegration;


use zmq;
use tracing::{info, error, debug, warn};
use std::sync::Arc;
use serde_json;
use crate::common::address::checksum_address;

use self::types::{PoolUpdatesMessage, TokenCreatorMessage, TokenCreatorsMessage, TokenUpdatesMessage};

// Default ZMQ endpoints
const DEFAULT_ZMQ_PUB_ENDPOINT: &str = "tcp://localhost:5557";
const DEFAULT_ZMQ_REP_ENDPOINT: &str = "tcp://localhost:5558";

pub struct TokenTrackingSubscriber {
    cache: Arc<TokenTrackingCache>,
    zmq_pub_endpoint: String,
    zmq_rep_endpoint: String,
}

impl TokenTrackingSubscriber {
    pub fn new(eth_threshold: f64) -> Self {
        Self::with_endpoints(eth_threshold, DEFAULT_ZMQ_PUB_ENDPOINT, DEFAULT_ZMQ_REP_ENDPOINT)
    }
    
    pub fn with_endpoints(eth_threshold: f64, pub_endpoint: &str, rep_endpoint: &str) -> Self {
        let cache = Arc::new(TokenTrackingCache::new(eth_threshold));
        Self { 
            cache,
            zmq_pub_endpoint: pub_endpoint.to_string(),
            zmq_rep_endpoint: rep_endpoint.to_string(),
        }
    }
    
    /// Get a clone of the combined cache for use by other components
    pub fn get_cache(&self) -> Arc<TokenTrackingCache> {
        self.cache.clone()
    }
    
    /// Get a clone of just the pool cache for backward compatibility
    pub fn get_pool_cache(&self) -> Arc<PoolStateCache> {
        Arc::new(self.cache.pools.clone())
    }

    /// Request initial pool state from Python service via REQ/REP socket
    async fn request_initial_pool_state(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Requesting pool data from Python service...");
        
        // Create REQ socket to request pool data
        let context = zmq::Context::new();
        let requester = context.socket(zmq::REQ)?;
        
        // Connect to REP endpoint
        requester.connect(&self.zmq_rep_endpoint)?;
        info!("Connected to Python REP socket at {}", self.zmq_rep_endpoint);
        
        // Create request for all tokens
        let request = serde_json::json!({
            "type": "get_all_tokens"
        });
        
        // Send request
        requester.send(&request.to_string(), 0)?;
        debug!("Sent get_all_tokens request");
        
        // Receive response
        let response_str = match requester.recv_string(0) {
            Ok(Ok(s)) => s,
            Ok(Err(e)) => return Err(format!("ZMQ string conversion error: {:?}", e).into()),
            Err(e) => return Err(format!("ZMQ recv error: {:?}", e).into()),
        };
        debug!("Received response from Python service");
        
        // Parse response
        let response: types::TokenQueryResponse = serde_json::from_str(&response_str)?;
        
        if response.status == "success" {
            let token_count = response.count.unwrap_or(0);
            info!("Received {} tokens from Python service", token_count);
            
            if let Some(token_data) = response.data {
                // Extract pools from token-centric data
                let mut pools_map = std::collections::HashMap::new();
                let mut creators_map = std::collections::HashMap::new();
                
                for (token_address, token_info) in token_data {
                    // Store creator info
                    let creator = types::TokenCreator {
                        creator_address: token_info.creator_address.clone(),
                        token_address: token_address.clone(),
                        creation_block: token_info.creation_block,
                        creation_tx_hash: token_info.creation_txn.clone(),
                        creation_time: 0.0, // Not provided in new format
                        uses_private_mempool: false, // Default value
                    };
                    creators_map.insert(checksum_address(&token_address), creator);
                    
                    // Extract pools from this token
                    for (pool_address, pool_info) in token_info.pools {
                        let pool_update = types::PoolUpdate {
                            eth_reserve: pool_info.denom_reserve,
                            token_reserve: pool_info.token_reserve,
                            token_address: checksum_address(&token_address),
                            block_number: pool_info.latest_block_number,
                            update_time: pool_info.last_update_time.unwrap_or(0.0),
                        };
                        
                        // Store with checksummed pool address
                        let checksummed_address = checksum_address(&pool_address);
                        pools_map.insert(checksummed_address, pool_update);
                    }
                }
                
                // Update caches with extracted data
                let updated_pools = self.cache.pools.update_pools(pools_map.iter()).await;
                let updated_creators = self.cache.creators.update_creators(creators_map.iter()).await;
                
                info!("✅ Initialized cache with {} tokens containing {} pools (above threshold: {})", 
                     token_count, pools_map.len(), updated_pools.len());
                info!("✅ Initialized {} token creators", updated_creators);
                
                // Log some sample pools for verification
                if !updated_pools.is_empty() {
                    info!("Sample initialized pools:");
                    for (i, addr) in updated_pools.iter().take(3).enumerate() {
                        if let Some(pool_state) = self.cache.pools.get_pool(addr).await {
                            info!("  {}: {} ({:.6} ETH)", i+1, addr, pool_state.eth_reserve);
                        }
                    }
                }
            }
        } else {
            warn!("Failed to get token data: {}", response.error.unwrap_or_else(|| "Unknown error".to_string()));
        }
        
        Ok(())
    }

    /// Request initial token creator data from Python service
    /// NOTE: This is now handled within request_initial_pool_data since creators are embedded in token data
    async fn request_initial_creator_data(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Creator data is now loaded as part of token data in request_initial_pool_data
        debug!("Creator data already loaded from token data");
        Ok(())
    }

    pub async fn start_listening(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("🚀 Starting token tracking subscriber");
        
        let context = zmq::Context::new();
        let subscriber = context.socket(zmq::SUB)?;

        info!("Connecting ZMQ SUB socket to Python publisher at {}", self.zmq_pub_endpoint);
        subscriber.connect(&self.zmq_pub_endpoint)?;

        // Subscribe to all messages (empty subscription prefix)
        subscriber.set_subscribe(b"")?;
        debug!("Subscribed to all messages from publisher.");

        // Request initial data from Python
        if let Err(e) = self.request_initial_pool_state().await {
            warn!("Failed to get initial pool state: {}", e);
        }
        
        if let Err(e) = self.request_initial_creator_data().await {
            warn!("Failed to get initial creator data: {}", e);
        }

        info!("📊 Real-time token tracking updates active");
        loop {
            match subscriber.recv_string(0) {
                Ok(Ok(msg_str)) => {
                    debug!("Received update message from Python publisher");
                    
                    // Try to parse as new token-centric message format first
                    if let Ok(token_message) = serde_json::from_str::<TokenUpdatesMessage>(&msg_str) {
                        // Handle token-centric updates
                        match token_message.message_type.as_str() {
                            "full_update" => {
                                info!("Received full update with {} tokens", token_message.token_count);
                            }
                            "block_update" => {
                                debug!("Received block update with {} tokens", token_message.token_count);
                            }
                            _ => {
                                warn!("Unknown token message type: {}", token_message.message_type);
                            }
                        }
                        
                        // Extract pools and creators from token data
                        let mut pools_map = std::collections::HashMap::new();
                        let mut creators_map = std::collections::HashMap::new();
                        
                        for (token_address, token_info) in token_message.data.iter() {
                            // Store creator info
                            let creator = types::TokenCreator {
                                creator_address: token_info.creator_address.clone(),
                                token_address: token_address.clone(),
                                creation_block: token_info.creation_block,
                                creation_tx_hash: token_info.creation_txn.clone(),
                                creation_time: 0.0, // Not provided in new format
                                uses_private_mempool: false, // Default value
                            };
                            creators_map.insert(checksum_address(token_address), creator);
                            
                            // Extract pools from this token
                            for (pool_address, pool_info) in token_info.pools.iter() {
                                let pool_update = types::PoolUpdate {
                                    eth_reserve: pool_info.denom_reserve,
                                    token_reserve: pool_info.token_reserve,
                                    token_address: checksum_address(token_address),
                                    block_number: pool_info.latest_block_number,
                                    update_time: pool_info.last_update_time.unwrap_or(0.0),
                                };
                                
                                // Store with checksummed pool address
                                let checksummed_address = checksum_address(pool_address);
                                pools_map.insert(checksummed_address, pool_update);
                            }
                        }
                        
                        // Update caches with extracted data
                        let updated_pools = self.cache.pools.update_pools(pools_map.iter()).await;
                        let updated_creators = self.cache.creators.update_creators(creators_map.iter()).await;
                        
                        debug!("Updated {} pools and {} creators from token message", 
                               updated_pools.len(), updated_creators);
                        
                    } else if let Ok(pool_message) = serde_json::from_str::<PoolUpdatesMessage>(&msg_str) {
                        // Handle legacy pool updates (backward compatibility)
                        debug!("Received legacy pool update: {} pools", pool_message.data.len());
                        
                        let mut python_updates = std::collections::HashMap::new();
                        
                        for (address, update) in pool_message.data.iter() {
                            let checksummed_address = checksum_address(address);
                            let mut python_update = update.clone();
                            python_update.token_address = checksum_address(&update.token_address);
                            
                            python_updates.insert(checksummed_address, python_update);
                        }
                        
                        let updated_pools = self.cache.pools.update_pools(python_updates.iter()).await;
                        debug!("Updated {} pools in cache", updated_pools.len());
                        
                    } else if let Ok(creator_message) = serde_json::from_str::<TokenCreatorMessage>(&msg_str) {
                        // Handle single creator update
                        debug!("Received token creator update");
                        
                        let checksummed_token = checksum_address(&creator_message.creator.token_address);
                        let mut creators_map = std::collections::HashMap::new();
                        creators_map.insert(checksummed_token, creator_message.creator);
                        
                        let updated_count = self.cache.creators.update_creators(creators_map.iter()).await;
                        debug!("Updated {} creators in cache", updated_count);
                        
                    } else if let Ok(creators_message) = serde_json::from_str::<TokenCreatorsMessage>(&msg_str) {
                        // Handle bulk creator updates
                        debug!("Received bulk token creators update: {} creators", creators_message.data.len());
                        
                        let mut creators_map = std::collections::HashMap::new();
                        for (token_address, creator) in creators_message.data.iter() {
                            let checksummed_token = checksum_address(token_address);
                            creators_map.insert(checksummed_token, creator.clone());
                        }
                        
                        let updated_count = self.cache.creators.update_creators(creators_map.iter()).await;
                        debug!("Updated {} creators in cache", updated_count);
                        
                    } else {
                        warn!("Failed to parse update message as any known type");
                        debug!("Raw message: {}", msg_str);
                    }
                }
                Ok(Err(zmq_err)) => {
                    // ZMQ string conversion error
                    error!("Error converting ZMQ message to string: {:?}", zmq_err);
                }
                Err(e) => {
                    // ZMQ recv error
                    error!("Error receiving ZMQ message: {:?}", e);
                    // Consider adding a small delay or break/reconnect logic here
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }
            }
            // Yield to allow other tasks to run, especially if in a tight loop without actual blocking IO
            tokio::task::yield_now().await;
        }
        // Unreachable in the current loop, but good practice for future changes
        // Ok(())
    }
}

// Basic test function to ensure the module structure is sound
#[cfg(test)]
mod basic_tests {
    use super::*;

    #[test]
    fn can_create_subscriber() {
        let subscriber = TokenTrackingSubscriber::new(0.05);
        assert!(subscriber.get_pool_cache().get_eth_threshold() == 0.05);
    }
    
    #[test]
    fn can_create_subscriber_with_custom_endpoints() {
        let pub_endpoint = "tcp://127.0.0.1:5557";
        let rep_endpoint = "tcp://127.0.0.1:5558";
        let subscriber = TokenTrackingSubscriber::with_endpoints(0.1, pub_endpoint, rep_endpoint);
        assert!(subscriber.get_pool_cache().get_eth_threshold() == 0.1);
        assert_eq!(subscriber.zmq_pub_endpoint, pub_endpoint);
        assert_eq!(subscriber.zmq_rep_endpoint, rep_endpoint);
    }
    
    #[test]
    fn can_access_both_caches() {
        let subscriber = TokenTrackingSubscriber::new(0.05);
        let combined_cache = subscriber.get_cache();
        let pool_cache = subscriber.get_pool_cache();
        
        assert!(combined_cache.pools.get_eth_threshold() == 0.05);
        assert!(pool_cache.get_eth_threshold() == 0.05);
    }
} 