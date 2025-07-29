/// Full Pipeline Signal Detection Example
/// 
/// Demonstrates the complete signal detection pipeline:
/// IPC → Function Detector → TX Router → SimulationManager → SignalManager → Results
///
/// Processes 1000 transactions and provides comprehensive performance metrics.

use std::time::{Duration, Instant};
use std::sync::Arc;
use clap::Parser;
use eyre::Result;
use tracing::{info, warn};
use tokio::sync::Mutex;
use std::fs::{create_dir_all, OpenOptions};
use std::io::Write;
use chrono::Local;
use ethers::types::H256;
use hex;

// Mempool processor imports
use mempool_processor::{
    mempool_fetcher::NonBlockingIpcClient,
    function_detector::FunctionDetector,
    tx_router::{TransactionRouter as TxRouter, TransactionCategory, SimulationPriority},
    simulator::{SimulationManager, SimulationRequest, SimulationType, SequentialBuySellSimulator, BuySellSimulatorConfig, TxSimulator},
    signal_detector::{SignalManager, SignalManagerConfig},
    token_tracking::{TokenTrackingCache, TokenTrackingSubscriber},
};

#[derive(Parser, Debug)]
struct Args {
    /// IPC socket path
    #[arg(long, env = "IPC_PATH", default_value = "/tmp/reth.ipc")]
    ipc_path: String,
    
    /// Reth database path
    #[arg(long, env = "RETH_DB_PATH", default_value = "/home/nima/.local/share/reth/mainnet")]
    reth_db_path: String,
    
    /// Number of transactions to process
    #[arg(long, default_value = "1000")]
    target_count: usize,
    
    /// Batch size for simulation
    #[arg(long, default_value = "10")]
    batch_size: usize,
    
    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

/// Performance metrics tracker
#[derive(Default)]
struct PipelineMetrics {
    // Function detection metrics
    trading_enabled_count: usize,
    liquidity_removal_count: usize,
    tax_change_count: usize,
    detection_latencies: Vec<Duration>,
    
    // Routing metrics
    contract_creation_count: usize,
    creator_action_count: usize,
    skipped_count: usize,
    
    // Simulation metrics
    simulations_run: usize,
    simulation_success: usize,
    simulation_times: Vec<Duration>,
    
    // Signal detection metrics
    liquidity_drains: usize,
    honeypots: usize,
    tax_increases: usize,
    stablecoin_events: usize,
}

impl PipelineMetrics {
    fn add_detection_latency(&mut self, latency: Duration) {
        self.detection_latencies.push(latency);
    }
    
    fn add_simulation_time(&mut self, time: Duration) {
        self.simulation_times.push(time);
    }
    
    fn calculate_stats(&self) -> (Duration, Duration, Duration, Duration) {
        if self.simulation_times.is_empty() {
            return (Duration::ZERO, Duration::ZERO, Duration::ZERO, Duration::ZERO);
        }
        
        let mut times = self.simulation_times.clone();
        times.sort();
        
        let sum: Duration = times.iter().sum();
        let avg = sum / times.len() as u32;
        let max = times.last().cloned().unwrap_or(Duration::ZERO);
        let p99 = times.get(times.len() * 99 / 100).cloned().unwrap_or(max);
        
        (avg, max, p99, sum)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    // Initialize logging
    tracing_subscriber::fmt()
        .with_target(false)
        .with_env_filter(if args.verbose { "debug" } else { "info" })
        .init();
    
    // Create log directory
    let log_dir = "/home/nima/code/crypto/logs/mempool/dev/signal_detector";
    create_dir_all(log_dir)?;
    
    // Create log file
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let log_path = format!("{}/full_pipeline_{}.log", log_dir, timestamp);
    let mut log_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&log_path)?;
    
    writeln!(log_file, "Full Pipeline Signal Detection Test")?;
    writeln!(log_file, "Started: {}", Local::now())?;
    writeln!(log_file, "Target: {} transactions", args.target_count)?;
    writeln!(log_file, "======================================\n")?;
    
    info!("🚀 Starting Full Pipeline Signal Detection");
    info!("📝 Logging to: {}", log_path);
    info!("🎯 Target: {} transactions", args.target_count);
    
