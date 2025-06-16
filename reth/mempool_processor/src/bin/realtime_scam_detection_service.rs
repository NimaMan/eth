/*
 * Real-time Scam Detection Service
 * 
 * This service monitors the Ethereum mempool for potential scam transactions,
 * particularly focusing on liquidity pool drains and rug pulls.
 */

use std::sync::Arc;
use std::time::{Duration, Instant};
use std::collections::HashMap;
use clap::Parser;
use eyre::Result;
use tracing::{info, warn, error, debug, Level};
use tokio::time;
use chrono::Local;

// Mempool processor imports
use mempool_processor::mempool_fetcher::{WebSocketClient, TransactionView};
use mempool_processor::pool_subscriber::PoolSubscriber;
use mempool_processor::signal_engine::{ScamDetectionService, ScamDetectionConfig};
use mempool_processor::mempool_fetcher::processor::DbLogger;
use mempool_processor::tx_simulator::DebugTraceCallSimulator;
use mempool_processor::common::address::to_checksum_address;

// Ethers imports
use ethers::providers::{Provider, Http, Middleware};
use ethers::types::{BlockId, BlockNumber, Address, H256, U256};

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
}

/// Convert ethers Transaction to TransactionView
fn convert_ethers_to_transaction_view(tx: &ethers::types::Transaction) -> TransactionView {
    TransactionView {
        hash: tx.hash.as_bytes().to_vec(),
        from: tx.from.as_bytes().to_vec(),
        to: tx.to.map(|addr| addr.as_bytes().to_vec()),
        value: tx.value,
        gas_price: tx.gas_price,
        gas_limit: Some(tx.gas),
        nonce: Some(tx.nonce),
        input_data: Some(tx.input.to_vec()),
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
    info!("   🎯 ETH threshold: {} ETH", args.eth_threshold);
    info!("   📊 Percentage threshold: {}%", args.percentage_threshold * 100.0);
    
    // Initialize HTTP provider
    info!("📡 Connecting to Ethereum node...");
    let http_provider = Arc::new(Provider::<Http>::try_from(&args.eth_rpc_url)?);
    
    // Get current block
    let latest_block = http_provider
        .get_block(BlockId::Number(BlockNumber::Latest))
        .await?
        .ok_or_else(|| eyre::eyre!("Failed to get latest block"))?;
    
    info!("📦 Current block: #{}", latest_block.number.unwrap_or_default());
    
    // Initialize WebSocket client
    info!("🔌 Initializing WebSocket client...");
    let mut ws_client = WebSocketClient::new(&args.eth_ws_url, &args.eth_rpc_url)?;
    ws_client.start_monitoring().await?;
    info!("✅ WebSocket subscription active");
    
    // Initialize pool subscriber
    info!("🏊 Initializing pool subscriber...");
    let mut pool_subscriber = PoolSubscriber::with_endpoint(args.eth_threshold, &args.pool_zmq_address);
    let pool_cache = pool_subscriber.get_pool_cache();
    
    // Start pool subscriber in background
    let pool_cache_clone = pool_cache.clone();
    tokio::spawn(async move {
        info!("Starting pool subscriber listener...");
        if let Err(e) = pool_subscriber.start_listening().await {
            error!("Pool subscriber failed: {}", e);
        }
    });
    
    // Give pool subscriber time to initialize
    tokio::time::sleep(Duration::from_secs(2)).await;
    let pool_count = pool_cache_clone.get_pool_count();
    info!("📊 Monitoring {} pools", pool_count);
    
    // Initialize database logger
    info!("💾 Connecting to database...");
    let db_logger = Arc::new(DbLogger::new(
        &args.db_user,
        &args.db_password,
        &args.db_host,
        args.db_port,
        &args.db_name
    ).await?);
    
    // Initialize decision engine service
    let scam_config = ScamDetectionConfig {
        thresholds: mempool_processor::signal_engine::SignalThresholds {
            eth_threshold: args.eth_threshold,
            scam_drain_percent: args.percentage_threshold,
            warning_drain_percent: 0.2,         // 20%
            supply_increase_percent: 0.1,       // 10%
            volume_spike_multiplier: 5.0,       // 5x average
            price_impact_percent: 0.15,         // 15%
            small_pool_max_eth: 5.0,
            medium_pool_max_eth: 50.0,
        },
        enable_ml_scoring: false,
        min_confidence: 0.7,
    };
    
    let scam_service = ScamDetectionService::new(
        pool_cache_clone.clone(),
        db_logger.clone(),
        scam_config,
    );
    
    info!("🛡️ Scam detection service initialized");
    
    // Initialize transaction simulator
    let tx_simulator = DebugTraceCallSimulator::new(&args.eth_rpc_url).await?;
    
    // Performance metrics
    let mut total_processed = 0u64;
    let mut total_scams = 0u64;
    let mut last_report = Instant::now();
    
    // Main processing loop
    info!("🔄 Starting main processing loop...");
    
    loop {
        // Get new transactions from WebSocket
        let new_txs = match ws_client.get_transactions(100).await {
            Ok(txs) => txs,
            Err(e) => {
                warn!("Failed to get transactions: {}", e);
                time::sleep(Duration::from_millis(100)).await;
                continue;
            }
        };
        
        if new_txs.is_empty() {
            // Small sleep to avoid busy waiting
            time::sleep(Duration::from_millis(10)).await;
            continue;
        }
        
        for ws_tx in new_txs {
            let start_time = Instant::now();
            
            // Parse transaction hash
            let tx_hash = match ws_tx.hash.parse::<H256>() {
                Ok(hash) => hash,
                Err(e) => {
                    warn!("Invalid transaction hash: {}", e);
                    continue;
                }
            };
            
            // Fetch full transaction
            match http_provider.get_transaction(tx_hash).await {
                Ok(Some(tx)) => {
                    // Check if transaction involves any pools
                    let to_address = if let Some(to) = tx.to {
                        Some(format!("{:?}", to))
                    } else {
                        None
                    };
                    
                    let from_address = format!("{:?}", tx.from);
                    
                    // Quick check if this might involve a pool
                    let might_involve_pool = if let Some(ref to_addr) = to_address {
                        pool_cache_clone.get_pool(to_addr).is_some() ||
                        is_known_router(&to_addr)
                    } else {
                        false
                    };
                    
                    if !might_involve_pool {
                        continue;
                    }
                    
                    // Convert ethers transaction to TransactionView for the simulator
                    let tx_view = convert_ethers_to_transaction_view(&tx);
                    
                    // Use debug_traceCall to get state changes
                    match tx_simulator.process_transaction(&tx_view, &Default::default()).await {
                        Ok(Some(state_changes)) => {
                            let mut affected_pools = HashMap::new();
                            
                            // Process state changes for each address
                            for (address, changes) in &state_changes {
                                let address_str = format!("0x{:040x}", address);
                                
                                // Check if this address is a pool
                                if pool_cache_clone.get_pool(&address_str).is_some() {
                                    // Calculate ETH impact
                                    if changes.eth_net_change.absolute_value > revm_primitives::U256::ZERO {
                                        let eth_amount = changes.eth_net_change.absolute_value
                                            .to_string()
                                            .parse::<u128>()
                                            .unwrap_or(0) as f64 / 1e18;
                                        let is_outgoing = changes.eth_net_change.is_negative;
                                        
                                        if is_outgoing {
                                            // ETH leaving the pool
                                            check_pool_impact(
                                                &address_str,
                                                &"external",
                                                eth_amount,
                                                true, // is_eth
                                                &pool_cache_clone,
                                                &mut affected_pools
                                            );
                                        }
                                    }
                                    
                                    // Calculate token impacts
                                    for (token_addr, token_change) in &changes.token_net_changes {
                                        if token_change.absolute_value > revm_primitives::U256::ZERO {
                                            let token_amount = token_change.absolute_value
                                                .to_string()
                                                .parse::<u128>()
                                                .unwrap_or(0) as f64;
                                            let is_outgoing = token_change.is_negative;
                                            
                                            if is_outgoing {
                                                // Tokens leaving the pool
                                                check_pool_impact(
                                                    &address_str,
                                                    &"external",
                                                    token_amount,
                                                    false, // is_eth
                                                    &pool_cache_clone,
                                                    &mut affected_pools
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                            
                            // If pools are affected, check for scams
                            if !affected_pools.is_empty() {
                                let simulation_result = mempool_processor::signal_engine::SimulationResult {
                                    tx_hash: format!("{:?}", tx_hash),
                                    affected_pools,
                                    simulation_successful: true,
                                    error_message: None,
                                };
                                
                                match scam_service.process_transaction(simulation_result).await {
                                    Ok(alerts) => {
                                        if !alerts.is_empty() {
                                            total_scams += alerts.len() as u64;
                                            for alert in alerts {
                                                error!("🚨 {:?} DETECTED: {}", alert.event_type, alert.tx_hash);
                                                error!("   Pool: {}", alert.pool_address);
                                                error!("   Token: {}", alert.token_address);
                                                error!("   Severity: {:?}", alert.severity);
                                                error!("   Confidence: {:.2}", alert.confidence);
                                                error!("   ETH Impact: {:.4} ETH ({:.1}%)", alert.metrics.eth_change, alert.metrics.eth_percent * 100.0);
                                                error!("   Details: {}", alert.details);
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        error!("Scam detection error: {}", e);
                                    }
                                }
                            }
                            
                            total_processed += 1;
                            let elapsed = start_time.elapsed();
                            
                            debug!("Processed {} in {:?}", tx_hash, elapsed);
                        }
                        Ok(None) => {
                            debug!("No state changes detected for transaction {}", tx_hash);
                        }
                        Err(e) => {
                            debug!("Failed to simulate transaction {}: {}", tx_hash, e);
                        }
                    }
                }
                Ok(None) => {
                    debug!("Transaction {} not found", tx_hash);
                }
                Err(e) => {
                    warn!("Failed to fetch transaction {}: {}", tx_hash, e);
                }
            }
        }
        
        // Report statistics periodically
        if last_report.elapsed() > Duration::from_secs(30) {
            info!("📊 Statistics:");
            info!("   Transactions processed: {}", total_processed);
            info!("   Scams detected: {}", total_scams);
            info!("   Pools monitored: {}", pool_cache_clone.get_pool_count());
            last_report = Instant::now();
        }
    }
}

/// Check if an address is a known DEX router
fn is_known_router(address: &str) -> bool {
    matches!(address,
        "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D" | // Uniswap V2 Router
        "0xE592427A0AEce92De3Edee1F18E0157C05861564" | // Uniswap V3 Router
        "0x68b3465833fb72A70ecDF485E0e4C7bD8665Fc45"   // Uniswap Universal Router
    )
}

/// Check if a transfer affects a pool and calculate the impact
fn check_pool_impact(
    from: &str,
    to: &str,
    amount: f64,
    is_eth: bool,
    pool_cache: &Arc<mempool_processor::pool_subscriber::cache::PoolStateCache>,
    affected_pools: &mut HashMap<String, mempool_processor::signal_engine::PoolEffect>,
) {
    // Check if sender is a pool
    if let Some(pool_state) = pool_cache.get_pool(from) {
        let current_reserve = if is_eth { pool_state.eth_reserve } else { pool_state.token_reserve };
        let simulated_reserve = current_reserve - amount;
        let percentage_change = -amount / current_reserve;
        
        affected_pools.insert(
            from.to_string(),
            mempool_processor::signal_engine::PoolEffect {
                pool_address: from.to_string(),
                current_eth_reserve: if is_eth { current_reserve } else { pool_state.eth_reserve },
                simulated_eth_reserve: if is_eth { simulated_reserve } else { pool_state.eth_reserve },
                current_token_reserve: if !is_eth { current_reserve } else { pool_state.token_reserve },
                simulated_token_reserve: if !is_eth { simulated_reserve } else { pool_state.token_reserve },
                eth_delta: if is_eth { -amount } else { 0.0 },
                token_delta: if !is_eth { -amount } else { 0.0 },
                percentage_change,
            }
        );
    }
    
    // Check if receiver is a pool
    if let Some(pool_state) = pool_cache.get_pool(to) {
        let current_reserve = if is_eth { pool_state.eth_reserve } else { pool_state.token_reserve };
        let simulated_reserve = current_reserve + amount;
        let percentage_change = amount / current_reserve;
        
        affected_pools.insert(
            to.to_string(),
            mempool_processor::signal_engine::PoolEffect {
                pool_address: to.to_string(),
                current_eth_reserve: if is_eth { current_reserve } else { pool_state.eth_reserve },
                simulated_eth_reserve: if is_eth { simulated_reserve } else { pool_state.eth_reserve },
                current_token_reserve: if !is_eth { current_reserve } else { pool_state.token_reserve },
                simulated_token_reserve: if !is_eth { simulated_reserve } else { pool_state.token_reserve },
                eth_delta: if is_eth { amount } else { 0.0 },
                token_delta: if !is_eth { amount } else { 0.0 },
                percentage_change,
            }
        );
    }
}