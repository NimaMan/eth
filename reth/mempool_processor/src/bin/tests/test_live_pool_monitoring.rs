/*
 * Live Pool Monitoring Test
 * 
 * This test connects to the live pool cache via ZeroMQ, logs current pool states,
 * monitors the latest 20 blocks for pool changes, and verifies that our scam
 * detection system correctly identifies and logs pool depletion events.
 * 
 * Algorithm:
 * 1. Connect to ZeroMQ pool updates from Python service
 * 2. Log initial pool states and statistics
 * 3. Monitor incoming pool updates for 20 blocks
 * 4. Track pool state changes and ETH reserve deltas
 * 5. Test scam detection on pools that show significant depletion
 * 6. Verify database logging works for detected scams
 * 7. Report statistics on pools monitored and changes detected
 */

use clap::Parser;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time;
use tracing::{info, error, warn, debug, Level};
use std::collections::HashMap;
use ethers::types::H256;
use serde_json;

use mempool_processor::mempool_processor::db_logger::DbLogger;
use mempool_processor::pool_subscriber::{cache::PoolStateCache, types::{PoolUpdate, PoolUpdatesMessage}};
use mempool_processor::scam_detection::{
    ScamDetectionService, 
    SimulationResult, 
    PoolEffect,
    ScamDetectionConfig
};

#[derive(Parser, Debug)]
struct Args {
    /// Database hostname
    #[arg(long, env = "DB_HOST", default_value = "localhost")]
    db_host: String,
    
    /// Database port
    #[arg(long, env = "DB_PORT", default_value = "5432")]
    db_port: u16,
    
    /// Database name
    #[arg(long, env = "DB_NAME", default_value = "eth_db")]
    db_name: String,
    
    /// Database user
    #[arg(long, env = "DB_USER", default_value = "postgres")]
    db_user: String,
    
    /// Database password
    #[arg(long, env = "DB_PASSWORD", default_value = "postgres")]
    db_password: String,
    
    /// ZeroMQ socket address for pool updates
    #[arg(long, env = "POOL_ZMQ_ADDRESS", default_value = "tcp://localhost:5557")]
    pool_zmq_address: String,
    
    /// Number of blocks to monitor
    #[arg(long, default_value = "20")]
    blocks_to_monitor: u32,
    
    /// ETH threshold for scam detection
    #[arg(long, default_value = "0.4")]
    eth_threshold: f64,
    
    /// Percentage threshold for large withdrawals
    #[arg(long, default_value = "0.5")]
    percentage_threshold: f64,
    
    /// Verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Debug, Default)]
struct MonitoringStats {
    blocks_processed: u32,
    pools_seen: u32,
    pool_updates_received: u32,
    significant_changes: u32,
    scams_detected: u32,
    scams_logged_to_db: u32,
    db_errors: u32,
}

#[derive(Debug, Clone)]
struct PoolSnapshot {
    address: String,
    token_address: String,
    eth_reserve: f64,
    block_number: u64,
    timestamp: f64,
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let args = Args::parse();
    
    // Configure logging
    let log_level = if args.verbose { Level::DEBUG } else { Level::INFO };
    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .init();
    
    info!("🔍 Starting Live Pool Monitoring Test");
    info!("Monitoring {} blocks with ETH threshold: {}", args.blocks_to_monitor, args.eth_threshold);
    info!("ZeroMQ Address: {}", args.pool_zmq_address);
    
    // Initialize database connection
    info!("Connecting to database...");
    let db_logger = match DbLogger::new(
        &args.db_user,
        &args.db_password,
        &args.db_host,
        args.db_port,
        &args.db_name,
    ).await {
        Ok(logger) => Arc::new(logger),
        Err(e) => {
            error!("Failed to connect to database: {}", e);
            return Err(e.into());
        }
    };
    
    // Create pool cache and scam detection service
    let pool_cache = Arc::new(PoolStateCache::new(args.eth_threshold));
    let scam_config = ScamDetectionConfig {
        eth_threshold: args.eth_threshold,
        percentage_threshold: args.percentage_threshold,
    };
    
    let scam_service = ScamDetectionService::new(
        pool_cache.clone(),
        db_logger.clone(),
        scam_config,
    );
    
    // Connect to ZeroMQ
    info!("Connecting to ZeroMQ at: {}", args.pool_zmq_address);
    let context = zmq::Context::new();
    let receiver = context.socket(zmq::SUB)?;
    receiver.connect(&args.pool_zmq_address)?;
    receiver.set_subscribe(b"")?;
    receiver.set_rcvtimeo(5000)?; // 5 second timeout
    
    let mut stats = MonitoringStats::default();
    let mut pool_snapshots: HashMap<String, PoolSnapshot> = HashMap::new();
    let mut current_block = 0u64;
    let start_time = Instant::now();
    
    info!("🚀 Starting live monitoring...");
    
