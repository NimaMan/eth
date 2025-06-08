// Live pool detection integration test
// This test verifies that our pool detection works correctly by:
// 1. Subscribing to Python pool updates via ZMQ
// 2. Monitoring mempool transactions
// 3. Detecting when transactions interact with known pools

use mempool_processor::{
    pool_subscriber::PoolSubscriber,
    mempool_processor::fetcher::{MempoolFetcher, TransactionSource},
    common::address::to_checksum_address,
};

use std::time::{Duration, Instant};
use std::collections::{HashMap, HashSet};
use tokio::time::sleep;
use tracing::{info, debug, warn};
use revm_primitives::Address;

#[derive(Debug, Clone)]
struct PoolInteraction {
    tx_hash: String,
    pool_address: String,
    token_address: String,
    eth_reserve: f64,
    interaction_type: String,
    timestamp: Instant,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("🚀 Starting live pool detection test");
    info!("This test will run for 60 seconds and monitor pool interactions");
    
    // Configuration
    let test_duration = Duration::from_secs(60); // Run for 1 minute
    let zmq_endpoint = "tcp://localhost:5557"; // Python pool publisher endpoint
    let rpc_url = "http://localhost:8545";
    let eth_threshold = 0.05; // ETH threshold for pool detection
    
    // Initialize pool subscriber
    let mut pool_subscriber = PoolSubscriber::with_endpoint(eth_threshold, zmq_endpoint);
    let pool_cache = pool_subscriber.get_pool_cache();
    
    // Start pool subscriber in background
    let subscriber_handle = {
        tokio::spawn(async move {
            info!("📡 Starting pool subscriber on {}", zmq_endpoint);
            match pool_subscriber.start_listening().await {
                Ok(_) => info!("Pool subscriber completed"),
                Err(e) => warn!("Pool subscriber error: {}", e),
            }
        })
    };
    
    // Give subscriber time to connect and receive initial pool data
    info!("⏳ Waiting 5 seconds for initial pool data...");
    sleep(Duration::from_secs(5)).await;
    
    // Check initial pool count
    let initial_pool_count = pool_cache.get_pool_count();
    info!("📊 Initial pool count: {}", initial_pool_count);
    
    if initial_pool_count == 0 {
        warn!("⚠️  No pools received from Python. Make sure the Python pool publisher is running!");
        warn!("   Run: python3 /path/to/pool_publisher.py");
    }
    
    // Initialize mempool fetcher
    let mempool_fetcher = MempoolFetcher::new(rpc_url)?;
    
    // Track statistics
    let mut total_transactions = 0;
    let mut pool_interactions = Vec::new();
    let mut unique_pools_seen = HashSet::new();
    let mut pool_tx_count = HashMap::new();
    
    // Start monitoring
    let start_time = Instant::now();
    info!("🔍 Starting transaction monitoring...");
    
    while start_time.elapsed() < test_duration {
        // Get current mempool transactions
        match mempool_fetcher.get_transactions().await {
            Ok(transactions) => {
                for tx in transactions {
                    total_transactions += 1;
                    
                    // Check if transaction interacts with any known pool
                    let to_address = if let Some(to) = tx.to {
                        // Convert to checksummed address
                        let addr_bytes = to.as_ref();
                        let mut addr_array = [0u8; 20];
                        addr_array.copy_from_slice(addr_bytes);
                        let addr = Address::from(addr_array);
                        to_checksum_address(addr)
                    } else {
                        continue; // Skip contract creation transactions
                    };
                    
                    // Check if this address is a known pool
                    if let Some(pool_state) = pool_cache.get_pool(&to_address) {
                        let interaction = PoolInteraction {
                            tx_hash: format!("0x{}", hex::encode(&tx.hash)),
                            pool_address: to_address.clone(),
                            token_address: pool_state.token_address.clone(),
                            eth_reserve: pool_state.eth_reserve,
                            interaction_type: "Direct Transfer".to_string(),
                            timestamp: Instant::now(),
                        };
                        
                        info!("🎯 Pool interaction detected!");
                        info!("   Transaction: {}", interaction.tx_hash);
                        info!("   Pool: {}", interaction.pool_address);
                        info!("   Token: {}", interaction.token_address);
                        info!("   ETH Reserve: {:.6} ETH", interaction.eth_reserve);
                        
                        unique_pools_seen.insert(to_address.clone());
                        *pool_tx_count.entry(to_address.clone()).or_insert(0) += 1;
                        pool_interactions.push(interaction);
                    }
                    
                    // Also check transaction input data for pool interactions
                    if let Some(input_data) = &tx.input_data {
                        if input_data.len() >= 4 {
                            // Check for common DEX function selectors
                            let selector = &input_data[0..4];
                            let function_name = match selector {
                            [0x7f, 0xf3, 0x6a, 0xb5] => Some("swapExactTokensForTokens"),
                            [0x38, 0xed, 0x17, 0x39] => Some("swapExactTokensForETH"),
                            [0x18, 0xcb, 0xaf, 0xe5] => Some("swapExactETHForTokens"),
                            [0xe8, 0xe3, 0x37, 0x00] => Some("addLiquidity"),
                            [0xf3, 0x05, 0xd7, 0x19] => Some("addLiquidityETH"),
                            [0xba, 0xa2, 0xab, 0xde] => Some("removeLiquidity"),
                            [0x02, 0x75, 0x1c, 0xec] => Some("removeLiquidityETH"),
                            _ => None,
                        };
                        
                            if let Some(func_name) = function_name {
                                debug!("   📝 DEX function: {}", func_name);
                            }
                        }
                    }
                }
            }
            Err(e) => {
                debug!("Error fetching transactions: {}", e);
            }
        }
        
        // Update pool count periodically
        let current_pool_count = pool_cache.get_pool_count();
        if current_pool_count != initial_pool_count {
            info!("📈 Pool count updated: {} -> {}", initial_pool_count, current_pool_count);
        }
        
        // Status update every 10 seconds
        let elapsed = start_time.elapsed();
        if elapsed.as_secs() % 10 == 0 && elapsed.as_millis() % 1000 < 100 {
            info!("⏱️  Status: {}s elapsed, {} transactions seen, {} pool interactions", 
                elapsed.as_secs(), total_transactions, pool_interactions.len());
        }
        
        // Small delay to avoid overwhelming the RPC
        sleep(Duration::from_millis(100)).await;
    }
    
