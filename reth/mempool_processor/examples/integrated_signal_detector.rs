/// Integrated Signal Detector Example
/// 
/// Demonstrates how to combine function detection (fast filter) with
/// transaction simulation (detailed analysis) for comprehensive signal detection.

use std::time::{Duration, Instant};
use std::sync::Arc;
use clap::Parser;
use eyre::Result;
use tracing::{info, warn};
use tokio::time;

// Mempool processor imports
use mempool_processor::mempool_fetcher::{NonBlockingIpcClient, MempoolTransaction};
use mempool_processor::signal_engine::FunctionDetector;
use mempool_processor::tx_simulator::{
    TxSimulator, BatchProcessor, SignalDetector, SignalDetectionConfig,
    BatchSimulationOptions, RethDirectTxSimulator,
};

#[derive(Parser, Debug)]
struct Args {
    /// IPC socket path
    #[arg(long, env = "IPC_PATH", default_value = "/tmp/reth.ipc")]
    ipc_path: String,
    
    /// Reth database path
    #[arg(long, env = "RETH_DB_PATH", default_value = "/home/nima/.local/share/reth/mainnet")]
    reth_db_path: String,
    
    /// Enable simulation for all transactions (not just filtered)
    #[arg(long)]
    simulate_all: bool,
    
    /// Batch size for simulation
    #[arg(long, default_value = "10")]
    batch_size: usize,
    
    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

/// Tracks interesting transactions for batch simulation
struct SimulationBatch {
    transactions: Vec<MempoolTransaction>,
    reasons: Vec<String>,
}

impl SimulationBatch {
    fn new() -> Self {
        Self {
            transactions: Vec::new(),
            reasons: Vec::new(),
        }
    }
    
    fn add(&mut self, tx: MempoolTransaction, reason: &str) {
        self.transactions.push(tx);
        self.reasons.push(reason.to_string());
    }
    
    fn is_full(&self, max_size: usize) -> bool {
        self.transactions.len() >= max_size
    }
    