    // Monitor for the specified number of blocks
    while stats.blocks_processed < args.blocks_to_monitor {
        match receiver.recv_string(0) {
            Ok(Ok(message)) => {
                debug!("Received ZeroMQ message: {}", message);
                
                // Parse the pool updates message
                match serde_json::from_str::<PoolUpdatesMessage>(&message) {
                    Ok(pool_message) => {
                        // Get the highest block number from the pool updates
                        let max_block_number = pool_message.data.values()
                            .map(|update| update.block_number)
                            .max()
                            .unwrap_or(0);
                        
                        // Check if this is a new block
                        if max_block_number > current_block {
                            if current_block > 0 {
                                info!("📦 Processed block {} ({} pool updates)", 
                                     current_block, pool_message.data.len());
                                stats.blocks_processed += 1;
                            }
                            current_block = max_block_number;
                        }
                        
                        // Process pool updates
                        for (pool_address, pool_update) in &pool_message.data {
                            stats.pool_updates_received += 1;
                            
                            // Check if this is a new pool
                            if !pool_snapshots.contains_key(pool_address) {
                                stats.pools_seen += 1;
                                info!("🆕 New pool discovered: {} (Token: {}, ETH: {})", 
                                     pool_address, pool_update.token_address, pool_update.eth_reserve);
                            }
                            
                            // Check for significant changes
                            if let Some(previous) = pool_snapshots.get(pool_address) {
                                let eth_delta = pool_update.eth_reserve - previous.eth_reserve;
                                let percentage_change = if previous.eth_reserve > 0.0 {
                                    eth_delta / previous.eth_reserve
                                } else {
                                    0.0
                                };
                                
                                // Log significant changes
                                if eth_delta.abs() > 0.1 || percentage_change.abs() > 0.1 {
                                    stats.significant_changes += 1;
                                    info!("📈 Significant change in pool {}: {} → {} ETH (Δ: {:.4}, {:.1}%)", 
                                         pool_address, previous.eth_reserve, pool_update.eth_reserve, 
                                         eth_delta, percentage_change * 100.0);
                                    
                                    // Test scam detection on significant depletion
                                    if eth_delta < -0.05 && pool_update.eth_reserve < args.eth_threshold {
                                        info!("🚨 Testing scam detection on depleted pool: {}", pool_address);
                                        test_scam_detection_on_pool(
                                            pool_address,
                                            pool_update,
                                            previous,
                                            &scam_service,
                                            &mut stats
                                        ).await;
                                    }
                                }
                            }
                            
                            // Update snapshot
                            pool_snapshots.insert(pool_address.clone(), PoolSnapshot {
                                address: pool_address.clone(),
                                token_address: pool_update.token_address.clone(),
                                eth_reserve: pool_update.eth_reserve,
                                block_number: pool_update.block_number,
                                timestamp: pool_update.update_time,
                            });
                        }
                        
                        // Update pool cache
                        let updated_pools = pool_cache.update_pools(pool_message.data.iter());
                        debug!("Updated {} pools in cache", updated_pools.len());
                        
                    },
                    Err(e) => {
                        error!("Failed to parse ZeroMQ message: {}", e);
                    }
                }
            },
            Ok(Err(zmq_err)) => {
                error!("ZMQ string conversion error: {:?}", zmq_err);
                time::sleep(Duration::from_millis(100)).await;
            },
            Err(zmq::Error::EAGAIN) => {
                // Timeout - continue monitoring
                debug!("ZeroMQ receive timeout, continuing...");
                time::sleep(Duration::from_millis(100)).await;
            },
            Err(e) => {
                error!("ZeroMQ receive error: {}", e);
                time::sleep(Duration::from_secs(1)).await;
            }
        }
        
        // Print periodic stats
        if stats.blocks_processed > 0 && stats.blocks_processed % 5 == 0 {
            print_monitoring_stats(&stats, start_time.elapsed());
        }
    }
    
    // Final summary
    info!("🏁 Monitoring completed!");
    print_final_summary(&stats, &pool_snapshots, start_time.elapsed());
    
    // Test scam detection on current low-reserve pools
    info!("🔍 Testing scam detection on current low-reserve pools...");
    test_scam_detection_on_low_reserve_pools(&pool_snapshots, &scam_service, &mut stats).await;
    
    print_final_summary(&stats, &pool_snapshots, start_time.elapsed());
    