    // Final statistics
    info!("");
    info!("{}", "=".repeat(80));
    info!("📊 LIVE POOL DETECTION TEST RESULTS");
    info!("{}", "=".repeat(80));
    info!("Test Duration: {} seconds", test_duration.as_secs());
    info!("Total Transactions Monitored: {}", total_transactions);
    info!("Pool Interactions Detected: {}", pool_interactions.len());
    info!("Unique Pools Involved: {}", unique_pools_seen.len());
    info!("Total Pools in Cache: {}", pool_cache.get_pool_count());
    
    if !pool_interactions.is_empty() {
        info!("");
        info!("🎯 Pool Interaction Summary:");
        info!("{}", "-".repeat(80));
        
        // Show most active pools
        let mut pool_activity: Vec<_> = pool_tx_count.iter().collect();
        pool_activity.sort_by(|a, b| b.1.cmp(a.1));
        
        info!("\nMost Active Pools:");
        for (pool_addr, count) in pool_activity.iter().take(10) {
            if let Some(pool_state) = pool_cache.get_pool(pool_addr) {
                info!("  {} ({} transactions)", pool_addr, count);
                info!("    Token: {}", pool_state.token_address);
                info!("    ETH Reserve: {:.6} ETH", pool_state.eth_reserve);
            }
        }
        
        // Show recent interactions
        info!("\nRecent Pool Interactions (last 10):");
        for interaction in pool_interactions.iter().rev().take(10) {
            info!("  TX: {} -> Pool: {}", 
                &interaction.tx_hash[..10], 
                &interaction.pool_address[..10]);
        }
        
        info!("\n✅ POOL DETECTION IS WORKING!");
        info!("   Successfully detected {} transactions interacting with {} known pools",
            pool_interactions.len(), unique_pools_seen.len());
    } else {
        info!("\n⚠️  No pool interactions detected during the test period.");
        info!("   This could mean:");
        info!("   1. The mempool is quiet (try running during active trading hours)");
        info!("   2. The Python pool publisher is not running");
        info!("   3. The address format fix hasn't been applied correctly");
        
        // Diagnostic information
        if pool_cache.get_pool_count() == 0 {
            info!("\n❌ CRITICAL: No pools in cache! Python publisher is likely not running.");
        } else {
            info!("\n📊 Diagnostic: {} pools in cache, but no matching transactions found.", 
                pool_cache.get_pool_count());
            
            // Show a few pool addresses for debugging
            let all_pools = pool_cache.get_all_pools();
            info!("Sample pool addresses in cache:");
            for (addr, _) in all_pools.iter().take(5) {
                info!("  {}", addr);
            }
        }
    }
    
    // Cleanup
    subscriber_handle.abort();
    
    info!("\n🏁 Test completed!");
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use mempool_processor::pool_subscriber::cache::PoolStateCache;
    
    #[test]
    fn test_pool_detection_setup() {
        // Basic test to ensure the module compiles and types are correct
        let cache = PoolStateCache::new(0.05);
        assert_eq!(cache.get_pool_count(), 0);
        assert_eq!(cache.get_eth_threshold(), 0.05);
    }
}