    fn take(&mut self) -> (Vec<MempoolTransaction>, Vec<String>) {
        let txs = std::mem::take(&mut self.transactions);
        let reasons = std::mem::take(&mut self.reasons);
        (txs, reasons)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    // Initialize logging
    tracing_subscriber::fmt()
        .with_target(false)
        .init();
    
    info!("🚀 Starting Integrated Signal Detection Service");
    info!("   ⚡ Function detection for fast filtering");
    info!("   🔬 Transaction simulation for detailed analysis");
    
    // Initialize components
    info!("📡 Initializing IPC client...");
    let ipc_client = NonBlockingIpcClient::new(Some(&args.ipc_path))?;
    ipc_client.start().await?;
    
    info!("🔍 Initializing function detector...");
    let function_detector = FunctionDetector::new();
    
    info!("🧪 Initializing transaction simulator...");
    let tx_simulator = TxSimulator::new(&args.reth_db_path)?;
    let reth_simulator = Arc::new(RethDirectTxSimulator::new(&args.reth_db_path)?);
    
    // Create batch processor with custom options
    let batch_options = BatchSimulationOptions {
        max_concurrent: 5,
        timeout_per_tx: Some(Duration::from_millis(100)),
        block_number: None,
    };
    let batch_processor = BatchProcessor::new(reth_simulator.clone())
        .with_options(batch_options);
    
    // Initialize signal detector
    let signal_detector = SignalDetector::new(SignalDetectionConfig::default());
    
    // Performance metrics
    info!("🎯 Starting main processing loop...");
    let mut total_processed = 0u64;
    let mut total_simulated = 0u64;
    let mut last_report = Instant::now();
    let mut simulation_batch = SimulationBatch::new();
    
    loop {
        // Get new transactions
        let new_txs = ipc_client.get_transactions_instant(100).await;
        
        if new_txs.is_empty() {
            time::sleep(Duration::from_millis(10)).await;
            
            // Process any pending batch
            if !simulation_batch.transactions.is_empty() {
                process_simulation_batch(
                    &mut simulation_batch,
                    &batch_processor,
                    &signal_detector,
                    &mut total_simulated,
                ).await?;
            }
            continue;
        }
        
        // Batch function detection
        let detected_functions = function_detector.detect_batch(&new_txs);
        let detected_count = detected_functions.len();
        
        if detected_count > 0 {
            info!("🎯 Detected {} interesting functions in batch of {}", 
                  detected_count, new_txs.len());
        }
        
        total_processed += new_txs.len() as u64;
        
        // Add transactions to simulation batch
        for tx in new_txs {
            if let Some(function_name) = detected_functions.get(&tx.hash) {
                simulation_batch.add(tx, function_name);
            } else if args.simulate_all {
                simulation_batch.add(tx, "all_transactions");
            }
        }
        
        // Process batch if full
        if simulation_batch.is_full(args.batch_size) {
            process_simulation_batch(
                &mut simulation_batch,
                &batch_processor,
                &signal_detector,
                &mut total_simulated,
            ).await?;
        }
        
        // Periodic reporting
        if last_report.elapsed() > Duration::from_secs(60) {
            info!("📊 PERFORMANCE REPORT:");
            info!("   Total processed: {}", total_processed);
            info!("   Total simulated: {}", total_simulated);
            info!("   Simulation rate: {:.1}%", (total_simulated as f64 / total_processed as f64) * 100.0);
            
            function_detector.log_stats_summary();
            signal_detector.log_stats_summary();
            
            last_report = Instant::now();
        }
    }
}

/// Process a batch of transactions for simulation
async fn process_simulation_batch(
    batch: &mut SimulationBatch,
    processor: &BatchProcessor,
    signal_detector: &SignalDetector,
    total_simulated: &mut u64,
) -> Result<()> {
    let (transactions, reasons) = batch.take();
    if transactions.is_empty() {
        return Ok(());
    }
    
    let batch_size = transactions.len();
    info!("🚀 Simulating batch of {} transactions...", batch_size);
    
    let start = Instant::now();
    
    // Simulate with state changes
    match processor.simulate_batch_with_state_changes(transactions).await {
        Ok(results) => {
            let elapsed = start.elapsed();
            info!("✅ Batch simulation complete in {:?}", elapsed);
            
            // Analyze each result
            for (i, (tx_hash, result)) in results.iter().enumerate() {
                match result {
                    Ok(state_changes) => {
                        // Parse state changes
                        if let Some(changes) = state_changes.as_object() {
                            let mut address_changes = std::collections::HashMap::new();
                            
                            // Convert JSON state changes to proper format
                            for (addr_str, change_value) in changes {
                                if let Ok(address) = addr_str.parse::<alloy_primitives::Address>() {
                                    if let Some(change_obj) = change_value.as_object() {
                                        let debug_change = mempool_processor::tx_simulator::DebugAddressStateChange {
                                            balance: change_obj.get("balance")
                                                .and_then(|v| v.as_str())
                                                .map(|s| s.to_string()),
                                            nonce: change_obj.get("nonce")
                                                .and_then(|v| v.as_u64()),
                                            code: change_obj.get("code")
                                                .and_then(|v| v.as_str())
                                                .map(|s| s.to_string()),
                                            storage: std::collections::HashMap::new(), // TODO: parse storage
                                        };
                                        address_changes.insert(address, debug_change);
                                    }
                                }
                            }
                            
                            // Detect signals from state changes
                            let from_address = alloy_primitives::Address::default(); // TODO: get from tx
                            let signals = signal_detector.analyze_simulation_result(
                                tx_hash,
                                from_address,
                                None,
                                &address_changes,
                                elapsed.as_micros() as u64 / batch_size as u64,
                            );
                            
                            if !signals.is_empty() {
                                warn!("🚨 {} signals detected in tx {} (reason: {})", 
                                      signals.len(), tx_hash, reasons[i]);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("❌ Simulation failed for {}: {}", tx_hash, e);
                    }
                }
            }
            
            *total_simulated += batch_size as u64;
        }
        Err(e) => {
            warn!("❌ Batch simulation failed: {}", e);
        }
    }
    
    Ok(())
}