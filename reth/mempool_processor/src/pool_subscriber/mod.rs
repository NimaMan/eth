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

    pub async fn start_listening(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Initializing ZMQ subscriber for pool levels.");
        let context = zmq::Context::new();
        let subscriber = context.socket(zmq::SUB)?;

        info!("Connecting ZMQ SUB socket to Python publisher at {}", self.zmq_endpoint);
        subscriber.connect(&self.zmq_endpoint)?;

        // Subscribe to all messages (empty subscription prefix)
        subscriber.set_subscribe(b"")?;
        debug!("Subscribed to all messages from publisher.");

        info!("Listening for pool level updates from Python...");
        loop {
            match subscriber.recv_string(0) {
                Ok(Ok(msg_str)) => {
                    debug!("Received message from Python publisher");
                    
                    // Attempt to deserialize the JSON message
                    match serde_json::from_str::<PoolUpdatesMessage>(&msg_str) {
                        Ok(message) => {
                            let pool_count = message.data.len();
                            info!("Successfully deserialized pool update with {} pools, timestamp: {}", 
                                  pool_count, message.timestamp);
                            
                            // Log some sample data (first few pools)
                            let mut sample_count = 0;
                            for (addr, update) in message.data.iter().take(2) {
                                debug!("Pool {}: address {}, ETH reserve: {}, block: {}", 
                                      sample_count, addr, update.eth_reserve, update.block_number);
                                sample_count += 1;
                            }
                            if pool_count > 2 {
                                debug!("... and {} more pools", pool_count - 2);
                            }
                            
                            // Update the pool cache with the new data
                            let updated_pools = self.pool_cache.update_pools(message.data.iter());
                            info!("Updated {} pools in cache", updated_pools.len());
                            
                            // In the future, this is where we would trigger scam detection
                            // if the 'trigger on new pool update' approach is used
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