    Ok(())
}

async fn test_scam_detection_on_pool(
    pool_address: &str,
    current_update: &PoolUpdate,
    previous_snapshot: &PoolSnapshot,
    scam_service: &ScamDetectionService,
    stats: &mut MonitoringStats,
) {
    // Create a mock simulation result for this pool change
    let tx_hash = H256::random();
    let mut affected_pools = HashMap::new();
    
    let eth_delta = current_update.eth_reserve - previous_snapshot.eth_reserve;
    let percentage_change = eth_delta / previous_snapshot.eth_reserve;
    
    affected_pools.insert(
        pool_address.to_string(),
        PoolEffect {
            pool_address: pool_address.to_string(),
            current_eth_reserve: previous_snapshot.eth_reserve,
            simulated_eth_reserve: current_update.eth_reserve,
            eth_delta,
            percentage_change,
        }
    );
    
    let simulation = SimulationResult {
        tx_hash,
        from: "0xlive_monitoring_test".to_string(),
        affected_pools,
    };
    
    // Process with scam detection service
    match scam_service.process_transaction(simulation).await {
        Ok(alerts) => {
            if !alerts.is_empty() {
                stats.scams_detected += alerts.len() as u32;
                stats.scams_logged_to_db += alerts.len() as u32;
                info!("✅ Scam detection successful: {} alerts generated and logged", alerts.len());
            } else {
                debug!("No scam alerts generated for this pool change");
            }
        },
        Err(e) => {
            stats.db_errors += 1;
            error!("Scam detection failed: {}", e);
        }
    }
}

async fn test_scam_detection_on_low_reserve_pools(
    pool_snapshots: &HashMap<String, PoolSnapshot>,
    scam_service: &ScamDetectionService,
    stats: &mut MonitoringStats,
) {
    let low_reserve_pools: Vec<_> = pool_snapshots
        .values()
        .filter(|snapshot| snapshot.eth_reserve < 0.5)
        .collect();
    
    info!("Found {} pools with low ETH reserves (< 0.5 ETH)", low_reserve_pools.len());
    
    for pool in low_reserve_pools.iter().take(5) { // Test up to 5 pools
        info!("Testing scam detection on low-reserve pool: {} (ETH: {})", 
             pool.address, pool.eth_reserve);
        
        // Create a mock transaction that would deplete this pool further
        let tx_hash = H256::random();
        let mut affected_pools = HashMap::new();
        
        let simulated_depletion = pool.eth_reserve * 0.1; // Simulate 90% depletion
        let eth_delta = simulated_depletion - pool.eth_reserve;
        
        affected_pools.insert(
            pool.address.clone(),
            PoolEffect {
                pool_address: pool.address.clone(),
                current_eth_reserve: pool.eth_reserve,
                simulated_eth_reserve: simulated_depletion,
                eth_delta,
                percentage_change: eth_delta / pool.eth_reserve,
            }
        );
        
        let simulation = SimulationResult {
            tx_hash,
            from: "0xtest_scammer".to_string(),
            affected_pools,
        };
        
        match scam_service.process_transaction(simulation).await {
            Ok(alerts) => {
                if !alerts.is_empty() {
                    stats.scams_detected += alerts.len() as u32;
                    stats.scams_logged_to_db += alerts.len() as u32;
                    info!("✅ Scam detection test successful on pool {}: {} alerts", 
                         pool.address, alerts.len());
                }
            },
            Err(e) => {
                stats.db_errors += 1;
                error!("Scam detection test failed on pool {}: {}", pool.address, e);
            }
        }
    }
}

fn print_monitoring_stats(stats: &MonitoringStats, elapsed: Duration) {
    info!("📊 Monitoring Stats ({}s elapsed):", elapsed.as_secs());
    info!("  Blocks processed: {}", stats.blocks_processed);
    info!("  Pools discovered: {}", stats.pools_seen);
    info!("  Pool updates: {}", stats.pool_updates_received);
    info!("  Significant changes: {}", stats.significant_changes);
    info!("  Scams detected: {}", stats.scams_detected);
    info!("  Scams logged to DB: {}", stats.scams_logged_to_db);
    if stats.db_errors > 0 {
        warn!("  Database errors: {}", stats.db_errors);
    }
}

fn print_final_summary(stats: &MonitoringStats, pool_snapshots: &HashMap<String, PoolSnapshot>, elapsed: Duration) {
    info!("🎯 FINAL MONITORING SUMMARY");
    info!("═══════════════════════════");
    info!("Duration: {:.1} minutes", elapsed.as_secs_f64() / 60.0);
    info!("Blocks processed: {}", stats.blocks_processed);
    info!("Unique pools discovered: {}", stats.pools_seen);
    info!("Total pool updates received: {}", stats.pool_updates_received);
    info!("Significant pool changes: {}", stats.significant_changes);
    info!("Scams detected: {}", stats.scams_detected);
    info!("Scams logged to database: {}", stats.scams_logged_to_db);
    
    if stats.db_errors > 0 {
        warn!("Database errors encountered: {}", stats.db_errors);
    }
    
    // Pool statistics
    let low_reserve_count = pool_snapshots.values()
        .filter(|p| p.eth_reserve < 0.5)
        .count();
    let very_low_reserve_count = pool_snapshots.values()
        .filter(|p| p.eth_reserve < 0.1)
        .count();
    
    info!("Pool Reserve Statistics:");
    info!("  Pools with < 0.5 ETH: {}", low_reserve_count);
    info!("  Pools with < 0.1 ETH: {}", very_low_reserve_count);
    
    // Show some example pools
    let mut sorted_pools: Vec<_> = pool_snapshots.values().collect();
    sorted_pools.sort_by(|a, b| a.eth_reserve.partial_cmp(&b.eth_reserve).unwrap());
    
    info!("Lowest Reserve Pools:");
    for pool in sorted_pools.iter().take(5) {
        info!("  {} (Token: {}): {} ETH", 
             pool.address, pool.token_address, pool.eth_reserve);
    }
} 