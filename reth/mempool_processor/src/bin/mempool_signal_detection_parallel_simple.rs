/*
 * Simplified Parallel Mempool Signal Detection
 * 
 * Uses thread-based parallelism to avoid Send/Sync issues with ZMQ
 */

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use clap::Parser;
use eyre::Result;
use tracing::{info, warn, error};
use tokio::sync::mpsc;

use mempool_processor::mempool_fetcher::ipc_ipc_variants::{FullTxIpcClient, FullIpcTransaction};
use mempool_processor::mempool_fetcher::TransactionView;
use mempool_processor::pool_subscriber::PoolSubscriber;
use mempool_processor::signal_engine::{ScamDetectionService, ScamDetectionConfig};
use mempool_processor::mempool_fetcher::processor::DbLogger;
use mempool_processor::tx_simulator::DebugTraceCallSimulator;

use ethers::providers::{Provider, Http, Middleware};
use ethers::types::{BlockId, BlockNumber};

#[derive(Parser, Debug)]
struct Args {
    #[arg(long, env = "ETH_RPC_URL", default_value = "http://localhost:8545")]
    eth_rpc_url: String,
    
    #[arg(long, env = "IPC_PATH", default_value = "/tmp/reth.ipc")]
    ipc_path: String,
    
    #[arg(long, env = "POOL_ZMQ_ADDRESS", default_value = "tcp://localhost:5557")]
    pool_zmq_address: String,
    
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
    
    #[arg(long, default_value = "0.01")]
    eth_threshold: f64,
    
    #[arg(long, default_value = "0.5")]
    percentage_threshold: f64,
}

/// Shared metrics
struct Metrics {
    total_processed: AtomicU64,
    pools_affected: AtomicU64,
    events_detected: AtomicU64,
    sub_1ms_count: AtomicU64,
}

