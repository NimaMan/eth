/// Test if specific pools are in our cache using different address formats

use mempool_processor::pool_subscriber::PoolSubscriber;
use mempool_processor::common::address::{checksum_address, alloy_address_to_checksum};
use alloy_primitives::Address;
use std::str::FromStr;
use tokio::time::Duration;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_target(false)
        .init();
    
    // Initialize pool subscriber
    info!("Initializing pool subscriber...");
    let mut pool_subscriber = PoolSubscriber::with_endpoint(0.01, "tcp://localhost:5557");
    let pool_cache = pool_subscriber.get_pool_cache();
    
    // Start listening in background
    let pool_cache_clone = pool_cache.clone();
    tokio::spawn(async move {
        if let Err(e) = pool_subscriber.start_listening().await {
            warn!("Pool subscriber error: {}", e);
        }
    });
    
    // Wait for initialization
    tokio::time::sleep(Duration::from_secs(3)).await;
    
    let pool_count = pool_cache_clone.get_pool_count();
    info!("Monitoring {} pools", pool_count);
    
    // Test pools from liquidity removal transactions
    let test_pools = vec![
        "0xd46f41239bd02760ea40698f0787a668ea631b48",
        "0x6ce3a16d1dd783addce2a140c5a9e6b92b84a1a3",
    ];
    
    info!("\nTesting pool lookups with different formats:");
    
    for pool_addr in test_pools {
        info!("\n{}", "=".repeat(60));
        info!("Testing pool: {}", pool_addr);
        
        // Try lowercase
        let lowercase = pool_addr.to_lowercase();
        if let Some(pool) = pool_cache_clone.get_pool(&lowercase) {
            info!("✅ Found with lowercase: {}", lowercase);
            info!("   ETH reserve: {:.6}", pool.eth_reserve);
            info!("   Token reserve: {:.6}", pool.token_reserve);
        } else {
            info!("❌ Not found with lowercase: {}", lowercase);
        }
        
        // Try checksummed
        let checksummed = checksum_address(pool_addr);
        if let Some(pool) = pool_cache_clone.get_pool(&checksummed) {
            info!("✅ Found with checksum: {}", checksummed);
            info!("   ETH reserve: {:.6}", pool.eth_reserve);
            info!("   Token reserve: {:.6}", pool.token_reserve);
        } else {
            info!("❌ Not found with checksum: {}", checksummed);
        }
        
        // Try original
        if let Some(pool) = pool_cache_clone.get_pool(pool_addr) {
            info!("✅ Found with original: {}", pool_addr);
            info!("   ETH reserve: {:.6}", pool.eth_reserve);
            info!("   Token reserve: {:.6}", pool.token_reserve);
        } else {
            info!("❌ Not found with original: {}", pool_addr);
        }
    }
    
    // Sample some pools from cache to see their format
    info!("\n{}", "=".repeat(60));
    info!("Sampling 5 pools from cache to check address format:");
    let all_pools = pool_cache_clone.get_all_pools();
    for (i, (addr, pool)) in all_pools.iter().take(5).enumerate() {
        info!("{}. {} (ETH: {:.6})", i + 1, addr, pool.eth_reserve);
        
        // Check if it's checksummed
        let expected_checksum = checksum_address(addr);
        if addr == &expected_checksum {
            info!("   ✓ Is checksummed");
        } else {
            info!("   ✗ Not checksummed (should be: {})", expected_checksum);
        }
    }
    
    Ok(())
}