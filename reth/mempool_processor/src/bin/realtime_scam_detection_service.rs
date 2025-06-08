/*
 * Real-time Scam Detection Service with True Mempool Arrival Tracking
 * 
 * This service uses WebSocket subscriptions to track actual mempool arrival times
 * and measure true end-to-end latency from transaction broadcast to detection.
 */

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use std::collections::HashMap;
use clap::Parser;
use eyre::Result;
use tracing::{info, warn, error, debug, Level};
use tokio::time;

// Mempool processor imports
use mempool_processor::mempool_processor::realtime_fetcher::{RealtimeMempoolFetcher, TimestampedTransaction};
use mempool_processor::mempool_processor::TransactionSource;
use mempool_processor::tx_simulator::TransactionSimulator;
use mempool_processor::pool_subscriber::PoolSubscriber;
use mempool_processor::scam_detection::{ScamDetectionService, ScamDetectionConfig};
use mempool_processor::common::address::to_checksum_address;

// REVM imports
use revm_context::BlockEnv as RevmBlockEnv;
use revm_primitives::hardfork::SpecId;
use ethers::providers::{Provider, Http, Middleware};
use ethers::types::{BlockId, BlockNumber, Address};
use mempool_processor::mempool_processor::db_logger::DbLogger;
use mempool_processor::scam_detection::{SimulationResult, PoolEffect};
use revm_tx_simulator_lib::state_diff_utils::CalculatedAccountChanges;

// For hex encoding
use hex;

#[derive(Parser, Debug)]
struct Args {
    /// JSON-RPC URL for Ethereum node
    #[arg(long, env = "ETH_RPC_URL", default_value = "http://localhost:8545")]
    eth_rpc_url: String,
    
    /// WebSocket URL for real-time mempool subscription
    #[arg(long, env = "ETH_WS_URL", default_value = "ws://localhost:8546")]
    eth_ws_url: String,
    
    /// ZeroMQ socket address for pool updates
    #[arg(long, env = "POOL_ZMQ_ADDRESS", default_value = "tcp://localhost:5557")]
    pool_zmq_address: String,
    
    /// Database connection parameters
    #[arg(long, env = "DB_HOST", default_value = "localhost")]
    db_host: String,
    
    #[arg(long, env = "DB_PORT", default_value = "5432")]
    db_port: u16,
    
    #[arg(long, env = "DB_NAME", default_value = "eth_db")]
    db_name: String,
    
    #[arg(long, env = "DB_USER", default_value = "postgres")]
    db_user: String,
    
    #[arg(long, env = "DB_PASSWORD", default_value = "postgres")]
    db_password: String,
    
    /// ETH threshold for scam detection
    #[arg(long, default_value = "0.01")]
    eth_threshold: f64,
    
    /// Percentage threshold for scam detection
    #[arg(long, default_value = "0.95")]
    percentage_threshold: f64,
    
    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
    
    /// Log file path
    #[arg(long, default_value = "/home/nima/code/crypto/logs/mempool/realtime_scam_detection.log")]
    log_file: String,
    
    /// Process all transactions (not just pool transactions)
    #[arg(long)]
    process_all: bool,
}

/// Performance metrics with real arrival tracking
struct RealTimeMetrics {
    total_processed: AtomicU64,
    total_discovery_latency_ms: AtomicU64,
    total_processing_latency_ms: AtomicU64,
    max_discovery_latency_ms: AtomicU64,
    max_processing_latency_ms: AtomicU64,
}

impl RealTimeMetrics {
    fn new() -> Self {
        Self {
            total_processed: AtomicU64::new(0),
            total_discovery_latency_ms: AtomicU64::new(0),
            total_processing_latency_ms: AtomicU64::new(0),
            max_discovery_latency_ms: AtomicU64::new(0),
            max_processing_latency_ms: AtomicU64::new(0),
        }
    }
    
    fn record_transaction(&self, discovery_latency_ms: u64, processing_latency_ms: u64) {
        self.total_processed.fetch_add(1, Ordering::Relaxed);
        self.total_discovery_latency_ms.fetch_add(discovery_latency_ms, Ordering::Relaxed);
        self.total_processing_latency_ms.fetch_add(processing_latency_ms, Ordering::Relaxed);
        
        // Update max values
        self.max_discovery_latency_ms.fetch_max(discovery_latency_ms, Ordering::Relaxed);
        self.max_processing_latency_ms.fetch_max(processing_latency_ms, Ordering::Relaxed);
    }
    
