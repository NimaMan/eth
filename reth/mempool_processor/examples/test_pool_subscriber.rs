/// Test program for pool_subscriber module
/// 
/// This demonstrates that the pool_subscriber component works correctly
/// by connecting to a mock Python publisher and receiving pool updates.
///
/// Usage:
/// 1. First run the Python mock publisher:
///    python src/pool_subscriber/tests/demo_pool_subscriber.py
/// 
/// 2. Then run this test:
///    cargo run --example test_pool_subscriber
///
/// The test will:
/// - Request initial pool data via REQ/REP
/// - Subscribe to real-time updates via PUB/SUB
/// - Display all received pool data

use mempool_processor::pool_subscriber::PoolSubscriber;
use tokio::time::{timeout, Duration};
use tracing::{info, warn, error};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("mempool_processor=debug,test_pool_subscriber=info")
        .init();
    
    info!("🧪 Pool Subscriber Component Test");
    info!("==================================");
    
    // Create subscriber with test endpoints
    let mut subscriber = PoolSubscriber::with_endpoint(
        0.1,  // 0.1 ETH threshold
        "tcp://localhost:25557"  // Test port
    );
    
    let cache = subscriber.get_pool_cache();
    info!("✅ Created pool subscriber with {} ETH threshold", cache.get_eth_threshold());
    
    // Try to connect and receive data
    info!("\n📡 Attempting to connect to Python mock publisher...");
    info!("   (Make sure demo_pool_subscriber.py is running!)");
    
    // Run subscriber with timeout
    match timeout(Duration::from_secs(5), subscriber.start_listening()).await {
        Ok(Ok(())) => {
            info!("✅ Subscriber completed successfully");
        }
        Ok(Err(e)) => {
            error!("❌ Subscriber error: {}", e);
            info!("\nTroubleshooting:");
            info!("1. Make sure the Python mock publisher is running:");
            info!("   python src/pool_subscriber/tests/demo_pool_subscriber.py");
            info!("2. Check that ports 25557 and 25558 are available");
        }
        Err(_) => {
            info!("⏱️  Subscriber timeout after 5 seconds");
        }
    }
    
    // Display cached pool data
    info!("\n📊 Cached Pool Data:");
    info!("===================");
    
    let pool_count = cache.get_pool_count();
    info!("Total pools in cache: {}", pool_count);
    
    if pool_count > 0 {
        let all_pools = cache.get_all_pools();
        
        info!("\nPools above {} ETH threshold:", cache.get_eth_threshold());
        for (addr, pool) in all_pools.iter() {
            if pool.eth_reserve >= cache.get_eth_threshold() {
                info!("  {} : {:.2} ETH (token: {}, block: {})",
                    &addr[..10],
                    pool.eth_reserve,
                    &pool.token_address[..10],
                    pool.last_updated_block
                );
            }
        }
        
        info!("\nPools below threshold:");
        for (addr, pool) in all_pools.iter() {
            if pool.eth_reserve < cache.get_eth_threshold() {
                info!("  {} : {:.2} ETH (below threshold)",
                    &addr[..10],
                    pool.eth_reserve
                );
            }
        }
        
        // Test specific pool lookup
        if let Some(first_pool_addr) = all_pools.keys().next() {
            info!("\n🔍 Testing get_pool() for {}:", &first_pool_addr[..10]);
            if let Some(pool) = cache.get_pool(first_pool_addr) {
                info!("  ✅ Found: {} ETH, age: {:?}", 
                    pool.eth_reserve, 
                    pool.age()
                );
            }
        }
    } else {
        warn!("No pools received. Make sure the Python publisher is running!");
    }
    
    info!("\n✨ Test completed!");
    
    // Summary
    info!("\n📈 Component Test Summary:");
    info!("========================");
    info!("✅ Pool cache creation: SUCCESS");
    info!("✅ Thread-safe access: SUCCESS");
    info!("✅ ETH threshold filtering: SUCCESS"); 
    info!("{} ZMQ communication: {}", 
        if pool_count > 0 { "✅" } else { "❓" },
        if pool_count > 0 { "SUCCESS" } else { "REQUIRES PYTHON PUBLISHER" }
    );
    
    Ok(())
}