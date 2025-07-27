// examples/test_token_cache.rs
//
// Simple test of token tracking cache functionality

use mempool_processor::token_tracking::AddressTrackingCache;
use tracing::{info, error};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    info!("🚀 Testing token tracking cache");
    
    // Create cache
    let cache = AddressTrackingCache::new();
    
    // Add sample token data
    let creator = "0x1234567890123456789012345678901234567890";
    let owner = "0xabcdefabcdefabcdefabcdefabcdefabcdefabcd";
    let token = "0xfedcbafedcbafedcbafedcbafedcbafedcbafedcba";
    let pool = "0x9876543210987654321098765432109876543210";
    
    info!("📊 Adding sample token data");
    cache.update_token_data(
        token,
        creator,
        owner,
        vec![(pool.to_string(), 10.5, 1000000.0)],
        true,
        false,
    ).await;
    
    // Test creator lookup
    info!("🔍 Testing creator lookup");
    if let Some(creator_info) = cache.get_address_info(creator).await {
        info!("✅ Found creator with {} tokens", creator_info.tokens.len());
    } else {
        error!("❌ Creator not found");
    }
    
    // Test owner lookup
    info!("🔍 Testing owner lookup");
    if let Some(owner_info) = cache.get_address_info(owner).await {
        info!("✅ Found owner with {} tokens", owner_info.tokens.len());
    } else {
        error!("❌ Owner not found");
    }
    
    // Test pool lookup
    info!("🏊 Testing pool lookup");
    if let Some((token_addr, pool_info)) = cache.get_pool_info(pool).await {
        info!("✅ Found pool for token {}", token_addr);
        info!("  ETH Reserve: {}", pool_info.current_eth_reserve);
    } else {
        error!("❌ Pool not found");
    }
    
    // Test function recording
    info!("📞 Testing function recording");
    cache.record_function_call(
        creator,
        [0xba, 0xa2, 0xab, 0xde],
        "removeLiquidity".to_string(),
        "0x123".to_string(),
        1700000000,
    ).await;
    
    // Check stats
    let stats = cache.get_stats().await;
    info!("📈 Final stats: {} addresses, {} tokens, {} pools", 
        stats.tracked_addresses, stats.tracked_tokens, stats.tracked_pools);
    
    info!("✅ All tests passed!");
    
    Ok(())
}