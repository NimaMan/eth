// examples/test_cache_basic.rs
//
// Basic compilation test for token tracking cache

use mempool_processor::token_tracking::address_tracking_cache::AddressTrackingCache;

#[tokio::main]
async fn main() {
    println!("🚀 Testing basic cache creation");
    
    let cache = AddressTrackingCache::new();
    let stats = cache.get_stats().await;
    
    println!("✅ Cache created with {} addresses", stats.tracked_addresses);
    println!("✅ Compilation successful!");
}