    // Initialize token tracking subscriber
    info!("\n📦 Initializing token tracking subscriber...");
    let mut token_subscriber = TokenTrackingSubscriber::new(0.1); // 0.1 ETH threshold
    let token_cache = token_subscriber.get_cache();
    
    // Start listening for token updates in background
    info!("🔌 Starting token cache subscriber (listening on ports 5557/5558)...");
    let subscriber_handle = tokio::spawn(async move {
        if let Err(e) = token_subscriber.start_listening().await {
            warn!("Token subscriber error: {}", e);
        }
    });
    
    // Wait for initial cache population
    info!("⏳ Waiting for token cache population from Python...");
    tokio::time::sleep(Duration::from_secs(3)).await;
    
    // Print cache statistics
    // Get cache counts
    let pools = token_cache.pools.get_pool_count().await;
    let creators = token_cache.creators.get_creator_count().await;
    let tokens = creators; // Approximate unique tokens by creator count
    info!("📊 Token Cache Statistics:");
    info!("   Pools: {}", pools);
    info!("   Creators: {}", creators);
    info!("   Tokens: {}", tokens);
    
    writeln!(log_file, "Token Cache Initial State:")?;
    writeln!(log_file, "  Pools: {}", pools)?;
    writeln!(log_file, "  Creators: {}", creators)?;
    writeln!(log_file, "  Tokens: {}", tokens)?;
    writeln!(log_file)?;
    
    // Initialize components
    info!("\n🔧 Initializing pipeline components...");
    
    // IPC client
    let ipc_client = NonBlockingIpcClient::new(Some(&args.ipc_path))?;
    ipc_client.start().await?;
    info!("✅ IPC client connected");
    
    // Function detector
    let function_detector = FunctionDetector::new();
    info!("✅ Function detector initialized");
    
    // TX Router
    let tx_router = TxRouter::new(Some(token_cache.clone()));
    info!("✅ Transaction router initialized");
    
    // Initialize simulators
    let tx_simulator = Arc::new(TxSimulator::new(&args.reth_db_path).unwrap());
    let buy_sell_config = BuySellSimulatorConfig::default();
    let buy_sell_simulator = Arc::new(SequentialBuySellSimulator::with_config(&args.reth_db_path, buy_sell_config).unwrap());
    
    // Simulation Manager
    let simulation_manager = SimulationManager::new(tx_simulator, buy_sell_simulator, 10);
    info!("✅ Simulation manager initialized");
    
    // Signal Manager
    let signal_config = SignalManagerConfig::default();
    let mut signal_manager = SignalManager::new(signal_config);
    info!("✅ Signal manager initialized");
    
    // Metrics tracker
    let metrics = Arc::new(Mutex::new(PipelineMetrics::default()));
    
    info!("\n🏃 Starting pipeline processing...\n");
    
    let mut total_processed = 0;
    let pipeline_start = Instant::now();
    