/// Process a batch of transactions
async fn process_batch(
    transactions: Vec<FullIpcTransaction>,
    pool_cache: Arc<mempool_processor::pool_subscriber::PoolStateCache>,
    db_logger: Arc<DbLogger>,
    scam_config: ScamDetectionConfig,
    metrics: Arc<Metrics>,
    worker_id: usize,
) -> Result<()> {
    // Create per-batch instances
    let tx_simulator = DebugTraceCallSimulator::new("http://localhost:8545").await?;
    let scam_service = ScamDetectionService::new(
        pool_cache.clone(),
        db_logger.clone(),
        scam_config,
    );
    
    for ipc_tx in transactions {
        let detection_latency_ms = ipc_tx.latency_us as f64 / 1000.0;
        if detection_latency_ms < 1.0 {
            metrics.sub_1ms_count.fetch_add(1, Ordering::Relaxed);
        }
        
        let tx = &ipc_tx.transaction;
        let tx_view = TransactionView {
            hash: tx.hash.as_bytes().to_vec(),
            from: tx.from.as_bytes().to_vec(),
            to: tx.to.map(|addr| addr.as_bytes().to_vec()),
            value: tx.value,
            gas_price: tx.gas_price,
            gas_limit: Some(tx.gas),
            nonce: Some(tx.nonce),
            input_data: Some(tx.input.to_vec()),
        };
        
        // Simulate transaction
        match tx_simulator.process_transaction(&tx_view, &Default::default()).await {
            Ok(Some(state_changes)) => {
                let mut affected_count = 0;
                
                for (address_str, changes) in &state_changes {
                    if pool_cache.get_pool(address_str).is_some() {
                        let eth_delta = if changes.eth_net_change.is_negative {
                            -(changes.eth_net_change.absolute_value.to_string()
                                .parse::<u128>()
                                .unwrap_or(0) as f64 / 1e18)
                        } else {
                            changes.eth_net_change.absolute_value.to_string()
                                .parse::<u128>()
                                .unwrap_or(0) as f64 / 1e18
                        };
                        
                        if eth_delta.abs() > 0.001 {
                            affected_count += 1;
                        }
                    }
                }
                
                if affected_count > 0 {
                    metrics.pools_affected.fetch_add(1, Ordering::Relaxed);
                    
                    // Check for scams
                    let simulation_result = mempool_processor::signal_engine::SimulationResult {
                        tx_hash: format!("{:?}", tx.hash),
                        affected_pools: Default::default(), // Simplified
                        simulation_successful: true,
                        error_message: None,
                    };
                    
                    match scam_service.process_transaction(simulation_result).await {
                        Ok(events) => {
                            if !events.is_empty() {
                                metrics.events_detected.fetch_add(events.len() as u64, Ordering::Relaxed);
                                info!("[W{}] Detected {} events in tx {}", 
                                      worker_id, events.len(), &format!("{:?}", tx.hash)[..10]);
                            }
                        }
                        Err(e) => {
                            error!("[W{}] Scam detection error: {}", worker_id, e);
                        }
                    }
                }
            }
            Ok(None) => {}
            Err(e) => {
                warn!("[W{}] Simulation error: {}", worker_id, e);
            }
        }
        
        metrics.total_processed.fetch_add(1, Ordering::Relaxed);
    }
    
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    tracing_subscriber::fmt()
        .with_target(false)
        .init();
    
    info!("🚀 Starting Simplified Parallel Signal Detection");
    
    // Initialize components
    let http_provider = Arc::new(Provider::<Http>::try_from(&args.eth_rpc_url)?);
    let latest_block = http_provider
        .get_block(BlockId::Number(BlockNumber::Latest))
        .await?
        .ok_or_else(|| eyre::eyre!("Failed to get latest block"))?;
    
    info!("📦 Current block: #{}", latest_block.number.unwrap_or_default());
    
    // Initialize IPC client
    let ipc_client = Arc::new(FullTxIpcClient::new(Some(&args.ipc_path))?);
    ipc_client.start_monitoring().await?;
    info!("✅ IPC subscription active");
    
    // Initialize pool subscriber
    let mut pool_subscriber = PoolSubscriber::with_endpoint(args.eth_threshold, &args.pool_zmq_address);
    let pool_cache = pool_subscriber.get_pool_cache();
    
    let pool_cache_clone = pool_cache.clone();
    tokio::spawn(async move {
        if let Err(e) = pool_subscriber.start_listening().await {
            error!("Pool subscriber failed: {}", e);
        }
    });
    
    tokio::time::sleep(Duration::from_secs(2)).await;
    info!("📊 Monitoring {} pools", pool_cache_clone.get_pool_count());
    
    // Initialize database
    let db_logger = Arc::new(DbLogger::new(
        &args.db_user,
        &args.db_password,
        &args.db_host,
        args.db_port,
        &args.db_name
    ).await?);
    
    // Scam config
    let scam_config = ScamDetectionConfig {
        thresholds: mempool_processor::signal_engine::SignalThresholds {
            eth_threshold: args.eth_threshold,
            scam_drain_percent: args.percentage_threshold,
            warning_drain_percent: 0.2,
            supply_increase_percent: 0.1,
            volume_spike_multiplier: 5.0,
            price_impact_percent: 0.15,
            small_pool_max_eth: 5.0,
            medium_pool_max_eth: 50.0,
        },
        enable_ml_scoring: false,
        min_confidence: 0.7,
    };
    
    let metrics = Arc::new(Metrics {
        total_processed: AtomicU64::new(0),
        pools_affected: AtomicU64::new(0),
        events_detected: AtomicU64::new(0),
        sub_1ms_count: AtomicU64::new(0),
    });
    
    // Main processing loop
    let mut batch_count = 0;
    let start_time = Instant::now();
    
    loop {
        // Get batch of transactions
        match ipc_client.get_full_transactions(50).await {
            Ok(txs) if !txs.is_empty() => {
                let batch_size = txs.len();
                
                // Process batch in a spawned task
                let pool_cache = pool_cache_clone.clone();
                let db_logger = db_logger.clone();
                let scam_config = scam_config.clone();
                let metrics = metrics.clone();
                let worker_id = batch_count % 2; // Simple round-robin
                
                tokio::spawn(async move {
                    if let Err(e) = process_batch(
                        txs,
                        pool_cache,
                        db_logger,
                        scam_config,
                        metrics,
                        worker_id,
                    ).await {
                        error!("Batch processing error: {}", e);
                    }
                });
                
                batch_count += 1;
                
                // Report stats every 100 batches
                if batch_count % 100 == 0 {
                    let total = metrics.total_processed.load(Ordering::Relaxed);
                    let elapsed = start_time.elapsed().as_secs_f64();
                    let rate = total as f64 / elapsed;
                    let sub_1ms = metrics.sub_1ms_count.load(Ordering::Relaxed);
                    let sub_1ms_pct = (sub_1ms as f64 / total as f64) * 100.0;
                    
                    info!("📊 Stats: {} txs, {:.1} tx/s, {:.1}% <1ms, {} pools affected, {} events",
                          total, rate, sub_1ms_pct,
                          metrics.pools_affected.load(Ordering::Relaxed),
                          metrics.events_detected.load(Ordering::Relaxed));
                }
            }
            Ok(_) => {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            Err(e) => {
                warn!("Failed to get transactions: {}", e);
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
    }
}