    fn report(&self) {
        let total = self.total_processed.load(Ordering::Relaxed);
        if total == 0 {
            return;
        }
        
        let avg_discovery = self.total_discovery_latency_ms.load(Ordering::Relaxed) as f64 / total as f64;
        let avg_processing = self.total_processing_latency_ms.load(Ordering::Relaxed) as f64 / total as f64;
        let max_discovery = self.max_discovery_latency_ms.load(Ordering::Relaxed);
        let max_processing = self.max_processing_latency_ms.load(Ordering::Relaxed);
        
        info!("📊 REAL-TIME PERFORMANCE METRICS");
        info!("  Total transactions: {}", total);
        info!("  Discovery latency (WebSocket notification → fetch complete):");
        info!("    Average: {:.1}ms", avg_discovery);
        info!("    Maximum: {}ms", max_discovery);
        info!("  Processing latency (fetch complete → state extraction):");
        info!("    Average: {:.1}ms", avg_processing);
        info!("    Maximum: {}ms", max_processing);
        info!("  TRUE END-TO-END (upper bound on mempool → detection):");
        info!("    Average: {:.1}ms", avg_discovery + avg_processing);
        info!("    Maximum: {}ms", max_discovery + max_processing);
    }
}

fn current_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// Helper function to prepare simulation result from REVM changes
fn prepare_simulation_result_from_revm_changes(
    tx: &mempool_processor::mempool_processor::types::TransactionView,
    account_changes: &HashMap<revm_primitives::Address, revm_tx_simulator_lib::state_diff_utils::CalculatedAccountChanges>,
    pool_cache: Arc<mempool_processor::pool_subscriber::cache::PoolStateCache>,
) -> Option<SimulationResult> {
    let mut affected_pools = HashMap::new();
    let tx_hash_hex = hex::encode(&tx.hash);
    
    for (address, changes) in account_changes {
        let addr_str = to_checksum_address(*address);
        
        // Calculate ETH delta
        let eth_delta_wei = changes.eth_net_change.absolute_value.to_string()
            .parse::<f64>()
            .unwrap_or(0.0);
        let eth_delta = if changes.eth_net_change.is_negative {
            -(eth_delta_wei / 1e18)
        } else {
            eth_delta_wei / 1e18
        };
        
        // Skip small changes
        if eth_delta.abs() < 0.001 {
            continue;
        }
        
        // Check if this is a pool
        if let Some(pool_state) = pool_cache.get_pool(&addr_str) {
            let current_eth = pool_state.eth_reserve;
            let simulated_eth = current_eth + eth_delta;
            let percentage_change = if current_eth > 0.0 {
                eth_delta / current_eth
            } else {
                0.0
            };
            
            let effect = PoolEffect {
                pool_address: addr_str.clone(),
                current_eth_reserve: current_eth,
                simulated_eth_reserve: simulated_eth,
                eth_delta,
                percentage_change,
            };
            
            affected_pools.insert(addr_str, effect);
        }
    }
    
    if affected_pools.is_empty() {
        None
    } else {
        Some(SimulationResult {
            tx_hash: tx_hash_hex,
            affected_pools,
            simulation_successful: true,
            error_message: None,
        })
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    // Initialize logging
    let log_level = if args.verbose { Level::DEBUG } else { Level::INFO };
    
    // Create log directory if needed
    if let Some(log_dir) = std::path::Path::new(&args.log_file).parent() {
        std::fs::create_dir_all(log_dir)?;
    }
    
    // Configure logging
    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .with_target(false)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();
    
    info!("🚀 Starting Real-time Scam Detection Service");
    info!("   ⚡ Using WebSocket subscriptions for TRUE mempool arrival tracking");
    info!("   🎯 Goal: Measure actual end-to-end latency from broadcast to detection");
    
    // Initialize real-time mempool fetcher
    info!("📡 Initializing real-time mempool fetcher...");
    let mut fetcher = RealtimeMempoolFetcher::new(
        &args.eth_rpc_url,
        &args.eth_ws_url,
        50000, // cache size
    ).await?;
    
    // Start WebSocket subscription
    fetcher.start().await?;
    info!("✅ WebSocket subscription active - receiving real-time notifications");
    
    // Initialize REVM simulator
    info!("🧪 Initializing REVM transaction simulator...");
    let tx_simulator = Arc::new(TransactionSimulator::new(
        &args.eth_rpc_url,
        1, // mainnet
        SpecId::CANCUN
    ).await?);
    
    // Get current block environment
    let provider = Provider::<Http>::try_from(&args.eth_rpc_url)?;
    let latest_block = provider
        .get_block(BlockId::Number(BlockNumber::Latest))
        .await?
        .ok_or_else(|| eyre::eyre!("Failed to get latest block"))?;
    
    let mut block_env = RevmBlockEnv::default();
    block_env.number = latest_block.number.unwrap_or_default().as_u64().into();
    block_env.timestamp = latest_block.timestamp.as_u64().into();
    block_env.basefee = latest_block.base_fee_per_gas.unwrap_or_default().as_u64();
    
    info!("📦 Current block: #{}", block_env.number);
    
    // Initialize pool cache
    info!("🏊 Initializing pool cache...");
    let pool_subscriber = PoolSubscriber::new(&args.pool_zmq_address)?;
    let pool_cache = pool_subscriber.get_cache();
    pool_subscriber.start_update_loop();
    
    // Initialize database logger
    info!("💾 Connecting to database...");
    let db_logger = Arc::new(DbLogger::new(
        &args.db_user,
        &args.db_password,
        &args.db_host,
        args.db_port,
        &args.db_name
    ).await?);
    
    // Initialize scam detection service
    let scam_config = ScamDetectionConfig {
        eth_threshold: args.eth_threshold,
        percentage_threshold: args.percentage_threshold,
    };
    
    let scam_service = ScamDetectionService::new(
        pool_cache.clone(),
        db_logger.clone(),
        scam_config,
    );
    
    info!("🛡️ Scam detection thresholds: ETH < {:.3}, Percentage > {:.1}%", 
          args.eth_threshold, args.percentage_threshold * 100.0);
    
    // Performance metrics
    let metrics = Arc::new(RealTimeMetrics::new());
    let metrics_reporter = metrics.clone();
    
    // Spawn metrics reporter
    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(30));
        loop {
            interval.tick().await;
            metrics_reporter.report();
        }
    });
    
    // Main processing loop
    info!("🔄 Starting main processing loop...");
    let mut total_scams = 0;
    let mut last_block_update = Instant::now();
    
    loop {
        // Update block environment periodically
        if last_block_update.elapsed() > Duration::from_secs(12) {
            if let Ok(Some(new_block)) = provider.get_block(BlockId::Number(BlockNumber::Latest)).await {
                block_env.number = new_block.number.unwrap_or_default().as_u64().into();
                block_env.timestamp = new_block.timestamp.as_u64().into();
                block_env.basefee = new_block.base_fee_per_gas.unwrap_or_default().as_u64();
                last_block_update = Instant::now();
                debug!("Updated block environment to #{}", block_env.number);
            }
        }
        
        // Get new transactions with timestamps
        let timestamped_txs = fetcher.get_timestamped_transactions().await;
        
        if timestamped_txs.is_empty() {
            // Small sleep to avoid busy waiting
            time::sleep(Duration::from_millis(10)).await;
            continue;
        }
        
        info!("⚡ Processing {} new transactions from WebSocket", timestamped_txs.len());
        
        for timestamped_tx in timestamped_txs {
            let processing_start = current_timestamp_ms();
            let tx_hash = hex::encode(&timestamped_tx.transaction.hash);
            
            // Calculate discovery latency
            let discovery_latency_ms = processing_start.saturating_sub(timestamped_tx.discovery_timestamp_ms);
            
            // Check if it's a pool transaction
            let to_address = if let Some(to_bytes) = &timestamped_tx.transaction.to {
                if to_bytes.len() >= 20 {
                    let mut addr_array = [0u8; 20];
                    addr_array.copy_from_slice(&to_bytes[to_bytes.len()-20..]);
                    let addr = Address::from(addr_array);
                    Some(to_checksum_address(addr))
                } else {
                    None
                }
            } else {
                None
            };
            
            let is_pool_tx = to_address.as_ref()
                .map(|addr| pool_cache.get_pool(addr).is_some())
                .unwrap_or(false);
            
            // Skip non-pool transactions unless --process-all
            if !is_pool_tx && !args.process_all {
                continue;
            }
            
            // Process with REVM
            match tx_simulator.process_transaction(&timestamped_tx.transaction, &block_env).await {
                Ok(Some(account_changes)) => {
                    let processing_end = current_timestamp_ms();
                    let processing_latency_ms = processing_end - processing_start;
                    
                    // Record metrics
                    metrics.record_transaction(discovery_latency_ms, processing_latency_ms);
                    
                    if args.verbose {
                        info!("✅ Processed {} in {}ms (discovery: {}ms, processing: {}ms)",
                              &tx_hash[..8], 
                              discovery_latency_ms + processing_latency_ms,
                              discovery_latency_ms,
                              processing_latency_ms);
                        info!("   Estimated mempool arrival: {} (upper bound)",
                              timestamped_tx.estimated_mempool_arrival_ms);
                        info!("   WebSocket notification: {}",
                              timestamped_tx.discovery_timestamp_ms);
                        info!("   Processing complete: {}", processing_end);
                        info!("   Accounts affected: {}", account_changes.len());
                    }
                    
                    // Check for scams
                    let simulation = prepare_simulation_result_from_revm_changes(
                        &timestamped_tx.transaction,
                        &account_changes,
                        pool_cache.clone()
                    );
                    
                    if let Some(sim_result) = simulation {
                        match scam_service.process_transaction(sim_result).await {
                            Ok(alerts) => {
                                if !alerts.is_empty() {
                                    total_scams += alerts.len();
                                    error!("🚨 SCAM DETECTED in {} ({}ms from mempool arrival)",
                                           tx_hash,
                                           discovery_latency_ms + processing_latency_ms);
                                    for alert in alerts {
                                        error!("   {}", alert.message);
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Scam detection error: {}", e);
                            }
                        }
                    }
                }
                Ok(None) => {
                    debug!("No state changes for {}", &tx_hash[..8]);
                }
                Err(e) => {
                    warn!("Failed to simulate {}: {}", &tx_hash[..8], e);
                }
            }
        }
    }
}