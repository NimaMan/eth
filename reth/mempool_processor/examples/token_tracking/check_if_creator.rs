use mempool_processor::token_tracking::{CacheConfig, TokenTrackingCache};
use std::sync::Arc;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Create cache config with simple defaults
    let config = CacheConfig {
        max_tokens: 10000,
        max_pools: 50000,
        eth_threshold: 100.0,
        evict_scam_first: true,
    };

    // Create token tracking cache
    let cache = Arc::new(TokenTrackingCache::new(config));

    // The address we want to check (from the LP approval transaction)
    let address_to_check = "0xcb1534B135450ac5433BE4c5f5eC61a781b05c35".to_string();

    println!("=== Checking Address: {} ===", address_to_check);

    // Check if it's a creator
    let is_creator = cache.is_creator(&address_to_check).await;
    println!("Is creator in cache: {}", is_creator);

    if is_creator {
        println!("✓ Address IS marked as a creator in cache");
        if let Some(token_info) = cache.get_token_for_creator(&address_to_check).await {
            println!("Creator of token: {}", token_info.address);
            println!("Token name: {}", token_info.name);
            println!("Token symbol: {}", token_info.symbol);
        }
    } else {
        println!("✗ Address is NOT marked as a creator in cache");
    }

    // Check the LP pair
    let lp_pair = "0x4547B34C4031393E1695E697675F6CEc17a2950E".to_string();
    println!("\n=== Checking LP Pair: {} ===", lp_pair);

    if cache.is_pool(&lp_pair).await {
        println!("✓ LP pair IS in cache as a pool");
        if let Some(pool_info) = cache.get_pool(&lp_pair).await {
            println!("Token address: {}", pool_info.token_address);
            println!("Pool type: {:?}", pool_info.pool_type);
        }
    } else {
        println!("✗ LP pair is NOT in cache");
    }

    // Check cache stats
    println!("\n=== Cache Status ===");
    let stats = cache.stats().await;
    println!("Total tokens: {}", stats.total_tokens);
    println!("Total pools: {}", stats.total_pools);
    println!("Total creators: {}", stats.total_creators);

    if stats.total_tokens == 0 {
        println!("\n⚠️ IMPORTANT: Cache is empty!");
        println!("The cache is only populated when:");
        println!("1. The signal detector is running and processing transactions");
        println!("2. The Python publisher sends updates via ZMQ");
        println!("\nSince the cache is empty, we CANNOT determine from cache alone whether");
        println!("address {} is a creator or not.", address_to_check);
        println!("\nTo verify this properly, we would need to:");
        println!("1. Run the signal detector with a populated database");
        println!("2. Or check the database directly");
        println!("3. Or trace on-chain to see if this address deployed any tokens");
    }

    Ok(())
}
