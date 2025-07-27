// examples/token_tracking/basic_connection.rs
//
// Example showing basic connection to Python token tracking publisher
// Tests ability to connect, receive initial data, and process updates

use std::collections::HashMap;
use mempool_processor::token_tracking::{
    types::{TokenQueryResponse, TokenUpdatesMessage},
    AddressTrackingCache,
};
use serde_json;
use tracing::{info, error, warn, debug};
use zmq;

const ZMQ_PUB_ENDPOINT: &str = "tcp://localhost:5557";
const ZMQ_REP_ENDPOINT: &str = "tcp://localhost:5558";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    info!("🚀 Starting basic token tracking connection test");
    
    // Create address tracking cache
    let cache = AddressTrackingCache::new();
    
    // Test 1: Request initial token data
    info!("📊 Test 1: Requesting initial token data from Python");
    match request_initial_tokens().await {
        Ok(response) => {
            info!("✅ Successfully received {} tokens", response.count.unwrap_or(0));
            
            if let Some(token_data) = response.data {
                // Load data into cache
                let mut tokens_loaded = 0;
                for (token_address, token_info) in token_data.iter().take(5) { // Show first 5 tokens
                    info!("Token: {} - Creator: {} - Owner: {} - Pools: {}", 
                        token_address, 
                        token_info.creator_address, 
                        token_info.current_owner,
                        token_info.pools.len()
                    );
                    
                    // Convert pools to the format expected by cache
                    let pools: Vec<(String, f64, f64)> = token_info.pools.iter()
                        .map(|(addr, pool)| (addr.clone(), pool.denom_reserve, pool.token_reserve))
                        .collect();
                    
                    // Update cache
                    cache.update_token_data(
                        token_address,
                        &token_info.creator_address,
                        &token_info.current_owner,
                        pools,
                        token_info.trading_enabled,
                        token_info.is_scam,
                    ).await;
                    
                    tokens_loaded += 1;
                }
                
                info!("✅ Loaded {} tokens into cache", tokens_loaded);
                
                // Test cache functionality
                let stats = cache.get_stats().await;
                info!("📈 Cache stats: {} addresses, {} tokens, {} pools", 
                    stats.tracked_addresses, stats.tracked_tokens, stats.tracked_pools);
            }
        }
        Err(e) => {
            error!("❌ Failed to get initial token data: {}", e);
        }
    }
    
    // Test 2: Listen for updates
    info!("📡 Test 2: Listening for token updates from Python");
    match listen_for_updates(&cache).await {
        Ok(_) => info!("✅ Update listening completed"),
        Err(e) => error!("❌ Update listening failed: {}", e),
    }
    
    Ok(())
}

async fn request_initial_tokens() -> Result<TokenQueryResponse, Box<dyn std::error::Error>> {
    info!("Connecting to Python REP socket at {}", ZMQ_REP_ENDPOINT);
    
    let context = zmq::Context::new();
    let requester = context.socket(zmq::REQ)?;
    requester.connect(ZMQ_REP_ENDPOINT)?;
    
    // Create request for all tokens
    let request = serde_json::json!({
        "type": "get_all_tokens"
    });
    
    info!("Sending get_all_tokens request");
    requester.send(&request.to_string(), 0)?;
    
    // Receive response
    let response_str = match requester.recv_string(0) {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => return Err(format!("ZMQ string conversion error: {:?}", e).into()),
        Err(e) => return Err(format!("ZMQ recv error: {:?}", e).into()),
    };
    
    debug!("Received response: {} bytes", response_str.len());
    
    // Parse response
    let response: TokenQueryResponse = serde_json::from_str(&response_str)?;
    
    Ok(response)
}

async fn listen_for_updates(cache: &AddressTrackingCache) -> Result<(), Box<dyn std::error::Error>> {
    info!("Connecting to Python PUB socket at {}", ZMQ_PUB_ENDPOINT);
    
    let context = zmq::Context::new();
    let subscriber = context.socket(zmq::SUB)?;
    subscriber.connect(ZMQ_PUB_ENDPOINT)?;
    
    // Subscribe to all messages
    subscriber.set_subscribe(b"")?;
    info!("Subscribed to all token update messages");
    
    let mut message_count = 0;
    let max_messages = 10; // Limit for example
    
    while message_count < max_messages {
        match subscriber.recv_string(0) {
            Ok(Ok(msg_str)) => {
                message_count += 1;
                debug!("Received message {} ({} bytes)", message_count, msg_str.len());
                
                // Try to parse as token update message
                match serde_json::from_str::<TokenUpdatesMessage>(&msg_str) {
                    Ok(token_message) => {
                        info!("📨 Received {} update with {} tokens", 
                            token_message.message_type, token_message.token_count);
                        
                        // Process a few tokens from the update
                        let mut processed = 0;
                        for (token_address, token_info) in token_message.data.iter().take(3) {
                            // Update cache with new data
                            let pools: Vec<(String, f64, f64)> = token_info.pools.iter()
                                .map(|(addr, pool)| (addr.clone(), pool.denom_reserve, pool.token_reserve))
                                .collect();
                            
                            cache.update_token_data(
                                token_address,
                                &token_info.creator_address,
                                &token_info.current_owner,
                                pools,
                                token_info.trading_enabled,
                                token_info.is_scam,
                            ).await;
                            
                            processed += 1;
                        }
                        
                        info!("✅ Processed {} tokens from update", processed);
                        
                        // Show cache stats
                        let stats = cache.get_stats().await;
                        info!("📈 Cache now has: {} addresses, {} tokens, {} pools", 
                            stats.tracked_addresses, stats.tracked_tokens, stats.tracked_pools);
                    }
                    Err(e) => {
                        warn!("Failed to parse message as TokenUpdatesMessage: {}", e);
                        debug!("Raw message: {}", msg_str.chars().take(200).collect::<String>());
                    }
                }
            }
            Ok(Err(zmq_err)) => {
                error!("ZMQ string conversion error: {:?}", zmq_err);
            }
            Err(e) => {
                error!("ZMQ recv error: {:?}", e);
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        }
    }
    
    info!("✅ Processed {} messages", message_count);
    Ok(())
}