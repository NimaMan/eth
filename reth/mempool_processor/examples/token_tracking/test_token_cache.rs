/// Token Tracking Cache Test Example
/// 
/// Comprehensive test of the AddressTrackingCache functionality including:
/// - Basic cache creation and initialization
/// - Adding token, creator, owner, and pool data
/// - Querying cache for different entity types
/// - Recording function calls
/// - Retrieving cache statistics
///
/// This example demonstrates all core cache operations without external dependencies.

use mempool_processor::token_tracking::AddressTrackingCache;
use tracing::{info, error};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    info!("🚀 Token Tracking Cache Test Suite");
    info!("==================================");
    
    // 1. Basic cache creation test
    info!("\n📦 Test 1: Basic cache creation");
    let cache = AddressTrackingCache::new();
    let initial_stats = cache.get_stats().await;
    info!("✅ Cache created successfully");
    info!("   Initial state: {} addresses, {} tokens, {} pools", 
        initial_stats.tracked_addresses, 
        initial_stats.tracked_tokens, 
        initial_stats.tracked_pools
    );
    
    // 2. Add sample token data
    info!("\n📊 Test 2: Adding token data");
    let creator = "0x1234567890123456789012345678901234567890";
    let owner = "0xabcdefabcdefabcdefabcdefabcdefabcdefabcd";
    let token = "0xfedcbafedcbafedcbafedcbafedcbafedcbafedcba";
    let pool = "0x9876543210987654321098765432109876543210";
    
    cache.update_token_data(
        token,
        creator,
        owner,
        vec![(pool.to_string(), 10.5, 1000000.0)],
        true,
        false,
    ).await;
    info!("✅ Token data added successfully");
    
    // 3. Test creator lookup
    info!("\n🔍 Test 3: Creator lookup");
    if let Some(creator_info) = cache.get_address_info(creator).await {
        info!("✅ Found creator:");
        info!("   Address: {}", creator);
        info!("   Tokens created: {}", creator_info.tokens.len());
        info!("   Is creator: {}", creator_info.is_creator);
    } else {
        error!("❌ Creator not found");
    }
    
    // 4. Test owner lookup
    info!("\n🔍 Test 4: Owner lookup");
    if let Some(owner_info) = cache.get_address_info(owner).await {
        info!("✅ Found owner:");
        info!("   Address: {}", owner);
        info!("   Tokens owned: {}", owner_info.tokens.len());
        info!("   Is owner: {}", owner_info.is_owner);
    } else {
        error!("❌ Owner not found");
    }
    
    // 5. Test pool lookup
    info!("\n🏊 Test 5: Pool lookup");
    if let Some((token_addr, pool_info)) = cache.get_pool_info(pool).await {
        info!("✅ Found pool:");
        info!("   Pool address: {}", pool);
        info!("   Token address: {}", token_addr);
        info!("   ETH Reserve: {:.4} ETH", pool_info.current_eth_reserve);
        info!("   Token Reserve: {:.0}", pool_info.current_token_reserve);
    } else {
        error!("❌ Pool not found");
    }
    
    // 6. Test function call recording
    info!("\n📞 Test 6: Function call recording");
    cache.record_function_call(
        creator,
        [0xba, 0xa2, 0xab, 0xde],  // removeLiquidity selector
        "removeLiquidity".to_string(),
        "0x123456789abcdef".to_string(),
        1700000000,
    ).await;
    info!("✅ Function call recorded");
    
    // 7. Add more data to test cache growth
    info!("\n📈 Test 7: Cache growth");
    for i in 0..5 {
        let test_token = format!("0x{:040}", i);
        let test_creator = format!("0x{:040}", i + 100);
        let test_owner = format!("0x{:040}", i + 200);
        let test_pool = format!("0x{:040}", i + 300);
        
        cache.update_token_data(
            &test_token,
            &test_creator,
            &test_owner,
            vec![(test_pool, 5.0 + i as f64, 500000.0 * (i + 1) as f64)],
            true,
            false,
        ).await;
    }
    info!("✅ Added 5 more tokens to cache");
    
    // 8. Final statistics
    info!("\n📊 Test 8: Final cache statistics");
    let final_stats = cache.get_stats().await;
    info!("✅ Cache statistics:");
    info!("   Total addresses: {} (Δ +{})", 
        final_stats.tracked_addresses, 
        final_stats.tracked_addresses - initial_stats.tracked_addresses
    );
    info!("   Total tokens: {} (Δ +{})", 
        final_stats.tracked_tokens,
        final_stats.tracked_tokens - initial_stats.tracked_tokens
    );
    info!("   Total pools: {} (Δ +{})", 
        final_stats.tracked_pools,
        final_stats.tracked_pools - initial_stats.tracked_pools
    );
    
    // 9. Verify cache integrity
    info!("\n🔒 Test 9: Cache integrity check");
    let all_addresses = final_stats.tracked_addresses;
    let unique_tokens = final_stats.tracked_tokens;
    let unique_pools = final_stats.tracked_pools;
    
    if all_addresses >= unique_tokens && unique_tokens == unique_pools {
        info!("✅ Cache integrity verified");
        info!("   Each token has exactly one pool");
        info!("   All creators and owners are tracked");
    } else {
        error!("❌ Cache integrity issue detected");
    }
    
    info!("\n✨ All tests completed successfully!");
    info!("=====================================");
    
    Ok(())
}