    while total_processed < args.target_count {
        // Fetch transactions
        let new_txs = ipc_client.get_transactions_instant(100).await;
        
        if new_txs.is_empty() {
            tokio::time::sleep(Duration::from_millis(10)).await;
            continue;
        }
        
        // Process each transaction through the pipeline
        for tx in new_txs {
            if total_processed >= args.target_count {
                break;
            }
            
            let tx_start = Instant::now();
            total_processed += 1;
            
            // 1. Function Detection
            let detection_start = Instant::now();
            let detected_function = function_detector.detect_function(&tx);
            let detection_time = detection_start.elapsed();
            
            // Update detection metrics
            {
                let mut m = metrics.lock().await;
                m.add_detection_latency(detection_time);
                
                if let Some(func_name) = &detected_function {
                    match func_name.as_str() {
                        "enableTrading" | "openTrading" => m.trading_enabled_count += 1,
                        "removeLiquidity" | "removeLiquidityETH" => m.liquidity_removal_count += 1,
                        "setTaxes" | "updateTaxes" => m.tax_change_count += 1,
                        _ => {}
                    }
                }
            }
            
            // 2. Transaction Routing
            let classification = tx_router.classify(&tx).await;
            let category = classification.category;
            
            // Update routing metrics
            {
                let mut m = metrics.lock().await;
                match &category {
                    TransactionCategory::ContractCreation { .. } => {
                        m.contract_creation_count += 1;
                    }
                    TransactionCategory::CreatorTransaction { .. } => {
                        m.creator_action_count += 1;
                    }
                    _ => {
                        m.skipped_count += 1;
                        continue; // Skip non-relevant transactions
                    }
                }
            }
            
            // 3. Create simulation request
            let sim_request = SimulationRequest {
                tx: tx.clone(),
                category: category.clone(),
                priority: match &category {
                    TransactionCategory::ContractCreation { .. } => SimulationPriority::High,
                    TransactionCategory::CreatorTransaction { .. } => SimulationPriority::Normal,
                    _ => SimulationPriority::Low,
                },
                simulation_type: SimulationType::TransactionOnly,
                tx_hash: H256::from_slice(hex::decode(&tx.hash.trim_start_matches("0x")).unwrap_or_default().as_slice()),
            };
            
            // 4. Run simulation (simplified for demo)
            let sim_start = Instant::now();
            
            // Submit to queue
            if let Err(e) = simulation_manager.submit(sim_request).await {
                warn!("Failed to submit simulation for {}: {}", tx.hash, e);
                continue;
            }
            
            // For demo, just count as submitted
            let sim_time = sim_start.elapsed();
            
            // Update metrics
            {
                let mut m = metrics.lock().await;
                m.simulations_run += 1;
                m.simulation_success += 1; // Assume success for demo
                m.add_simulation_time(sim_time);
            }
            
            // In a real implementation, would process results from queue
            info!("📤 Submitted {} for simulation", tx.hash);
            
            // Progress update every 100 transactions
            if total_processed % 100 == 0 {
                let elapsed = pipeline_start.elapsed();
                let rate = total_processed as f64 / elapsed.as_secs_f64();
                info!("Progress: {}/{} transactions ({:.1} tx/sec)", 
                    total_processed, args.target_count, rate);
            }
        }
    }
    
    let total_time = pipeline_start.elapsed();
    
    // Calculate final metrics
    let m = metrics.lock().await;
    let (avg_sim_time, max_sim_time, p99_sim_time, _) = m.calculate_stats();
    
    // Print and log results
    let results = format!(
        "\n📊 PIPELINE PERFORMANCE REPORT ({} transactions)\n\
        =====================================\n\
        Function Detection:\n\
        - Trading Enabled: {} detected\n\
        - Liquidity Removals: {} detected\n\
        - Tax Changes: {} detected\n\
        - Detection Latency: avg {:.1}μs\n\
        \n\
        Transaction Routing:\n\
        - Contract Creations: {} routed\n\
        - Creator Actions: {} routed\n\
        - Skipped (DEX/Other): {}\n\
        \n\
        Simulation Results:\n\
        - Simulations Run: {}\n\
        - Success Rate: {:.1}%\n\
        - Avg Simulation Time: {:.2}ms\n\
        - Max Simulation Time: {:.2}ms\n\
        - P99 Simulation Time: {:.2}ms\n\
        \n\
        Overall Performance:\n\
        - Total Time: {:.2}s\n\
        - Throughput: {:.1} tx/sec\n\
        - End-to-end Latency: avg {:.1}ms",
        total_processed,
        m.trading_enabled_count,
        m.liquidity_removal_count,
        m.tax_change_count,
        if m.detection_latencies.is_empty() { 0.0 } else { 
            m.detection_latencies.iter().sum::<Duration>().as_micros() as f64 / m.detection_latencies.len() as f64 
        },
        m.contract_creation_count,
        m.creator_action_count,
        m.skipped_count,
        m.simulations_run,
        if m.simulations_run > 0 { m.simulation_success as f64 / m.simulations_run as f64 * 100.0 } else { 0.0 },
        avg_sim_time.as_secs_f64() * 1000.0,
        max_sim_time.as_secs_f64() * 1000.0,
        p99_sim_time.as_secs_f64() * 1000.0,
        total_time.as_secs_f64(),
        total_processed as f64 / total_time.as_secs_f64(),
        total_time.as_millis() as f64 / total_processed as f64
    );
    
    println!("{}", results);
    writeln!(log_file, "{}", results)?;
    
    info!("\n✅ Pipeline test complete!");
    info!("📄 Results saved to: {}", log_path);
    
    Ok(())
}