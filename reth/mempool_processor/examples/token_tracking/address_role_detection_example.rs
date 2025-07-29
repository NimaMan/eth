// examples/token_tracking/cache_operations.rs
//
// Example showing cache operations and address lookup functionality
// Tests the core functionality needed for signal detection

use mempool_processor::token_tracking::{AddressTrackingCache, AddressRole};
use tracing::{info, error};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    info!("🚀 Starting cache operations test");
    
    // Create cache
    let cache = AddressTrackingCache::new();
    
    // Test 1: Add sample token data
    info!("📊 Test 1: Adding sample token data");
    
    let creator_address = "0x1234567890123456789012345678901234567890";
    let owner_address = "0xabcdefabcdefabcdefabcdefabcdefabcdefabcd";
    let token_address = "0xfedcbafedcbafedcbafedcbafedcbafedcbafedcba";
    let pool_address = "0x9876543210987654321098765432109876543210";
    
    // Add token with separate creator and owner
    cache.update_token_data(
        token_address,
        creator_address,
        owner_address,
        vec![(pool_address.to_string(), 10.5, 1000000.0)],
        true,
        false,
    ).await;
    
    info!("✅ Added token {} with creator {} and owner {}", 
        token_address, creator_address, owner_address);
    
    // Test 2: Address lookups
    info!("🔍 Test 2: Testing address lookups");
    
    // Check if creator is tracked
    if let Some(creator_info) = cache.get_address_info(creator_address).await {
        info!("✅ Found creator {} with {} tokens", 
            creator_address, creator_info.tokens.len());
        
        for (token, role) in &creator_info.tokens {
            info!("  Token: {} - Role: {:?}", token, role);
        }
    } else {
        error!("❌ Creator not found in cache");
    }
    
    // Check if owner is tracked
    if let Some(owner_info) = cache.get_address_info(owner_address).await {
        info!("✅ Found owner {} with {} tokens", 
            owner_address, owner_info.tokens.len());
        
        for (token, role) in &owner_info.tokens {
            info!("  Token: {} - Role: {:?}", token, role);
        }
    } else {
        error!("❌ Owner not found in cache");
    }
    
    // Test 3: Function call tracking
    info!("📞 Test 3: Testing function call tracking");
    
    // Simulate function calls from creator
    let function_calls = vec![
        ("transfer", [0xa9, 0x05, 0x9c, 0xbb]),
        ("approve", [0x09, 0x5e, 0xa7, 0xb3]),
        ("removeLiquidity", [0xba, 0xa2, 0xab, 0xde]),
    ];
    
    for (i, (func_name, selector)) in function_calls.iter().enumerate() {
        cache.record_function_call(
            creator_address,
            *selector,
            func_name.to_string(),
            format!("0x{:064x}", i),
            1700000000 + i as u64,
        ).await;
        
        info!("📝 Recorded function call: {} from {}", func_name, creator_address);
    }
    
    // Check function history
    if let Some(creator_info) = cache.get_address_info(creator_address).await {
        info!("📊 Creator function history ({} calls):", creator_info.function_history.len());
        for call in &creator_info.function_history {
            info!("  {} - {} ({})", 
                call.function_name, 
                call.tx_hash, 
                call.timestamp);
        }
    }
    
    // Test 4: Pool lookups
    info!("🏊 Test 4: Testing pool lookups");
    
    if let Some((token, pool_info)) = cache.get_pool_info(pool_address).await {
        info!("✅ Found pool {} for token {}", pool_address, token);
        info!("  ETH Reserve: {}", pool_info.current_eth_reserve);
        info!("  Token Reserve: {}", pool_info.current_token_reserve);
        info!("  Pool Type: {}", pool_info.pool_type);
    } else {
        error!("❌ Pool not found in cache");
    }
    
    // Test 5: Risk marking
    info!("⚠️  Test 5: Testing risk marking");
    
    cache.mark_address_high_risk(creator_address, "Suspicious removeLiquidity pattern").await;
    
    if cache.is_high_risk_address(creator_address).await {
        info!("✅ Creator marked as high risk");
    } else {
        error!("❌ Risk marking failed");
    }
    
    // Test 6: Cache statistics
    info!("📈 Test 6: Cache statistics");
    
    let stats = cache.get_stats().await;
    info!("Cache Statistics:");
    info!("  Tracked Addresses: {}", stats.tracked_addresses);
    info!("  Tracked Tokens: {}", stats.tracked_tokens);
    info!("  Tracked Pools: {}", stats.tracked_pools);
    info!("  High Risk Addresses: {}", stats.high_risk_addresses);
    
    // Test 7: Address token relationships
    info!("🔗 Test 7: Testing address-token relationships");
    
    let creator_tokens = cache.get_address_tokens(creator_address).await;
    info!("Creator {} is associated with {} tokens:", creator_address, creator_tokens.len());
    for (token, role) in &creator_tokens {
        info!("  {} - {:?}", token, role);
    }
    
    let owner_tokens = cache.get_address_tokens(owner_address).await;
    info!("Owner {} is associated with {} tokens:", owner_address, owner_tokens.len());
    for (token, role) in &owner_tokens {
        info!("  {} - {:?}", token, role);
    }
    
    info!("✅ All cache operation tests completed successfully");
    
    Ok(())
}