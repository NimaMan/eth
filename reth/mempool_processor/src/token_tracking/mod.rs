// rust/mempool_processor/src/token_tracking/mod.rs
//
// Module for subscribing to pool and token creator updates published by the Python component.

pub mod types;
pub mod cache;
pub mod address_tracking_cache;
pub mod token_parameter_extraction;

// Re-export commonly used types
pub use cache::{TokenTrackingCache, CacheStats, UpdateResult};
pub use types::CacheConfig;
pub use types::{Token, Pool, Address, TokenUpdate, TokenWithPools, PoolType};
pub use address_tracking_cache::{AddressTrackingCache, AddressRole};
// Tax calculation functions have been moved to tx_processor
// pub use token_parameter_extraction::{calculate_buy_tax, calculate_sell_tax, TaxCalculationResult};


use zmq;
use tracing::{info, error, debug, warn};
use std::sync::Arc;
use serde_json;

use self::types::{PoolUpdatesMessage, TokenCreatorMessage, TokenCreatorsMessage, TokenUpdatesMessage, TokenQueryResponse};

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
        let config = CacheConfig {
            eth_threshold,
            ..Default::default()
        };
        let cache = Arc::new(TokenTrackingCache::new(config));
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
    
    /// Get a clone of the combined cache for backward compatibility
    pub fn get_pool_cache(&self) -> Arc<TokenTrackingCache> {
        self.cache.clone()
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
        let response: TokenQueryResponse = serde_json::from_str(&response_str)?;
        
        if response.status == "success" {
            let token_count = response.count.unwrap_or(0);
            info!("Received {} tokens from Python service", token_count);
            
            if let Some(token_data) = response.data {
                // Data is already in TokenWithPools format
                let update = types::TokenUpdate {
                    message_type: "initial_load".to_string(),
                    token_count,
                    block_number: 0, // Not provided in response
                    timestamp: 0.0, // Not provided in response
                    data: token_data,
                };
                
                // Update cache with batch update
                let result = self.cache.batch_update(update).await;
                
                info!("✅ Initialized cache with {} tokens, {} pools, {} creators", 
                     result.tokens_updated, result.pools_updated, result.creators_added);
                
                // Log cache stats for verification
                let stats = self.cache.stats().await;
                info!("Cache stats: {} tokens, {} pools, {} creators",
                      stats.total_tokens, stats.total_pools, stats.total_creators);
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
                        
                        // Convert to TokenUpdate for batch processing
                        let update = types::TokenUpdate {
                            message_type: token_message.message_type,
                            token_count: token_message.token_count,
                            block_number: token_message.block_number,
                            timestamp: token_message.timestamp,
                            data: token_message.data,
                        };
                        
                        // Update cache with batch update
                        let result = self.cache.batch_update(update).await;
                        
                        debug!("Updated {} tokens, {} pools, {} creators from token message", 
                               result.tokens_updated, result.pools_updated, result.creators_added);
                        
                    } else if let Ok(pool_message) = serde_json::from_str::<PoolUpdatesMessage>(&msg_str) {
                        // Handle legacy pool updates (backward compatibility)
                        debug!("Received legacy pool update: {} pools", pool_message.data.len());
                        
                        // Note: Legacy pool updates don't contain full token info
                        // For now, we'll skip them as the new cache requires full token data
                        warn!("Legacy pool updates not supported with new cache. Skipping.");
                        
                    } else if let Ok(creator_message) = serde_json::from_str::<TokenCreatorMessage>(&msg_str) {
                        // Handle single creator update
                        debug!("Received token creator update");
                        
                        // Note: Creator-only updates don't contain full token info
                        // For now, we'll skip them as the new cache requires full token data
                        warn!("Creator-only updates not supported with new cache. Skipping.");
                        
                    } else if let Ok(creators_message) = serde_json::from_str::<TokenCreatorsMessage>(&msg_str) {
                        // Handle bulk creator updates
                        debug!("Received bulk token creators update: {} creators", creators_message.data.len());
                        
                        // Note: Creator-only updates don't contain full token info
                        // For now, we'll skip them as the new cache requires full token data
                        warn!("Creator-only updates not supported with new cache. Skipping.");
                        
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
        let cache = subscriber.get_cache();
        // Cache exists and can be accessed
        assert!(Arc::strong_count(&cache) > 0);
    }
    
    #[test]
    fn can_create_subscriber_with_custom_endpoints() {
        let pub_endpoint = "tcp://127.0.0.1:5557";
        let rep_endpoint = "tcp://127.0.0.1:5558";
        let subscriber = TokenTrackingSubscriber::with_endpoints(0.1, pub_endpoint, rep_endpoint);
        assert_eq!(subscriber.zmq_pub_endpoint, pub_endpoint);
        assert_eq!(subscriber.zmq_rep_endpoint, rep_endpoint);
    }
} 