/// Pool Detection Monitor - Connects to Live Python Publisher
/// 
/// Monitors pool updates from the live Python token processing system
/// to verify the pool detection and publishing pipeline is working.
///
/// Usage:
///    cargo run --example pool_detection_monitor --release
///
/// Connects to:
/// - REQ/REP on tcp://localhost:5558 for initial pool data
/// - PUB/SUB on tcp://localhost:5557 for real-time updates
///
/// Logs all pool details to: /home/nima/code/crypto/logs/mempool/pool_detection_YYYYMMDD_HHMMSS.log

use mempool_processor::pool_subscriber::PoolSubscriber;
use tokio::time::{interval, Duration};
use tracing::{info, warn, error};
use tracing_subscriber;
use std::fs::File;
use std::io::Write;
use chrono::Local;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_target(false)
        .with_thread_ids(false)
        .with_level(true)
        .init();
    
    // Create log file
    let log_dir = "/home/nima/code/crypto/logs/mempool";
    std::fs::create_dir_all(log_dir)?;
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let log_path = format!("{}/pool_detection_{}.log", log_dir, timestamp);
    let mut log_file = File::create(&log_path)?;
    
    info!("Pool Detection Monitor");
    info!("=====================");
    info!("Log file: {}", log_path);
    writeln!(log_file, "timestamp,pool_address,token_address,eth_reserve,token_reserve,last_updated_block,age_seconds")?;
    
    // Create subscriber for live system
    let mut subscriber = PoolSubscriber::with_endpoint(
        0.01,  // 0.01 ETH threshold to see more pools
        "tcp://localhost:5557"  // Live publisher port
    );
    
    let cache = subscriber.get_pool_cache();
    info!("Created pool subscriber with {} ETH threshold", cache.get_eth_threshold());
    
    // Start subscriber in background
    info!("\nConnecting to live Python publisher...");
    info!("REQ endpoint: tcp://localhost:5558");
    info!("PUB endpoint: tcp://localhost:5557");
    
    // Start subscriber in background
    let subscriber_handle = tokio::spawn(async move {
        if let Err(e) = subscriber.start_listening().await {
            error!("Subscriber error: {}", e);
        }
    });
    
    // Monitor pools periodically
    let mut ticker = interval(Duration::from_secs(5));
    let mut last_pool_count = 0;
    let mut pool_history: HashMap<String, (f64, u64)> = HashMap::new();  // (eth_reserve, last_block)
    
    info!("\nMonitoring pool updates... Press Ctrl+C to stop");
    
    loop {
        ticker.tick().await;
        
        let pool_count = cache.get_pool_count();
        
        if pool_count != last_pool_count {
            info!("\nPool count changed: {} -> {}", last_pool_count, pool_count);
            last_pool_count = pool_count;
        }
        
        if pool_count > 0 {
            let all_pools = cache.get_all_pools();
            let now = Local::now();
            
            // Log new or updated pools
            for (addr, pool) in all_pools.iter() {
                let age = pool.age().as_secs();
                
                // Check if this is a new pool or has been updated
                let is_new_or_updated = match pool_history.get(addr) {
                    None => true,
                    Some((old_eth, old_block)) => {
                        pool.eth_reserve != *old_eth || pool.last_updated_block > *old_block
                    }
                };
                
                if is_new_or_updated {
                    // Log to file
                    writeln!(
                        log_file,
                        "{},{},{},{:.6},{:.6},{},{}",
                        now.format("%Y-%m-%d %H:%M:%S%.3f"),
                        addr,
                        pool.token_address,
                        pool.eth_reserve,
                        pool.token_reserve,
                        pool.last_updated_block,
                        age
                    )?;
                    log_file.flush()?;
                    
                    // Log to console
                    if pool_history.contains_key(addr) {
                        info!(
                            "UPDATED: {} ETH: {:.4} -> {:.4}, Token: {:.2}, Block: {}",
                            addr,
                            pool_history.get(addr).unwrap().0,
                            pool.eth_reserve,
                            pool.token_reserve,
                            pool.last_updated_block
                        );
                    } else {
                        info!(
                            "NEW POOL: {} ETH: {:.4}, Token: {:.2}, Block: {}",
                            addr,
                            pool.eth_reserve,
                            pool.token_reserve,
                            pool.last_updated_block
                        );
                    }
                    
                    // Update history
                    pool_history.insert(addr.clone(), (pool.eth_reserve, pool.last_updated_block));
                }
            }
            
            // Periodic summary
            if pool_count > 0 && pool_count % 10 == 0 {
                info!("\n=== SUMMARY ===");
                info!("Total pools tracked: {}", pool_count);
                
                let above_threshold = all_pools.values()
                    .filter(|p| p.eth_reserve >= cache.get_eth_threshold())
                    .count();
                let below_threshold = pool_count - above_threshold;
                
                info!("Above {} ETH: {}", cache.get_eth_threshold(), above_threshold);
                info!("Below threshold: {}", below_threshold);
                
                // Find pool with most ETH
                if let Some((addr, pool)) = all_pools.iter()
                    .max_by(|a, b| a.1.eth_reserve.partial_cmp(&b.1.eth_reserve).unwrap()) {
                    info!("Largest pool: {} with {:.4} ETH", addr, pool.eth_reserve);
                }
            }
        } else if pool_count == 0 && ticker.period().as_secs() % 30 == 0 {
            warn!("No pools received. Check if Python publisher is running on ports 5557/5558");
        }
    }
}