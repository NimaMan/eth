// rust/mempool_processor/src/pool_subscriber/mod.rs
//
// Module for subscribing to pool level updates published by the Python component.

pub mod types;
pub mod cache;

use zmq;
use tracing::{info, error, debug, warn};
use std::sync::Arc;
use serde_json;

use self::types::PoolUpdatesMessage;
use self::cache::PoolStateCache;

// Default ZMQ endpoint for backward compatibility
const DEFAULT_ZMQ_PUB_ENDPOINT: &str = "tcp://localhost:5557";

pub struct PoolSubscriber {
    pool_cache: Arc<PoolStateCache>,
    zmq_endpoint: String,
}

impl PoolSubscriber {
    pub fn new(eth_threshold: f64) -> Self {
        Self::with_endpoint(eth_threshold, DEFAULT_ZMQ_PUB_ENDPOINT)
    }
    
    pub fn with_endpoint(eth_threshold: f64, zmq_endpoint: &str) -> Self {
        let pool_cache = Arc::new(PoolStateCache::new(eth_threshold));
        Self { 
            pool_cache,
            zmq_endpoint: zmq_endpoint.to_string()
        }
    }
    
    /// Get a clone of the pool cache for use by other components
    pub fn get_pool_cache(&self) -> Arc<PoolStateCache> {
        self.pool_cache.clone()
    }

    /// Request initial pool state from Python service via REQ/REP socket
    async fn request_initial_pool_state(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Requesting initial pool state from Python service...");
        
        // Create REQ socket to request full pool state
        let context = zmq::Context::new();
        let requester = context.socket(zmq::REQ)?;
        
        // Connect to REP endpoint (port 5558 by default)
        let rep_endpoint = self.zmq_endpoint.replace("5557", "5558");
        requester.connect(&rep_endpoint)?;
        info!("Connected to Python REP socket at {}", rep_endpoint);
        
        // Create request for all pools
        let request = serde_json::json!({
            "type": "get_all_pools"
        });
        
        // Send request
        requester.send(&request.to_string(), 0)?;
        debug!("Sent get_all_pools request");
        
        // Receive response
        let response_str = match requester.recv_string(0) {
            Ok(Ok(s)) => s,
            Ok(Err(e)) => return Err(format!("ZMQ string conversion error: {:?}", e).into()),
            Err(e) => return Err(format!("ZMQ recv error: {:?}", e).into()),
        };
        debug!("Received response from Python service");
        
        // Parse response
        let response: serde_json::Value = serde_json::from_str(&response_str)?;
        
        if response["status"] == "success" {
            let pool_count = response["count"].as_u64().unwrap_or(0);
            info!("Received {} pools from Python service", pool_count);
            
            if let Some(pool_data) = response["data"].as_object() {
                // Convert to our PoolUpdate format
                let mut pools_map = std::collections::HashMap::new();
                
                for (address, data) in pool_data {
                    if let Some(pool_obj) = data.as_object() {
                        // Extract required fields with defaults
                        let eth_reserve = pool_obj.get("eth_reserve").and_then(|v| v.as_f64()).unwrap_or(0.0);
                        let token_address = pool_obj.get("token_address").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        let block_number = pool_obj.get("block_number").and_then(|v| v.as_u64()).unwrap_or(0);
                        let update_time = pool_obj.get("update_time").and_then(|v| v.as_f64()).unwrap_or(0.0);
                        
                        let pool_update = types::PoolUpdate {
                            eth_reserve,
                            token_address,
                            block_number,
                            update_time,
                        };
                        
                        pools_map.insert(address.clone(), pool_update);
                    }
                }
                
                // Update cache with initial data
                let updated_pools = self.pool_cache.update_pools(pools_map.iter());
                info!("Initialized cache with {} pools (above threshold: {})", 
                     pool_count, updated_pools.len());
                
                // Log some sample pools for verification
                if !updated_pools.is_empty() {
                    info!("Sample initialized pools:");
                    for (i, addr) in updated_pools.iter().take(5).enumerate() {
                        if let Some(pool_state) = self.pool_cache.get_pool(addr) {
                            info!("  {}: {} ({:.6} ETH)", i+1, addr, pool_state.eth_reserve);
                        }
                    }
                }
            }
        } else {
            warn!("Failed to get initial pool state: {}", response["error"].as_str().unwrap_or("Unknown error"));
        }
        
        Ok(())
    }

    pub async fn start_listening(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Initializing ZMQ subscriber for pool levels.");
        let context = zmq::Context::new();
        let subscriber = context.socket(zmq::SUB)?;

        info!("Connecting ZMQ SUB socket to Python publisher at {}", self.zmq_endpoint);
        subscriber.connect(&self.zmq_endpoint)?;

        // Subscribe to all messages (empty subscription prefix)
        subscriber.set_subscribe(b"")?;
        debug!("Subscribed to all messages from publisher.");

        // Request initial full pool state
        if let Err(e) = self.request_initial_pool_state().await {
            warn!("Failed to get initial pool state: {}", e);
        }

        info!("Listening for pool level updates from Python...");
        loop {
            match subscriber.recv_string(0) {
                Ok(Ok(msg_str)) => {
                    debug!("Received message from Python publisher");
                    
                    // Attempt to deserialize the JSON message
                    match serde_json::from_str::<PoolUpdatesMessage>(&msg_str) {
                        Ok(message) => {
                            // Only log significant events, not every update
                            if message.data.len() > 10 {
                                debug!("Large pool update: {} pools", message.data.len());
                            }
                            
                            // Update the cache
                            let updated_pools = self.pool_cache.update_pools(message.data.iter());
                            
                            // Only log cache updates for debugging if needed
                            debug!("Updated {} pools in cache", updated_pools.len());
                        },
                        Err(e) => {
                            warn!("Failed to parse pool update JSON: {}", e);
                            debug!("Raw message: {}", msg_str);
                        }
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
mod tests {
    use super::*;

    #[test]
    fn can_create_subscriber() {
        let subscriber = PoolSubscriber::new(0.05);
        assert!(subscriber.get_pool_cache().get_eth_threshold() == 0.05);
    }
    
    #[test]
    fn can_create_subscriber_with_custom_endpoint() {
        let endpoint = "tcp://127.0.0.1:5558";
        let subscriber = PoolSubscriber::with_endpoint(0.1, endpoint);
        assert!(subscriber.get_pool_cache().get_eth_threshold() == 0.1);
        assert_eq!(subscriber.zmq_endpoint, endpoint);
    }
} 